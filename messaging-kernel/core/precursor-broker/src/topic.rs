//! Topic implementation for Precursor broker

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset, StorageEngine};

/// Topic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    /// Topic name
    pub name: String,
    /// Number of partitions
    pub partition_count: u32,
    /// Message retention period
    pub retention: Duration,
    /// Enable persistence
    pub enable_persistence: bool,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u32,
    /// Segment size
    pub segment_size: usize,
    /// Index interval
    pub index_interval: usize,
    /// Cleanup policy
    pub cleanup_policy: CleanupPolicy,
    /// Replication factor
    pub replication_factor: u32,
}

impl Default for TopicConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            partition_count: 1,
            retention: Duration::from_secs(7 * 24 * 3600), // 7 days
            enable_persistence: true,
            enable_compression: false,
            compression_level: 6,
            segment_size: 1024 * 1024, // 1MB
            index_interval: 4096, // 4KB
            cleanup_policy: CleanupPolicy::Delete,
            replication_factor: 1,
        }
    }
}

/// Cleanup policy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CleanupPolicy {
    /// Delete old messages
    Delete,
    /// Compact messages (keep latest for each key)
    Compact,
}

/// Topic partition
pub struct Partition {
    /// Partition ID
    partition_id: i32,
    /// Topic name
    topic_name: String,
    /// Messages in partition
    messages: Arc<RwLock<VecDeque<Message>>>,
    /// Next offset
    next_offset: Arc<RwLock<Offset>>,
    /// Consumer group offsets
    consumer_group_offsets: Arc<RwLock<HashMap<String, HashMap<String, Offset>>>>,
    /// Storage engine
    storage: Arc<StorageEngine>,
    /// Partition statistics
    stats: Arc<RwLock<PartitionStats>>,
}

/// Partition statistics
#[derive(Debug, Default)]
pub struct PartitionStats {
    /// Total messages produced
    pub total_produced: u64,
    /// Total messages consumed
    pub total_consumed: u64,
    /// Current partition size
    pub current_size: usize,
    /// Average message size
    pub avg_message_size: u64,
    /// Oldest message timestamp
    pub oldest_message: Option<SystemTime>,
    /// Newest message timestamp
    pub newest_message: Option<SystemTime>,
}

impl Partition {
    /// Create a new partition
    pub fn new(partition_id: i32, topic_name: String, storage: Arc<StorageEngine>) -> Self {
        Self {
            partition_id,
            topic_name,
            messages: Arc::new(RwLock::new(VecDeque::new())),
            next_offset: Arc::new(RwLock::new(Offset::new(0))),
            consumer_group_offsets: Arc::new(RwLock::new(HashMap::new())),
            storage,
            stats: Arc::new(RwLock::new(PartitionStats::default())),
        }
    }

    /// Initialize the partition
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing partition {} for topic {}", self.partition_id, self.topic_name);

        // Load persisted messages if needed
        self.load_messages().await?;

        info!("Partition {} initialized for topic {}", self.partition_id, self.topic_name);
        Ok(())
    }

    /// Delete the partition
    pub async fn delete(&self) -> Result<()> {
        info!("Deleting partition {} for topic {}", self.partition_id, self.topic_name);

        // Clear in-memory messages
        {
            let mut messages = self.messages.write().await;
            messages.clear();
        }

        // Clear consumer group offsets
        {
            let mut offsets = self.consumer_group_offsets.write().await;
            offsets.clear();
        }

        // Delete persisted data
        self.storage.delete_partition(&self.topic_name, self.partition_id).await?;

        info!("Partition {} deleted for topic {}", self.partition_id, self.topic_name);
        Ok(())
    }

    /// Get partition ID
    pub fn partition_id(&self) -> i32 {
        self.partition_id
    }

    /// Produce a message to the partition
    pub async fn produce(&self, mut message: Message) -> Result<Offset> {
        // Generate offset
        let offset = {
            let mut next_offset = self.next_offset.write().await;
            let current_offset = *next_offset;
            *next_offset = next_offset.increment();
            current_offset
        };

        // Set message ID
        message.metadata.message_id = offset.value();

        // Add to in-memory partition
        {
            let mut messages = self.messages.write().await;
            messages.push_back(message.clone());
        }

        // Persist message if needed
        self.persist_message(&message, offset).await?;

        // Update statistics
        self.update_produce_stats(&message).await;

        debug!(
            "Produced message to partition {} of topic {} at offset {}",
            self.partition_id,
            self.topic_name,
            offset.value()
        );

        Ok(offset)
    }

    /// Consume a message from the partition
    pub async fn consume(
        &self,
        consumer_group: &str,
        consumer_id: &str,
    ) -> Result<Option<Message>> {
        // Get consumer group offset
        let consumer_offset = self.get_consumer_group_offset(consumer_group, consumer_id).await;
        
        // Find message at or after the offset
        let messages = self.messages.read().await;
        for (index, message) in messages.iter().enumerate() {
            let message_offset = Offset::new(message.metadata.message_id);
            if message_offset >= consumer_offset {
                // Update consumer group offset
                self.set_consumer_group_offset(
                    consumer_group,
                    consumer_id,
                    message_offset.increment(),
                ).await?;

                // Update statistics
                self.update_consume_stats().await;

                debug!(
                    "Consumed message from partition {} of topic {} at offset {} by consumer group {}",
                    self.partition_id,
                    self.topic_name,
                    message_offset.value(),
                    consumer_group
                );

                return Ok(Some(message.clone()));
            }
        }

        Ok(None)
    }

    /// Get consumer group offset
    pub async fn get_consumer_group_offset(&self, consumer_group: &str, consumer_id: &str) -> Offset {
        let offsets = self.consumer_group_offsets.read().await;
        if let Some(group_offsets) = offsets.get(consumer_group) {
            if let Some(offset) = group_offsets.get(consumer_id) {
                return *offset;
            }
        }
        Offset::new(0) // Default to beginning
    }

    /// Set consumer group offset
    pub async fn set_consumer_group_offset(
        &self,
        consumer_group: &str,
        consumer_id: &str,
        offset: Offset,
    ) -> Result<()> {
        let mut offsets = self.consumer_group_offsets.write().await;
        let group_offsets = offsets.entry(consumer_group.to_string()).or_insert_with(HashMap::new);
        group_offsets.insert(consumer_id.to_string(), offset);
        Ok(())
    }

    /// Get partition statistics
    pub async fn get_stats(&self) -> PartitionStats {
        let stats = self.stats.read().await;
        let messages = self.messages.read().await;
        
        PartitionStats {
            total_produced: stats.total_produced,
            total_consumed: stats.total_consumed,
            current_size: messages.len(),
            avg_message_size: stats.avg_message_size,
            oldest_message: stats.oldest_message,
            newest_message: stats.newest_message,
        }
    }

    /// Cleanup expired messages
    pub async fn cleanup_expired_messages(&self, retention: Duration) -> Result<()> {
        let now = SystemTime::now();
        let mut messages = self.messages.write().await;
        let mut removed_count = 0;

        // Remove expired messages
        messages.retain(|message| {
            let message_age = now.duration_since(message.timestamp()).unwrap_or_default();
            
            if message_age > retention {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        if removed_count > 0 {
            info!(
                "Cleaned up {} expired messages from partition {} of topic {}",
                removed_count,
                self.partition_id,
                self.topic_name
            );
        }

        Ok(())
    }

    /// Load messages from storage
    async fn load_messages(&self) -> Result<()> {
        // TODO: Implement message loading from storage
        // This would involve reading persisted messages and reconstructing the partition
        Ok(())
    }

    /// Persist a message to storage
    async fn persist_message(&self, message: &Message, offset: Offset) -> Result<()> {
        // TODO: Implement message persistence to storage
        // This would involve serializing the message and writing it to disk
        Ok(())
    }

    /// Update produce statistics
    async fn update_produce_stats(&self, message: &Message) {
        let mut stats = self.stats.write().await;
        stats.total_produced += 1;
        
        // Update average message size
        let total_size = stats.avg_message_size * (stats.total_produced - 1) + message.size() as u64;
        stats.avg_message_size = total_size / stats.total_produced;
        
        // Update timestamp bounds
        let message_time = message.timestamp();
        if stats.oldest_message.is_none() || message_time < stats.oldest_message.unwrap() {
            stats.oldest_message = Some(message_time);
        }
        if stats.newest_message.is_none() || message_time > stats.newest_message.unwrap() {
            stats.newest_message = Some(message_time);
        }
    }

    /// Update consume statistics
    async fn update_consume_stats(&self) {
        let mut stats = self.stats.write().await;
        stats.total_consumed += 1;
    }
}

/// Topic implementation
pub struct Topic {
    /// Topic name
    name: String,
    /// Topic configuration
    config: TopicConfig,
    /// Partitions
    partitions: Arc<RwLock<HashMap<i32, Arc<Partition>>>>,
    /// Storage engine
    storage: Arc<StorageEngine>,
    /// Topic statistics
    stats: Arc<RwLock<TopicStats>>,
}

/// Topic statistics
#[derive(Debug, Default)]
pub struct TopicStats {
    /// Total messages produced
    pub total_produced: u64,
    /// Total messages consumed
    pub total_consumed: u64,
    /// Total partitions
    pub partition_count: u32,
    /// Average message size
    pub avg_message_size: u64,
}

impl Topic {
    /// Create a new topic
    pub fn new(name: String, config: TopicConfig, storage: Arc<StorageEngine>) -> Self {
        Self {
            name,
            config,
            partitions: Arc::new(RwLock::new(HashMap::new())),
            storage,
            stats: Arc::new(RwLock::new(TopicStats::default())),
        }
    }

    /// Initialize the topic
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing topic: {}", self.name);

        // Create partitions
        for partition_id in 0..self.config.partition_count as i32 {
            let partition = Arc::new(Partition::new(
                partition_id,
                self.name.clone(),
                self.storage.clone(),
            ));
            partition.initialize().await?;

            let mut partitions = self.partitions.write().await;
            partitions.insert(partition_id, partition);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.partition_count = self.config.partition_count;
        }

        info!("Topic initialized: {} with {} partitions", self.name, self.config.partition_count);
        Ok(())
    }

    /// Delete the topic
    pub async fn delete(&self) -> Result<()> {
        info!("Deleting topic: {}", self.name);

        // Delete all partitions
        {
            let partitions = self.partitions.read().await;
            for partition in partitions.values() {
                partition.delete().await?;
            }
        }

        // Clear partitions
        {
            let mut partitions = self.partitions.write().await;
            partitions.clear();
        }

        info!("Topic deleted: {}", self.name);
        Ok(())
    }

    /// Get topic name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get topic configuration
    pub fn config(&self) -> &TopicConfig {
        &self.config
    }

    /// Produce a message to the topic
    pub async fn produce(&self, message: Message) -> Result<Offset> {
        // Determine partition based on partition key or round-robin
        let partition_id = self.select_partition(&message);
        
        // Get partition
        let partitions = self.partitions.read().await;
        let partition = partitions.get(&partition_id)
            .ok_or_else(|| Error::PartitionNotFound(format!("Partition {} not found", partition_id)))?;

        // Produce to partition
        let offset = partition.produce(message).await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_produced += 1;
        }

        debug!("Produced message to topic {} at offset {}", self.name, offset.value());
        Ok(offset)
    }

    /// Consume a message from the topic
    pub async fn consume(
        &self,
        partition: i32,
        consumer_group: &str,
        consumer_id: &str,
    ) -> Result<Option<Message>> {
        // Get partition
        let partitions = self.partitions.read().await;
        let partition = partitions.get(&partition)
            .ok_or_else(|| Error::PartitionNotFound(format!("Partition {} not found", partition)))?;

        // Consume from partition
        let message = partition.consume(consumer_group, consumer_id).await?;

        if message.is_some() {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_consumed += 1;
        }

        Ok(message)
    }

    /// Get topic statistics
    pub async fn get_stats(&self) -> TopicStats {
        let stats = self.stats.read().await;
        let partitions = self.partitions.read().await;
        
        TopicStats {
            total_produced: stats.total_produced,
            total_consumed: stats.total_consumed,
            partition_count: partitions.len() as u32,
            avg_message_size: stats.avg_message_size,
        }
    }

    /// Get partition statistics
    pub async fn get_partition_stats(&self, partition_id: i32) -> Result<PartitionStats> {
        let partitions = self.partitions.read().await;
        let partition = partitions.get(&partition_id)
            .ok_or_else(|| Error::PartitionNotFound(format!("Partition {} not found", partition_id)))?;
        
        Ok(partition.get_stats().await)
    }

    /// Cleanup expired messages
    pub async fn cleanup_expired_messages(&self) -> Result<()> {
        let partitions = self.partitions.read().await;
        for partition in partitions.values() {
            partition.cleanup_expired_messages(self.config.retention).await?;
        }
        Ok(())
    }

    /// Select partition for a message
    fn select_partition(&self, message: &Message) -> i32 {
        if let Some(partition_key) = message.partition_key() {
            // Use partition key to determine partition
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            partition_key.hash(&mut hasher);
            let hash = hasher.finish();
            
            (hash % self.config.partition_count as u64) as i32
        } else {
            // Round-robin partition selection
            // For simplicity, we'll use a random selection
            // In a real implementation, you'd want to track the last used partition
            (SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() % self.config.partition_count as u128) as i32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_topic_config_default() {
        let config = TopicConfig::default();
        assert_eq!(config.name, "default");
        assert_eq!(config.partition_count, 1);
        assert!(config.enable_persistence);
        assert!(!config.enable_compression);
        assert_eq!(config.cleanup_policy, CleanupPolicy::Delete);
    }

    #[tokio::test]
    async fn test_topic_operations() {
        let config = TopicConfig {
            name: "test_topic".to_string(),
            partition_count: 2,
            ..Default::default()
        };
        let storage = Arc::new(StorageEngine::new("./test_data".to_string()));
        let topic = Topic::new("test_topic".to_string(), config, storage);
        
        topic.initialize().await.unwrap();

        // Test produce
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = topic.produce(message).await.unwrap();
        
        assert_eq!(offset.value(), 0);

        // Test consume
        let consumed_message = topic.consume(0, "test_group", "consumer1").await.unwrap();
        assert!(consumed_message.is_some());

        // Test stats
        let stats = topic.get_stats().await;
        assert_eq!(stats.total_produced, 1);
        assert_eq!(stats.total_consumed, 1);
        assert_eq!(stats.partition_count, 2);
    }

    #[tokio::test]
    async fn test_partition_operations() {
        let storage = Arc::new(StorageEngine::new("./test_data".to_string()));
        let partition = Partition::new(0, "test_topic".to_string(), storage);
        
        partition.initialize().await.unwrap();

        // Test produce
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = partition.produce(message).await.unwrap();
        
        assert_eq!(offset.value(), 0);

        // Test consume
        let consumed_message = partition.consume("test_group", "consumer1").await.unwrap();
        assert!(consumed_message.is_some());

        // Test consumer group offset
        let consumer_offset = partition.get_consumer_group_offset("test_group", "consumer1").await;
        assert_eq!(consumer_offset.value(), 1); // Should be incremented after consume
    }
}