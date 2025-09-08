//! Storage engine for Precursor broker

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset};

/// Storage engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory
    pub data_dir: String,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u32,
    /// Segment size
    pub segment_size: usize,
    /// Index interval
    pub index_interval: usize,
    /// Flush interval
    pub flush_interval: Duration,
    /// Sync on write
    pub sync_on_write: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            enable_compression: false,
            compression_level: 6,
            segment_size: 1024 * 1024, // 1MB
            index_interval: 4096, // 4KB
            flush_interval: Duration::from_millis(100),
            sync_on_write: false,
        }
    }
}

/// Storage segment
#[derive(Debug, Clone)]
pub struct Segment {
    /// Segment file path
    pub file_path: PathBuf,
    /// Base offset
    pub base_offset: Offset,
    /// Next offset
    pub next_offset: Offset,
    /// Segment size
    pub size: usize,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Last modified timestamp
    pub last_modified: SystemTime,
}

/// Storage index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Offset
    pub offset: Offset,
    /// Position in segment file
    pub position: u64,
    /// Message size
    pub size: u32,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Storage engine
pub struct StorageEngine {
    /// Storage configuration
    config: StorageConfig,
    /// Data directory
    data_dir: PathBuf,
    /// Active segments
    segments: Arc<RwLock<HashMap<String, Vec<Segment>>>>,
    /// Segment indices
    indices: Arc<RwLock<HashMap<String, HashMap<Offset, IndexEntry>>>>,
    /// Flush task handle
    flush_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl StorageEngine {
    /// Create a new storage engine
    pub fn new(data_dir: String) -> Self {
        Self {
            config: StorageConfig::default(),
            data_dir: PathBuf::from(data_dir),
            segments: Arc::new(RwLock::new(HashMap::new())),
            indices: Arc::new(RwLock::new(HashMap::new())),
            flush_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new storage engine with configuration
    pub fn with_config(data_dir: String, config: StorageConfig) -> Self {
        Self {
            config,
            data_dir: PathBuf::from(data_dir),
            segments: Arc::new(RwLock::new(HashMap::new())),
            indices: Arc::new(RwLock::new(HashMap::new())),
            flush_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the storage engine
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing storage engine at: {}", self.data_dir.display());

        // Create data directory if it doesn't exist
        if !self.data_dir.exists() {
            fs::create_dir_all(&self.data_dir)?;
        }

        // Load existing segments
        self.load_segments().await?;

        // Start flush task
        self.start_flush_task().await;

        info!("Storage engine initialized");
        Ok(())
    }

    /// Flush all pending writes
    pub async fn flush(&self) -> Result<()> {
        info!("Flushing storage engine");
        
        // Flush all active segments
        let segments = self.segments.read().await;
        for (topic_name, topic_segments) in segments.iter() {
            for segment in topic_segments {
                self.flush_segment(topic_name, segment).await?;
            }
        }

        info!("Storage engine flushed");
        Ok(())
    }

    /// Write a message to storage
    pub async fn write_message(
        &self,
        topic_name: &str,
        partition_id: i32,
        message: &Message,
        offset: Offset,
    ) -> Result<()> {
        // Get or create segment
        let segment = self.get_or_create_segment(topic_name, partition_id, offset).await?;
        
        // Write message to segment
        let position = self.write_message_to_segment(&segment, message, offset).await?;
        
        // Update index
        self.update_index(topic_name, partition_id, offset, position, message).await?;
        
        debug!(
            "Written message to storage: topic={}, partition={}, offset={}, position={}",
            topic_name, partition_id, offset.value(), position
        );
        
        Ok(())
    }

    /// Read a message from storage
    pub async fn read_message(
        &self,
        topic_name: &str,
        partition_id: i32,
        offset: Offset,
    ) -> Result<Option<Message>> {
        // Get index entry
        let index_entry = self.get_index_entry(topic_name, partition_id, offset).await?;
        let index_entry = match index_entry {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // Find segment
        let segment = self.find_segment(topic_name, partition_id, offset).await?;
        let segment = match segment {
            Some(seg) => seg,
            None => return Ok(None),
        };

        // Read message from segment
        let message = self.read_message_from_segment(&segment, index_entry).await?;
        
        debug!(
            "Read message from storage: topic={}, partition={}, offset={}",
            topic_name, partition_id, offset.value()
        );
        
        Ok(Some(message))
    }

    /// Delete a queue
    pub async fn delete_queue(&self, queue_name: &str) -> Result<()> {
        let queue_dir = self.data_dir.join("queues").join(queue_name);
        if queue_dir.exists() {
            fs::remove_dir_all(&queue_dir)?;
            info!("Deleted queue storage: {}", queue_name);
        }
        Ok(())
    }

    /// Delete a topic partition
    pub async fn delete_partition(&self, topic_name: &str, partition_id: i32) -> Result<()> {
        let partition_dir = self.data_dir.join("topics").join(topic_name).join(format!("partition-{}", partition_id));
        if partition_dir.exists() {
            fs::remove_dir_all(&partition_dir)?;
            info!("Deleted partition storage: {}:{}", topic_name, partition_id);
        }
        Ok(())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        let segments = self.segments.read().await;
        let indices = self.indices.read().await;
        
        let mut total_segments = 0;
        let mut total_size = 0;
        let mut total_messages = 0;
        
        for topic_segments in segments.values() {
            total_segments += topic_segments.len();
            for segment in topic_segments {
                total_size += segment.size;
            }
        }
        
        for topic_indices in indices.values() {
            total_messages += topic_indices.len();
        }
        
        StorageStats {
            total_segments,
            total_size,
            total_messages,
            data_dir: self.data_dir.to_string_lossy().to_string(),
        }
    }

    /// Load existing segments
    async fn load_segments(&self) -> Result<()> {
        let topics_dir = self.data_dir.join("topics");
        if !topics_dir.exists() {
            return Ok(());
        }

        let mut segments = self.segments.write().await;
        
        for topic_entry in fs::read_dir(&topics_dir)? {
            let topic_entry = topic_entry?;
            let topic_name = topic_entry.file_name().to_string_lossy().to_string();
            
            if topic_entry.file_type()?.is_dir() {
                let mut topic_segments = Vec::new();
                
                for partition_entry in fs::read_dir(topic_entry.path())? {
                    let partition_entry = partition_entry?;
                    let partition_name = partition_entry.file_name().to_string_lossy().to_string();
                    
                    if partition_name.starts_with("partition-") && partition_entry.file_type()?.is_dir() {
                        let partition_id: i32 = partition_name.strip_prefix("partition-")
                            .unwrap()
                            .parse()
                            .map_err(|e| Error::Storage(format!("Invalid partition ID: {}", e)))?;
                        
                        // Load segments for this partition
                        let partition_segments = self.load_partition_segments(&topic_name, partition_id).await?;
                        topic_segments.extend(partition_segments);
                    }
                }
                
                if !topic_segments.is_empty() {
                    segments.insert(topic_name, topic_segments);
                }
            }
        }
        
        Ok(())
    }

    /// Load segments for a partition
    async fn load_partition_segments(&self, topic_name: &str, partition_id: i32) -> Result<Vec<Segment>> {
        let partition_dir = self.data_dir.join("topics").join(topic_name).join(format!("partition-{}", partition_id));
        if !partition_dir.exists() {
            return Ok(Vec::new());
        }

        let mut segments = Vec::new();
        
        for entry in fs::read_dir(&partition_dir)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            if file_name.ends_with(".log") {
                let base_offset = file_name.strip_suffix(".log")
                    .unwrap()
                    .parse::<u64>()
                    .map_err(|e| Error::Storage(format!("Invalid segment file name: {}", e)))?;
                
                let metadata = entry.metadata()?;
                let segment = Segment {
                    file_path: entry.path(),
                    base_offset: Offset::new(base_offset),
                    next_offset: Offset::new(base_offset), // Will be updated when loading
                    size: metadata.len() as usize,
                    created_at: metadata.created()?,
                    last_modified: metadata.modified()?,
                };
                
                segments.push(segment);
            }
        }
        
        // Sort segments by base offset
        segments.sort_by_key(|s| s.base_offset);
        
        Ok(segments)
    }

    /// Get or create a segment
    async fn get_or_create_segment(
        &self,
        topic_name: &str,
        partition_id: i32,
        offset: Offset,
    ) -> Result<Segment> {
        let mut segments = self.segments.write().await;
        let topic_segments = segments.entry(topic_name.to_string()).or_insert_with(Vec::new);
        
        // Check if we can use the last segment
        if let Some(last_segment) = topic_segments.last() {
            if last_segment.size < self.config.segment_size {
                return Ok(last_segment.clone());
            }
        }
        
        // Create new segment
        let segment_dir = self.data_dir.join("topics").join(topic_name).join(format!("partition-{}", partition_id));
        fs::create_dir_all(&segment_dir)?;
        
        let segment_file = segment_dir.join(format!("{}.log", offset.value()));
        let segment = Segment {
            file_path: segment_file,
            base_offset: offset,
            next_offset: offset,
            size: 0,
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
        };
        
        topic_segments.push(segment.clone());
        Ok(segment)
    }

    /// Write message to segment
    async fn write_message_to_segment(
        &self,
        segment: &Segment,
        message: &Message,
        offset: Offset,
    ) -> Result<u64> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&segment.file_path)?;
        
        let position = file.seek(io::SeekFrom::End(0))?;
        
        // Serialize message
        let serialized = bincode::serialize(message)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        // Write message size and data
        file.write_all(&(serialized.len() as u32).to_le_bytes())?;
        file.write_all(&serialized)?;
        
        if self.config.sync_on_write {
            file.sync_all()?;
        }
        
        Ok(position)
    }

    /// Read message from segment
    async fn read_message_from_segment(
        &self,
        segment: &Segment,
        index_entry: IndexEntry,
    ) -> Result<Message> {
        let mut file = File::open(&segment.file_path)?;
        file.seek(io::SeekFrom::Start(index_entry.position))?;
        
        // Read message size
        let mut size_bytes = [0u8; 4];
        file.read_exact(&mut size_bytes)?;
        let message_size = u32::from_le_bytes(size_bytes) as usize;
        
        // Read message data
        let mut message_data = vec![0u8; message_size];
        file.read_exact(&mut message_data)?;
        
        // Deserialize message
        let message: Message = bincode::deserialize(&message_data)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        Ok(message)
    }

    /// Update index
    async fn update_index(
        &self,
        topic_name: &str,
        partition_id: i32,
        offset: Offset,
        position: u64,
        message: &Message,
    ) -> Result<()> {
        let mut indices = self.indices.write().await;
        let topic_indices = indices.entry(format!("{}:{}", topic_name, partition_id))
            .or_insert_with(HashMap::new);
        
        let index_entry = IndexEntry {
            offset,
            position,
            size: message.size() as u32,
            timestamp: message.timestamp(),
        };
        
        topic_indices.insert(offset, index_entry);
        Ok(())
    }

    /// Get index entry
    async fn get_index_entry(
        &self,
        topic_name: &str,
        partition_id: i32,
        offset: Offset,
    ) -> Result<Option<IndexEntry>> {
        let indices = self.indices.read().await;
        let topic_indices = indices.get(&format!("{}:{}", topic_name, partition_id));
        
        Ok(topic_indices.and_then(|indices| indices.get(&offset).cloned()))
    }

    /// Find segment for offset
    async fn find_segment(
        &self,
        topic_name: &str,
        partition_id: i32,
        offset: Offset,
    ) -> Result<Option<Segment>> {
        let segments = self.segments.read().await;
        let topic_segments = segments.get(topic_name);
        
        Ok(topic_segments.and_then(|segments| {
            segments.iter()
                .find(|segment| offset >= segment.base_offset)
                .cloned()
        }))
    }

    /// Flush segment
    async fn flush_segment(&self, topic_name: &str, segment: &Segment) -> Result<()> {
        // In a real implementation, this would flush any pending writes
        // For now, we'll just log the flush
        debug!("Flushed segment for topic {}: {}", topic_name, segment.file_path.display());
        Ok(())
    }

    /// Start flush task
    async fn start_flush_task(&self) {
        let config = self.config.clone();
        let segments = self.segments.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.flush_interval);
            
            loop {
                interval.tick().await;
                
                // Flush all segments
                let segments_guard = segments.read().await;
                for (topic_name, topic_segments) in segments_guard.iter() {
                    for segment in topic_segments {
                        // In a real implementation, this would flush pending writes
                        debug!("Periodic flush for topic {}: {}", topic_name, segment.file_path.display());
                    }
                }
            }
        });
        
        let mut flush_task = self.flush_task.write().await;
        *flush_task = Some(handle);
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total number of segments
    pub total_segments: usize,
    /// Total storage size in bytes
    pub total_size: usize,
    /// Total number of messages
    pub total_messages: usize,
    /// Data directory
    pub data_dir: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert_eq!(config.data_dir, "./data");
        assert!(!config.enable_compression);
        assert!(!config.sync_on_write);
        assert_eq!(config.segment_size, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_storage_engine_creation() {
        let storage = StorageEngine::new("./test_data".to_string());
        let stats = storage.get_stats().await;
        
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.total_messages, 0);
    }

    #[tokio::test]
    async fn test_storage_operations() {
        let storage = StorageEngine::new("./test_data".to_string());
        storage.initialize().await.unwrap();

        // Test write message
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = Offset::new(1);
        
        storage.write_message("test_topic", 0, &message, offset).await.unwrap();

        // Test read message
        let read_message = storage.read_message("test_topic", 0, offset).await.unwrap();
        assert!(read_message.is_some());
        assert_eq!(read_message.unwrap().payload, message.payload);

        // Test stats
        let stats = storage.get_stats().await;
        assert_eq!(stats.total_messages, 1);
    }
}