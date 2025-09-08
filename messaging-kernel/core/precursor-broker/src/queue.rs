//! Queue implementation for Precursor broker

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset, StorageEngine};

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Queue name
    pub name: String,
    /// Maximum queue size
    pub max_size: usize,
    /// Message retention period
    pub retention: Duration,
    /// Enable persistence
    pub enable_persistence: bool,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u32,
    /// Dead letter queue
    pub dead_letter_queue: Option<String>,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Visibility timeout
    pub visibility_timeout: Duration,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            max_size: 10000,
            retention: Duration::from_secs(7 * 24 * 3600), // 7 days
            enable_persistence: true,
            enable_compression: false,
            compression_level: 6,
            dead_letter_queue: None,
            max_retry_attempts: 3,
            visibility_timeout: Duration::from_secs(30),
        }
    }
}

/// Queue message with metadata
#[derive(Debug, Clone)]
pub struct QueueMessage {
    /// Message
    pub message: Message,
    /// Offset in queue
    pub offset: Offset,
    /// Delivery count
    pub delivery_count: u32,
    /// First delivery time
    pub first_delivery_time: SystemTime,
    /// Last delivery time
    pub last_delivery_time: SystemTime,
    /// Visibility timeout
    pub visibility_timeout: SystemTime,
    /// Consumer ID (if currently being processed)
    pub consumer_id: Option<String>,
}

impl QueueMessage {
    /// Create a new queue message
    pub fn new(message: Message, offset: Offset) -> Self {
        let now = SystemTime::now();
        Self {
            message,
            offset,
            delivery_count: 0,
            first_delivery_time: now,
            last_delivery_time: now,
            visibility_timeout: now,
            consumer_id: None,
        }
    }

    /// Check if message is visible (not being processed)
    pub fn is_visible(&self) -> bool {
        SystemTime::now() >= self.visibility_timeout
    }

    /// Check if message has exceeded max retry attempts
    pub fn has_exceeded_max_retries(&self, max_retries: u32) -> bool {
        self.delivery_count >= max_retries
    }

    /// Mark message as delivered
    pub fn mark_delivered(&mut self, consumer_id: String, visibility_timeout: Duration) {
        self.delivery_count += 1;
        self.last_delivery_time = SystemTime::now();
        self.visibility_timeout = SystemTime::now() + visibility_timeout;
        self.consumer_id = Some(consumer_id);
    }

    /// Mark message as processed
    pub fn mark_processed(&mut self) {
        self.consumer_id = None;
    }
}

/// Queue implementation
pub struct Queue {
    /// Queue name
    name: String,
    /// Queue configuration
    config: QueueConfig,
    /// Message storage
    messages: Arc<RwLock<VecDeque<QueueMessage>>>,
    /// Consumer offsets
    consumer_offsets: Arc<RwLock<HashMap<String, Offset>>>,
    /// Storage engine
    storage: Arc<StorageEngine>,
    /// Next offset
    next_offset: Arc<RwLock<Offset>>,
    /// Queue statistics
    stats: Arc<RwLock<QueueStats>>,
}

/// Queue statistics
#[derive(Debug, Default)]
pub struct QueueStats {
    /// Total messages produced
    pub total_produced: u64,
    /// Total messages consumed
    pub total_consumed: u64,
    /// Total messages failed
    pub total_failed: u64,
    /// Current queue size
    pub current_size: usize,
    /// Average message size
    pub avg_message_size: u64,
    /// Oldest message timestamp
    pub oldest_message: Option<SystemTime>,
    /// Newest message timestamp
    pub newest_message: Option<SystemTime>,
}

impl Queue {
    /// Create a new queue
    pub fn new(name: String, config: QueueConfig, storage: Arc<StorageEngine>) -> Self {
        Self {
            name,
            config,
            messages: Arc::new(RwLock::new(VecDeque::new())),
            consumer_offsets: Arc::new(RwLock::new(HashMap::new())),
            storage,
            next_offset: Arc::new(RwLock::new(Offset::new(0))),
            stats: Arc::new(RwLock::new(QueueStats::default())),
        }
    }

    /// Initialize the queue
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing queue: {}", self.name);

        // Load persisted messages if persistence is enabled
        if self.config.enable_persistence {
            self.load_messages().await?;
        }

        info!("Queue initialized: {}", self.name);
        Ok(())
    }

    /// Delete the queue
    pub async fn delete(&self) -> Result<()> {
        info!("Deleting queue: {}", self.name);

        // Clear in-memory messages
        {
            let mut messages = self.messages.write().await;
            messages.clear();
        }

        // Clear consumer offsets
        {
            let mut offsets = self.consumer_offsets.write().await;
            offsets.clear();
        }

        // Delete persisted data if persistence is enabled
        if self.config.enable_persistence {
            self.storage.delete_queue(&self.name).await?;
        }

        info!("Queue deleted: {}", self.name);
        Ok(())
    }

    /// Get queue name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get queue configuration
    pub fn config(&self) -> &QueueConfig {
        &self.config
    }

    /// Produce a message to the queue
    pub async fn produce(&self, mut message: Message) -> Result<Offset> {
        // Check queue size limit
        {
            let messages = self.messages.read().await;
            if messages.len() >= self.config.max_size {
                return Err(Error::QueueFull(self.name.clone()));
            }
        }

        // Generate offset
        let offset = {
            let mut next_offset = self.next_offset.write().await;
            let current_offset = *next_offset;
            *next_offset = next_offset.increment();
            current_offset
        };

        // Set message ID
        message.metadata.message_id = offset.value();

        // Create queue message
        let queue_message = QueueMessage::new(message, offset);

        // Add to in-memory queue
        {
            let mut messages = self.messages.write().await;
            messages.push_back(queue_message.clone());
        }

        // Persist message if persistence is enabled
        if self.config.enable_persistence {
            self.persist_message(&queue_message).await?;
        }

        // Update statistics
        self.update_produce_stats(&queue_message).await;

        debug!("Produced message to queue {} at offset {}", self.name, offset.value());
        Ok(offset)
    }

    /// Consume a message from the queue
    pub async fn consume(&self, consumer_id: &str) -> Result<Option<Message>> {
        let mut messages = self.messages.write().await;
        
        // Find the next visible message
        for (index, queue_message) in messages.iter_mut().enumerate() {
            if queue_message.is_visible() {
                // Mark message as delivered
                queue_message.mark_delivered(
                    consumer_id.to_string(),
                    self.config.visibility_timeout,
                );

                // Update consumer offset
                {
                    let mut offsets = self.consumer_offsets.write().await;
                    offsets.insert(consumer_id.to_string(), queue_message.offset);
                }

                // Update statistics
                self.update_consume_stats().await;

                debug!(
                    "Consumed message from queue {} at offset {} by consumer {}",
                    self.name,
                    queue_message.offset.value(),
                    consumer_id
                );

                return Ok(Some(queue_message.message.clone()));
            }
        }

        Ok(None)
    }

    /// Acknowledge a message
    pub async fn acknowledge(&self, consumer_id: &str, offset: Offset) -> Result<()> {
        let mut messages = self.messages.write().await;
        
        // Find and remove the acknowledged message
        if let Some(pos) = messages.iter().position(|qm| qm.offset == offset) {
            let queue_message = messages.remove(pos).unwrap();
            
            // Check if message should go to dead letter queue
            if queue_message.has_exceeded_max_retries(self.config.max_retry_attempts) {
                if let Some(dlq_name) = &self.config.dead_letter_queue {
                    // Send to dead letter queue
                    warn!(
                        "Message at offset {} exceeded max retries, sending to DLQ: {}",
                        offset.value(),
                        dlq_name
                    );
                    // TODO: Implement dead letter queue functionality
                }
            }

            // Update statistics
            self.update_ack_stats().await;

            debug!(
                "Acknowledged message at offset {} by consumer {}",
                offset.value(),
                consumer_id
            );
        }

        Ok(())
    }

    /// Get consumer offset
    pub async fn get_consumer_offset(&self, consumer_id: &str) -> Option<Offset> {
        let offsets = self.consumer_offsets.read().await;
        offsets.get(consumer_id).copied()
    }

    /// Set consumer offset
    pub async fn set_consumer_offset(&self, consumer_id: &str, offset: Offset) -> Result<()> {
        let mut offsets = self.consumer_offsets.write().await;
        offsets.insert(consumer_id.to_string(), offset);
        Ok(())
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> QueueStats {
        let stats = self.stats.read().await;
        let messages = self.messages.read().await;
        
        QueueStats {
            total_produced: stats.total_produced,
            total_consumed: stats.total_consumed,
            total_failed: stats.total_failed,
            current_size: messages.len(),
            avg_message_size: stats.avg_message_size,
            oldest_message: stats.oldest_message,
            newest_message: stats.newest_message,
        }
    }

    /// Cleanup expired messages
    pub async fn cleanup_expired_messages(&self) -> Result<()> {
        let now = SystemTime::now();
        let mut messages = self.messages.write().await;
        let mut removed_count = 0;

        // Remove expired messages
        messages.retain(|queue_message| {
            let message_age = now.duration_since(queue_message.message.timestamp())
                .unwrap_or_default();
            
            if message_age > self.config.retention {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        if removed_count > 0 {
            info!(
                "Cleaned up {} expired messages from queue {}",
                removed_count,
                self.name
            );
        }

        Ok(())
    }

    /// Load messages from storage
    async fn load_messages(&self) -> Result<()> {
        // TODO: Implement message loading from storage
        // This would involve reading persisted messages and reconstructing the queue
        Ok(())
    }

    /// Persist a message to storage
    async fn persist_message(&self, queue_message: &QueueMessage) -> Result<()> {
        // TODO: Implement message persistence to storage
        // This would involve serializing the message and writing it to disk
        Ok(())
    }

    /// Update produce statistics
    async fn update_produce_stats(&self, queue_message: &QueueMessage) {
        let mut stats = self.stats.write().await;
        stats.total_produced += 1;
        
        // Update average message size
        let total_size = stats.avg_message_size * (stats.total_produced - 1) + queue_message.message.size() as u64;
        stats.avg_message_size = total_size / stats.total_produced;
        
        // Update timestamp bounds
        let message_time = queue_message.message.timestamp();
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

    /// Update acknowledge statistics
    async fn update_ack_stats(&self) {
        // Acknowledged messages are considered successfully processed
        // This could be used for success rate calculations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.name, "default");
        assert_eq!(config.max_size, 10000);
        assert!(config.enable_persistence);
        assert!(!config.enable_compression);
    }

    #[test]
    fn test_queue_message_creation() {
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = Offset::new(1);
        let queue_message = QueueMessage::new(message, offset);

        assert_eq!(queue_message.offset, offset);
        assert_eq!(queue_message.delivery_count, 0);
        assert!(queue_message.is_visible());
        assert!(!queue_message.has_exceeded_max_retries(3));
    }

    #[test]
    fn test_queue_message_delivery() {
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = Offset::new(1);
        let mut queue_message = QueueMessage::new(message, offset);

        queue_message.mark_delivered("consumer1".to_string(), Duration::from_secs(30));
        
        assert_eq!(queue_message.delivery_count, 1);
        assert_eq!(queue_message.consumer_id, Some("consumer1".to_string()));
        assert!(!queue_message.is_visible()); // Should not be visible during processing
    }

    #[tokio::test]
    async fn test_queue_operations() {
        let config = QueueConfig::default();
        let storage = Arc::new(StorageEngine::new("./test_data".to_string()));
        let queue = Queue::new("test_queue".to_string(), config, storage);
        
        queue.initialize().await.unwrap();

        // Test produce
        let payload = bytes::Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        let offset = queue.produce(message).await.unwrap();
        
        assert_eq!(offset.value(), 0);

        // Test consume
        let consumed_message = queue.consume("consumer1").await.unwrap();
        assert!(consumed_message.is_some());

        // Test acknowledge
        queue.acknowledge("consumer1", offset).await.unwrap();

        // Test stats
        let stats = queue.get_stats().await;
        assert_eq!(stats.total_produced, 1);
        assert_eq!(stats.total_consumed, 1);
    }
}