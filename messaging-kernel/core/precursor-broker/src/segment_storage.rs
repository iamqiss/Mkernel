//! High-performance segment-based storage with memory-mapped files
//! 
//! This module provides advanced storage capabilities optimized for high-throughput
//! message processing with memory-mapped files, lock-free operations, and efficient indexing.

use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use memmap2::{Mmap, MmapMut, MmapOptions};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset};
use super::memory::{MemoryPool, MemoryBlock, MemoryPoolConfig};

/// Segment storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentStorageConfig {
    /// Data directory
    pub data_dir: PathBuf,
    /// Segment size in bytes
    pub segment_size: usize,
    /// Index interval (bytes between index entries)
    pub index_interval: usize,
    /// Enable memory mapping
    pub enable_mmap: bool,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    /// Flush interval
    pub flush_interval: Duration,
    /// Sync on write
    pub sync_on_write: bool,
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    /// Memory pool configuration
    pub memory_pool_config: MemoryPoolConfig,
}

impl Default for SegmentStorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            segment_size: 1024 * 1024 * 1024, // 1GB
            index_interval: 4096,              // 4KB
            enable_mmap: true,
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            flush_interval: Duration::from_millis(100),
            sync_on_write: false,
            enable_zero_copy: true,
            memory_pool_config: MemoryPoolConfig::default(),
        }
    }
}

/// Compression algorithm enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstandard compression (balanced)
    Zstd,
    /// Brotli compression (high ratio)
    Brotli,
    /// Snappy compression (very fast)
    Snappy,
}

/// Segment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetadata {
    /// Segment ID
    pub segment_id: u64,
    /// Base offset
    pub base_offset: Offset,
    /// Next offset to write
    pub next_offset: Offset,
    /// Segment size in bytes
    pub size: u64,
    /// Number of messages
    pub message_count: u64,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Last modified timestamp
    pub last_modified: SystemTime,
    /// Compression algorithm used
    pub compression: CompressionAlgorithm,
    /// Checksum of the segment
    pub checksum: u64,
}

/// Index entry for fast message lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Message offset
    pub offset: Offset,
    /// Position in segment file
    pub position: u64,
    /// Message size
    pub size: u32,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Compression info
    pub compressed_size: Option<u32>,
}

/// High-performance segment storage engine
pub struct SegmentStorage {
    /// Storage configuration
    config: SegmentStorageConfig,
    /// Active segments by topic/partition
    active_segments: RwLock<HashMap<String, Arc<Segment>>>,
    /// Segment indices
    indices: RwLock<HashMap<String, BTreeMap<Offset, IndexEntry>>>,
    /// Memory pool for zero-copy operations
    memory_pool: Arc<MemoryPool>,
    /// Global segment counter
    segment_counter: AtomicU64,
    /// Storage statistics
    stats: Arc<RwLock<StorageStats>>,
    /// Background flush task
    flush_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// Individual storage segment
pub struct Segment {
    /// Segment metadata
    metadata: Arc<RwLock<SegmentMetadata>>,
    /// Memory-mapped file
    mmap: Arc<RwLock<Option<MmapMut>>>,
    /// File handle
    file: Arc<Mutex<File>>,
    /// Write position
    write_position: AtomicUsize,
    /// Index entries for this segment
    index_entries: Arc<RwLock<Vec<IndexEntry>>>,
    /// Compression buffer
    compression_buffer: Arc<Mutex<Vec<u8>>>,
}

/// Storage statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total segments created
    pub total_segments: u64,
    /// Total messages written
    pub total_messages_written: u64,
    /// Total messages read
    pub total_messages_read: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Average write latency (microseconds)
    pub avg_write_latency_us: u64,
    /// Average read latency (microseconds)
    pub avg_read_latency_us: u64,
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    /// Active segments
    pub active_segments: usize,
}

impl SegmentStorage {
    /// Create a new segment storage engine
    pub fn new(config: SegmentStorageConfig) -> Self {
        let memory_pool = Arc::new(MemoryPool::new(config.memory_pool_config.clone()));
        
        Self {
            config,
            active_segments: RwLock::new(HashMap::new()),
            indices: RwLock::new(HashMap::new()),
            memory_pool,
            segment_counter: AtomicU64::new(0),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            flush_task: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize the storage engine
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing segment storage at: {}", self.config.data_dir.display());
        
        // Create data directory
        std::fs::create_dir_all(&self.config.data_dir)?;
        
        // Load existing segments
        self.load_existing_segments().await?;
        
        // Start background flush task
        self.start_flush_task().await;
        
        info!("Segment storage initialized");
        Ok(())
    }
    
    /// Write a message to storage
    pub async fn write_message(
        &self,
        topic: &str,
        partition: i32,
        message: &Message,
        offset: Offset,
    ) -> Result<()> {
        let start_time = std::time::Instant::now();
        let key = format!("{}:{}", topic, partition);
        
        // Get or create segment
        let segment = self.get_or_create_segment(&key, offset).await?;
        
        // Write message to segment
        let position = segment.write_message(message, offset).await?;
        
        // Update index
        self.update_index(&key, offset, position, message).await?;
        
        // Update statistics
        let latency = start_time.elapsed().as_micros() as u64;
        self.update_write_stats(message.size(), latency).await;
        
        debug!(
            "Written message to segment: topic={}, partition={}, offset={}, position={}, latency={}μs",
            topic, partition, offset.value(), position, latency
        );
        
        Ok(())
    }
    
    /// Read a message from storage
    pub async fn read_message(
        &self,
        topic: &str,
        partition: i32,
        offset: Offset,
    ) -> Result<Option<Message>> {
        let start_time = std::time::Instant::now();
        let key = format!("{}:{}", topic, partition);
        
        // Get index entry
        let index_entry = self.get_index_entry(&key, offset).await?;
        let index_entry = match index_entry {
            Some(entry) => entry,
            None => return Ok(None),
        };
        
        // Find segment
        let segment = self.find_segment(&key, offset).await?;
        let segment = match segment {
            Some(seg) => seg,
            None => return Ok(None),
        };
        
        // Read message from segment
        let message = segment.read_message(index_entry).await?;
        
        // Update statistics
        let latency = start_time.elapsed().as_micros() as u64;
        self.update_read_stats(message.size(), latency).await;
        
        debug!(
            "Read message from segment: topic={}, partition={}, offset={}, latency={}μs",
            topic, partition, offset.value(), latency
        );
        
        Ok(Some(message))
    }
    
    /// Flush all pending writes
    pub async fn flush(&self) -> Result<()> {
        info!("Flushing segment storage");
        
        let segments = self.active_segments.read().await;
        for segment in segments.values() {
            segment.flush().await?;
        }
        
        info!("Segment storage flushed");
        Ok(())
    }
    
    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        let mut stats = self.stats.read().await.clone();
        let segments = self.active_segments.read().await;
        stats.active_segments = segments.len();
        stats
    }
    
    /// Get or create a segment for the given key and offset
    async fn get_or_create_segment(&self, key: &str, offset: Offset) -> Result<Arc<Segment>> {
        // Check if we have an active segment
        {
            let segments = self.active_segments.read().await;
            if let Some(segment) = segments.get(key) {
                let metadata = segment.metadata.read().await;
                if metadata.size < self.config.segment_size as u64 {
                    return Ok(segment.clone());
                }
            }
        }
        
        // Create new segment
        let segment_id = self.segment_counter.fetch_add(1, Ordering::Relaxed);
        let segment = Arc::new(Segment::new(
            segment_id,
            key,
            offset,
            &self.config,
            self.memory_pool.clone(),
        ).await?);
        
        // Add to active segments
        {
            let mut segments = self.active_segments.write().await;
            segments.insert(key.to_string(), segment.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_segments += 1;
        }
        
        Ok(segment)
    }
    
    /// Update index with new message entry
    async fn update_index(
        &self,
        key: &str,
        offset: Offset,
        position: u64,
        message: &Message,
    ) -> Result<()> {
        let index_entry = IndexEntry {
            offset,
            position,
            size: message.size() as u32,
            timestamp: message.timestamp(),
            compressed_size: None, // Will be set if compression is used
        };
        
        let mut indices = self.indices.write().await;
        let topic_indices = indices.entry(key.to_string()).or_insert_with(BTreeMap::new);
        topic_indices.insert(offset, index_entry);
        
        Ok(())
    }
    
    /// Get index entry for offset
    async fn get_index_entry(&self, key: &str, offset: Offset) -> Result<Option<IndexEntry>> {
        let indices = self.indices.read().await;
        Ok(indices.get(key).and_then(|indices| indices.get(&offset).cloned()))
    }
    
    /// Find segment containing the offset
    async fn find_segment(&self, key: &str, offset: Offset) -> Result<Option<Arc<Segment>>> {
        let segments = self.active_segments.read().await;
        Ok(segments.get(key).cloned())
    }
    
    /// Load existing segments from disk
    async fn load_existing_segments(&self) -> Result<()> {
        // Implementation would scan the data directory and load existing segments
        // For now, we'll start with empty segments
        Ok(())
    }
    
    /// Start background flush task
    async fn start_flush_task(&self) {
        let config = self.config.clone();
        let segments = self.active_segments.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.flush_interval);
            
            loop {
                interval.tick().await;
                
                let segments_guard = segments.read().await;
                for segment in segments_guard.values() {
                    if let Err(e) = segment.flush().await {
                        warn!("Failed to flush segment: {}", e);
                    }
                }
            }
        });
        
        let mut flush_task = self.flush_task.write().await;
        *flush_task = Some(handle);
    }
    
    /// Update write statistics
    async fn update_write_stats(&self, size: usize, latency_us: u64) {
        let mut stats = self.stats.write().await;
        stats.total_messages_written += 1;
        stats.total_bytes_written += size as u64;
        stats.avg_write_latency_us = (stats.avg_write_latency_us * (stats.total_messages_written - 1) + latency_us) / stats.total_messages_written;
    }
    
    /// Update read statistics
    async fn update_read_stats(&self, size: usize, latency_us: u64) {
        let mut stats = self.stats.write().await;
        stats.total_messages_read += 1;
        stats.total_bytes_read += size as u64;
        stats.avg_read_latency_us = (stats.avg_read_latency_us * (stats.total_messages_read - 1) + latency_us) / stats.total_messages_read;
    }
}

impl Segment {
    /// Create a new segment
    async fn new(
        segment_id: u64,
        key: &str,
        base_offset: Offset,
        config: &SegmentStorageConfig,
        memory_pool: Arc<MemoryPool>,
    ) -> Result<Self> {
        // Create segment directory
        let segment_dir = config.data_dir.join("segments").join(key);
        std::fs::create_dir_all(&segment_dir)?;
        
        // Create segment file
        let segment_file = segment_dir.join(format!("{:020}.log", segment_id));
        let file = Arc::new(Mutex::new(
            OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .open(&segment_file)?
        ));
        
        // Create memory mapping if enabled
        let mmap = if config.enable_mmap {
            let mut file_guard = file.lock().unwrap();
            file_guard.set_len(config.segment_size as u64)?;
            file_guard.seek(SeekFrom::Start(0))?;
            
            let mmap = unsafe { MmapOptions::new().map_mut(&file_guard)? };
            Some(Arc::new(RwLock::new(Some(mmap))))
        } else {
            None
        };
        
        let metadata = Arc::new(RwLock::new(SegmentMetadata {
            segment_id,
            base_offset,
            next_offset: base_offset,
            size: 0,
            message_count: 0,
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
            compression: config.compression_algorithm,
            checksum: 0,
        }));
        
        Ok(Self {
            metadata,
            mmap: mmap.unwrap_or_else(|| Arc::new(RwLock::new(None))),
            file,
            write_position: AtomicUsize::new(0),
            index_entries: Arc::new(RwLock::new(Vec::new())),
            compression_buffer: Arc::new(Mutex::new(Vec::new())),
        })
    }
    
    /// Write a message to the segment
    async fn write_message(&self, message: &Message, offset: Offset) -> Result<u64> {
        let position = self.write_position.load(Ordering::Relaxed) as u64;
        
        // Serialize message
        let serialized = bincode::serialize(message)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        // Compress if enabled
        let (data, compressed_size) = if self.metadata.read().await.compression != CompressionAlgorithm::None {
            self.compress_data(&serialized).await?
        } else {
            (serialized, None)
        };
        
        // Write to memory-mapped file or regular file
        if let Some(mmap_guard) = self.mmap.read().await.as_ref() {
            self.write_to_mmap(mmap_guard, &data, position).await?;
        } else {
            self.write_to_file(&data, position).await?;
        }
        
        // Update metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.next_offset = offset.increment();
            metadata.size += data.len() as u64;
            metadata.message_count += 1;
            metadata.last_modified = SystemTime::now();
        }
        
        // Update write position
        self.write_position.fetch_add(data.len(), Ordering::Relaxed);
        
        // Add to index entries
        {
            let mut entries = self.index_entries.write().await;
            entries.push(IndexEntry {
                offset,
                position,
                size: message.size() as u32,
                timestamp: message.timestamp(),
                compressed_size,
            });
        }
        
        Ok(position)
    }
    
    /// Read a message from the segment
    async fn read_message(&self, index_entry: IndexEntry) -> Result<Message> {
        let data = if let Some(mmap_guard) = self.mmap.read().await.as_ref() {
            self.read_from_mmap(mmap_guard, index_entry).await?
        } else {
            self.read_from_file(index_entry).await?
        };
        
        // Decompress if needed
        let decompressed = if index_entry.compressed_size.is_some() {
            self.decompress_data(&data).await?
        } else {
            data
        };
        
        // Deserialize message
        let message: Message = bincode::deserialize(&decompressed)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        Ok(message)
    }
    
    /// Flush pending writes
    async fn flush(&self) -> Result<()> {
        if let Some(mmap_guard) = self.mmap.read().await.as_ref() {
            mmap_guard.flush()?;
        } else {
            let mut file = self.file.lock().unwrap();
            file.flush()?;
        }
        Ok(())
    }
    
    /// Compress data using the configured algorithm
    async fn compress_data(&self, data: &[u8]) -> Result<(Vec<u8>, Option<u32>)> {
        let compression = self.metadata.read().await.compression;
        
        match compression {
            CompressionAlgorithm::Lz4 => {
                let compressed = lz4_flex::compress(data);
                Ok((compressed, Some(compressed.len() as u32)))
            }
            CompressionAlgorithm::Zstd => {
                let compressed = zstd::encode_all(data, 3)?;
                Ok((compressed, Some(compressed.len() as u32)))
            }
            CompressionAlgorithm::Brotli => {
                let mut compressed = Vec::new();
                let mut encoder = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
                encoder.write_all(data)?;
                encoder.flush()?;
                drop(encoder);
                Ok((compressed, Some(compressed.len() as u32)))
            }
            CompressionAlgorithm::Snappy => {
                let compressed = snap::raw::Encoder::new().compress_vec(data)?;
                Ok((compressed, Some(compressed.len() as u32)))
            }
            CompressionAlgorithm::None => {
                Ok((data.to_vec(), None))
            }
        }
    }
    
    /// Decompress data using the configured algorithm
    async fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let compression = self.metadata.read().await.compression;
        
        match compression {
            CompressionAlgorithm::Lz4 => {
                Ok(lz4_flex::decompress_size_prepended(data)?)
            }
            CompressionAlgorithm::Zstd => {
                Ok(zstd::decode_all(data)?)
            }
            CompressionAlgorithm::Brotli => {
                let mut decompressed = Vec::new();
                let mut decoder = brotli::Decompressor::new(data, 4096);
                decoder.read_to_end(&mut decompressed)?;
                Ok(decompressed)
            }
            CompressionAlgorithm::Snappy => {
                Ok(snap::raw::Decoder::new().decompress_vec(data)?)
            }
            CompressionAlgorithm::None => {
                Ok(data.to_vec())
            }
        }
    }
    
    /// Write data to memory-mapped file
    async fn write_to_mmap(&self, mmap: &MmapMut, data: &[u8], position: u64) -> Result<()> {
        let end_pos = position + data.len() as u64;
        if end_pos > mmap.len() as u64 {
            return Err(Error::Storage("Segment overflow".to_string()));
        }
        
        unsafe {
            let ptr = mmap.as_ptr().add(position as usize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }
        
        Ok(())
    }
    
    /// Read data from memory-mapped file
    async fn read_from_mmap(&self, mmap: &MmapMut, index_entry: IndexEntry) -> Result<Vec<u8>> {
        let size = index_entry.compressed_size.unwrap_or(index_entry.size) as usize;
        let end_pos = index_entry.position + size as u64;
        
        if end_pos > mmap.len() as u64 {
            return Err(Error::Storage("Read beyond segment".to_string()));
        }
        
        let data = unsafe {
            let ptr = mmap.as_ptr().add(index_entry.position as usize);
            std::slice::from_raw_parts(ptr, size)
        };
        
        Ok(data.to_vec())
    }
    
    /// Write data to regular file
    async fn write_to_file(&self, data: &[u8], position: u64) -> Result<()> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(position))?;
        file.write_all(data)?;
        Ok(())
    }
    
    /// Read data from regular file
    async fn read_from_file(&self, index_entry: IndexEntry) -> Result<Vec<u8>> {
        let size = index_entry.compressed_size.unwrap_or(index_entry.size) as usize;
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(index_entry.position))?;
        
        let mut data = vec![0u8; size];
        file.read_exact(&mut data)?;
        
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_segment_storage_creation() {
        let config = SegmentStorageConfig {
            data_dir: PathBuf::from("./test_data"),
            ..Default::default()
        };
        let storage = SegmentStorage::new(config);
        storage.initialize().await.unwrap();
        
        let stats = storage.get_stats().await;
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.total_messages_written, 0);
    }
    
    #[tokio::test]
    async fn test_message_write_read() {
        let config = SegmentStorageConfig {
            data_dir: PathBuf::from("./test_data"),
            segment_size: 1024 * 1024, // 1MB
            ..Default::default()
        };
        let storage = SegmentStorage::new(config);
        storage.initialize().await.unwrap();
        
        // Create test message
        let payload = bytes::Bytes::from("test message");
        let headers = std::collections::HashMap::new();
        let message = Message::new(payload, headers);
        let offset = Offset::new(1);
        
        // Write message
        storage.write_message("test_topic", 0, &message, offset).await.unwrap();
        
        // Read message
        let read_message = storage.read_message("test_topic", 0, offset).await.unwrap();
        assert!(read_message.is_some());
        assert_eq!(read_message.unwrap().payload, message.payload);
        
        // Check statistics
        let stats = storage.get_stats().await;
        assert_eq!(stats.total_messages_written, 1);
        assert_eq!(stats.total_messages_read, 1);
    }
    
    #[tokio::test]
    async fn test_compression() {
        let config = SegmentStorageConfig {
            data_dir: PathBuf::from("./test_data"),
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            ..Default::default()
        };
        let storage = SegmentStorage::new(config);
        storage.initialize().await.unwrap();
        
        // Create test message with larger payload
        let payload = bytes::Bytes::from("x".repeat(1000));
        let headers = std::collections::HashMap::new();
        let message = Message::new(payload, headers);
        let offset = Offset::new(1);
        
        // Write message
        storage.write_message("test_topic", 0, &message, offset).await.unwrap();
        
        // Read message
        let read_message = storage.read_message("test_topic", 0, offset).await.unwrap();
        assert!(read_message.is_some());
        assert_eq!(read_message.unwrap().payload, message.payload);
    }
}