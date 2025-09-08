//! Built-in message broker with zero-copy optimization
//! 
//! This crate is part of the Neo Messaging Kernel, providing built-in message broker with zero-copy optimization.
//! 
//! The Precursor broker is designed for maximum performance with zero-copy message passing where possible.
//! It provides both queue-based messaging and topic-based pub/sub with persistence and replication.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc, oneshot, Mutex, broadcast};
use tokio::time::{interval, timeout};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

pub mod error;
pub mod queue;
pub mod topic;
pub mod consumer;
pub mod producer;
pub mod storage;
pub mod replication;
pub mod cluster;
pub mod memory;
pub mod segment_storage;
pub mod compression;
pub mod networking;
pub mod consumer_groups;
pub mod monitoring;
pub mod enhanced_broker;
pub mod security;
pub mod consensus;
pub mod advanced_replication;

pub use error::Error;
pub use queue::*;
pub use topic::*;
pub use consumer::*;
pub use producer::*;
pub use storage::*;
pub use replication::*;
pub use cluster::*;
pub use memory::*;
pub use segment_storage::*;
pub use compression::*;
pub use networking::*;
pub use consumer_groups::*;
pub use monitoring::*;
pub use enhanced_broker::*;
pub use security::*;
pub use consensus::*;
pub use advanced_replication::*;

/// Result type for this crate
pub type Result<T> = std::result::Result<T, Error>;

/// Precursor broker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerConfig {
    /// Broker ID
    pub broker_id: String,
    /// Data directory
    pub data_dir: String,
    /// Maximum message size
    pub max_message_size: usize,
    /// Default retention period
    pub default_retention: Duration,
    /// Enable persistence
    pub enable_persistence: bool,
    /// Enable replication
    pub enable_replication: bool,
    /// Replication factor
    pub replication_factor: u32,
    /// Sync replication
    pub sync_replication: bool,
    /// Compression enabled
    pub enable_compression: bool,
    /// Compression level
    pub compression_level: u32,
    /// Flush interval
    pub flush_interval: Duration,
    /// Segment size
    pub segment_size: usize,
    /// Index interval
    pub index_interval: usize,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Cluster configuration
    pub cluster: ClusterConfig,
}

impl Default for BrokerConfig {
    fn default() -> Self {
        Self {
            broker_id: "broker-1".to_string(),
            data_dir: "./data".to_string(),
            max_message_size: 10 * 1024 * 1024, // 10MB
            default_retention: Duration::from_secs(7 * 24 * 3600), // 7 days
            enable_persistence: true,
            enable_replication: false,
            replication_factor: 1,
            sync_replication: false,
            enable_compression: false,
            compression_level: 6,
            flush_interval: Duration::from_millis(100),
            segment_size: 1024 * 1024, // 1MB
            index_interval: 4096, // 4KB
            cleanup_interval: Duration::from_secs(3600), // 1 hour
            cluster: ClusterConfig::default(),
        }
    }
}

/// Message metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Message ID
    pub message_id: u64,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Message size
    pub size: usize,
    /// Compression type
    pub compression: Option<CompressionType>,
    /// Checksum
    pub checksum: u32,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Partition key
    pub partition_key: Option<String>,
}

/// Compression type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// Snappy compression
    Snappy,
}

/// Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message metadata
    pub metadata: MessageMetadata,
    /// Message payload
    pub payload: Bytes,
}

impl Message {
    /// Create a new message
    pub fn new(payload: Bytes, headers: HashMap<String, String>) -> Self {
        let size = payload.len();
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&payload);
        let checksum = hasher.finalize();

        Self {
            metadata: MessageMetadata {
                message_id: 0, // Will be set by the broker
                timestamp: SystemTime::now(),
                size,
                compression: None,
                checksum,
                headers,
                partition_key: None,
            },
            payload,
        }
    }

    /// Create a message with partition key
    pub fn with_partition_key(payload: Bytes, headers: HashMap<String, String>, partition_key: String) -> Self {
        let mut message = Self::new(payload, headers);
        message.metadata.partition_key = Some(partition_key);
        message
    }

    /// Get message size
    pub fn size(&self) -> usize {
        self.metadata.size
    }

    /// Get message timestamp
    pub fn timestamp(&self) -> SystemTime {
        self.metadata.timestamp
    }

    /// Get message ID
    pub fn message_id(&self) -> u64 {
        self.metadata.message_id
    }

    /// Get partition key
    pub fn partition_key(&self) -> Option<&String> {
        self.metadata.partition_key.as_ref()
    }

    /// Get header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.metadata.headers.get(key)
    }

    /// Set header value
    pub fn set_header(&mut self, key: String, value: String) {
        self.metadata.headers.insert(key, value);
    }
}

/// Message offset
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Offset(pub u64);

impl Offset {
    /// Create a new offset
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the offset value
    pub fn value(self) -> u64 {
        self.0
    }

    /// Increment the offset
    pub fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    /// Decrement the offset
    pub fn decrement(self) -> Self {
        Self(self.0.saturating_sub(1))
    }
}

/// Consumer group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroup {
    /// Group ID
    pub group_id: String,
    /// Group members
    pub members: HashMap<String, ConsumerMember>,
    /// Group coordinator
    pub coordinator: Option<String>,
    /// Group state
    pub state: ConsumerGroupState,
    /// Protocol type
    pub protocol_type: String,
    /// Generation ID
    pub generation_id: u32,
    /// Leader ID
    pub leader_id: Option<String>,
    /// Protocol metadata
    pub protocol_metadata: Bytes,
}

/// Consumer group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerMember {
    /// Member ID
    pub member_id: String,
    /// Client ID
    pub client_id: String,
    /// Client host
    pub client_host: String,
    /// Session timeout
    pub session_timeout: Duration,
    /// Rebalance timeout
    pub rebalance_timeout: Duration,
    /// Subscription
    pub subscription: Vec<String>,
    /// Assignment
    pub assignment: HashMap<String, Vec<i32>>,
    /// Last heartbeat
    pub last_heartbeat: SystemTime,
}

/// Consumer group state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsumerGroupState {
    /// Empty group
    Empty,
    /// Preparing rebalance
    PreparingRebalance,
    /// Completing rebalance
    CompletingRebalance,
    /// Stable
    Stable,
    /// Dead
    Dead,
}

/// Precursor broker
pub struct PrecursorBroker {
    /// Broker configuration
    config: BrokerConfig,
    /// Queues
    queues: Arc<RwLock<HashMap<String, Arc<Queue>>>>,
    /// Topics
    topics: Arc<RwLock<HashMap<String, Arc<Topic>>>>,
    /// Producers
    producers: Arc<RwLock<HashMap<String, Arc<Producer>>>>,
    /// Consumers
    consumers: Arc<RwLock<HashMap<String, Arc<Consumer>>>>,
    /// Consumer groups
    consumer_groups: Arc<RwLock<HashMap<String, ConsumerGroup>>>,
    /// Storage engine
    storage: Arc<StorageEngine>,
    /// Replication manager
    replication: Arc<ReplicationManager>,
    /// Cluster manager
    cluster: Arc<ClusterManager>,
    /// Broker statistics
    stats: Arc<RwLock<BrokerStats>>,
    /// Message ID counter
    message_id_counter: Arc<RwLock<u64>>,
    /// Shutdown signal
    shutdown_tx: Option<broadcast::Sender<()>>,
}

/// Broker statistics
#[derive(Debug, Default)]
pub struct BrokerStats {
    /// Total messages produced
    pub total_messages_produced: u64,
    /// Total messages consumed
    pub total_messages_consumed: u64,
    /// Total bytes produced
    pub total_bytes_produced: u64,
    /// Total bytes consumed
    pub total_bytes_consumed: u64,
    /// Active producers
    pub active_producers: usize,
    /// Active consumers
    pub active_consumers: usize,
    /// Active queues
    pub active_queues: usize,
    /// Active topics
    pub active_topics: usize,
    /// Consumer groups
    pub consumer_groups: usize,
    /// Uptime
    pub uptime: Duration,
}

impl PrecursorBroker {
    /// Create a new Precursor broker
    pub fn new(config: BrokerConfig) -> Self {
        Self {
            storage: Arc::new(StorageEngine::new(config.data_dir.clone())),
            replication: Arc::new(ReplicationManager::new(config.clone())),
            cluster: Arc::new(ClusterManager::new(config.cluster.clone())),
            config,
            queues: Arc::new(RwLock::new(HashMap::new())),
            topics: Arc::new(RwLock::new(HashMap::new())),
            producers: Arc::new(RwLock::new(HashMap::new())),
            consumers: Arc::new(RwLock::new(HashMap::new())),
            consumer_groups: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(BrokerStats::default())),
            message_id_counter: Arc::new(RwLock::new(0)),
            shutdown_tx: None,
        }
    }

    /// Start the broker
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Precursor broker: {}", self.config.broker_id);

        // Initialize storage
        self.storage.initialize().await?;

        // Start replication manager
        if self.config.enable_replication {
            self.replication.start().await?;
        }

        // Start cluster manager
        self.cluster.start().await?;

        // Start background tasks
        self.start_background_tasks().await;

        info!("Precursor broker started successfully");
        Ok(())
    }

    /// Stop the broker
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Precursor broker");

        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }

        // Stop replication manager
        if self.config.enable_replication {
            self.replication.stop().await?;
        }

        // Stop cluster manager
        self.cluster.stop().await?;

        // Flush storage
        self.storage.flush().await?;

        info!("Precursor broker stopped");
        Ok(())
    }

    /// Create a queue
    pub async fn create_queue(&self, name: String, config: QueueConfig) -> Result<()> {
        let queue = Arc::new(Queue::new(name.clone(), config, self.storage.clone()));
        queue.initialize().await?;

        let mut queues = self.queues.write().await;
        queues.insert(name.clone(), queue);

        info!("Created queue: {}", name);
        Ok(())
    }

    /// Delete a queue
    pub async fn delete_queue(&self, name: &str) -> Result<()> {
        let mut queues = self.queues.write().await;
        if let Some(queue) = queues.remove(name) {
            queue.delete().await?;
            info!("Deleted queue: {}", name);
            Ok(())
        } else {
            Err(Error::QueueNotFound(name.to_string()))
        }
    }

    /// Create a topic
    pub async fn create_topic(&self, name: String, config: TopicConfig) -> Result<()> {
        let topic = Arc::new(Topic::new(name.clone(), config, self.storage.clone()));
        topic.initialize().await?;

        let mut topics = self.topics.write().await;
        topics.insert(name.clone(), topic);

        info!("Created topic: {}", name);
        Ok(())
    }

    /// Delete a topic
    pub async fn delete_topic(&self, name: &str) -> Result<()> {
        let mut topics = self.topics.write().await;
        if let Some(topic) = topics.remove(name) {
            topic.delete().await?;
            info!("Deleted topic: {}", name);
            Ok(())
        } else {
            Err(Error::TopicNotFound(name.to_string()))
        }
    }

    /// Create a producer
    pub async fn create_producer(&self, config: ProducerConfig) -> Result<Arc<Producer>> {
        let producer = Arc::new(Producer::new(config, self.clone()));
        let producer_id = producer.id().clone();

        let mut producers = self.producers.write().await;
        producers.insert(producer_id.to_string(), producer.clone());

        info!("Created producer: {}", producer_id);
        Ok(producer)
    }

    /// Create a consumer
    pub async fn create_consumer(&self, config: ConsumerConfig) -> Result<Arc<Consumer>> {
        let consumer = Arc::new(Consumer::new(config, self.clone()));
        let consumer_id = consumer.id().clone();

        let mut consumers = self.consumers.write().await;
        consumers.insert(consumer_id.to_string(), consumer.clone());

        info!("Created consumer: {}", consumer_id);
        Ok(consumer)
    }

    /// Produce a message to a queue
    pub async fn produce_to_queue(&self, queue_name: &str, message: Message) -> Result<Offset> {
        let queues = self.queues.read().await;
        let queue = queues.get(queue_name)
            .ok_or_else(|| Error::QueueNotFound(queue_name.to_string()))?;

        let offset = queue.produce(message).await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_messages_produced += 1;
        }

        Ok(offset)
    }

    /// Produce a message to a topic
    pub async fn produce_to_topic(&self, topic_name: &str, message: Message) -> Result<Offset> {
        let topics = self.topics.read().await;
        let topic = topics.get(topic_name)
            .ok_or_else(|| Error::TopicNotFound(topic_name.to_string()))?;

        let offset = topic.produce(message).await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_messages_produced += 1;
        }

        Ok(offset)
    }

    /// Consume a message from a queue
    pub async fn consume_from_queue(&self, queue_name: &str, consumer_id: &str) -> Result<Option<Message>> {
        let queues = self.queues.read().await;
        let queue = queues.get(queue_name)
            .ok_or_else(|| Error::QueueNotFound(queue_name.to_string()))?;

        let message = queue.consume(consumer_id).await?;

        if message.is_some() {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_messages_consumed += 1;
        }

        Ok(message)
    }

    /// Consume a message from a topic
    pub async fn consume_from_topic(
        &self,
        topic_name: &str,
        partition: i32,
        consumer_group: &str,
        consumer_id: &str,
    ) -> Result<Option<Message>> {
        let topics = self.topics.read().await;
        let topic = topics.get(topic_name)
            .ok_or_else(|| Error::TopicNotFound(topic_name.to_string()))?;

        let message = topic.consume(partition, consumer_group, consumer_id).await?;

        if message.is_some() {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.total_messages_consumed += 1;
        }

        Ok(message)
    }

    /// Get broker statistics
    pub async fn get_stats(&self) -> BrokerStats {
        let stats = self.stats.read().await;
        let queues = self.queues.read().await;
        let topics = self.topics.read().await;
        let producers = self.producers.read().await;
        let consumers = self.consumers.read().await;
        let consumer_groups = self.consumer_groups.read().await;

        BrokerStats {
            total_messages_produced: stats.total_messages_produced,
            total_messages_consumed: stats.total_messages_consumed,
            total_bytes_produced: stats.total_bytes_produced,
            total_bytes_consumed: stats.total_bytes_consumed,
            active_producers: producers.len(),
            active_consumers: consumers.len(),
            active_queues: queues.len(),
            active_topics: topics.len(),
            consumer_groups: consumer_groups.len(),
            uptime: stats.uptime,
        }
    }

    /// Generate a unique message ID
    pub async fn generate_message_id(&self) -> u64 {
        let mut counter = self.message_id_counter.write().await;
        *counter += 1;
        *counter
    }

    /// Start background tasks
    async fn start_background_tasks(&mut self) {
        let (shutdown_tx, _) = broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx.clone());

        // Start cleanup task
        self.start_cleanup_task(shutdown_tx.subscribe()).await;

        // Start statistics task
        self.start_stats_task(shutdown_tx.subscribe()).await;
    }

    /// Start cleanup task
    async fn start_cleanup_task(&self, mut shutdown_rx: broadcast::Receiver<()>) {
        let config = self.config.clone();
        let queues = self.queues.clone();
        let topics = self.topics.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.cleanup_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Cleanup expired messages
                        let queues_guard = queues.read().await;
                        for queue in queues_guard.values() {
                            if let Err(e) = queue.cleanup_expired_messages().await {
                                warn!("Failed to cleanup queue {}: {}", queue.name(), e);
                            }
                        }

                        let topics_guard = topics.read().await;
                        for topic in topics_guard.values() {
                            if let Err(e) = topic.cleanup_expired_messages().await {
                                warn!("Failed to cleanup topic {}: {}", topic.name(), e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });
    }

    /// Start statistics task
    async fn start_stats_task(&self, mut shutdown_rx: broadcast::Receiver<()>) {
        let stats = self.stats.clone();
        let start_time = SystemTime::now();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Update every minute

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut stats_guard = stats.write().await;
                        stats_guard.uptime = start_time.elapsed().unwrap_or_default();
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });
    }
}

impl Clone for PrecursorBroker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            queues: self.queues.clone(),
            topics: self.topics.clone(),
            producers: self.producers.clone(),
            consumers: self.consumers.clone(),
            consumer_groups: self.consumer_groups.clone(),
            storage: self.storage.clone(),
            replication: self.replication.clone(),
            cluster: self.cluster.clone(),
            stats: self.stats.clone(),
            message_id_counter: self.message_id_counter.clone(),
            shutdown_tx: None, // Don't clone shutdown signal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_config_default() {
        let config = BrokerConfig::default();
        assert_eq!(config.broker_id, "broker-1");
        assert_eq!(config.data_dir, "./data");
        assert!(config.enable_persistence);
        assert!(!config.enable_replication);
        assert_eq!(config.replication_factor, 1);
    }

    #[test]
    fn test_message_creation() {
        let payload = Bytes::from("test message");
        let headers = HashMap::new();
        let message = Message::new(payload.clone(), headers);

        assert_eq!(message.payload, payload);
        assert_eq!(message.size(), payload.len());
        assert!(message.timestamp() <= SystemTime::now());
    }

    #[test]
    fn test_message_with_partition_key() {
        let payload = Bytes::from("test message");
        let headers = HashMap::new();
        let partition_key = "key1".to_string();
        let message = Message::with_partition_key(payload, headers, partition_key.clone());

        assert_eq!(message.partition_key(), Some(&partition_key));
    }

    #[test]
    fn test_offset_operations() {
        let offset = Offset::new(100);
        assert_eq!(offset.value(), 100);

        let incremented = offset.increment();
        assert_eq!(incremented.value(), 101);

        let decremented = offset.decrement();
        assert_eq!(decremented.value(), 99);
    }

    #[test]
    fn test_consumer_group_state() {
        assert_eq!(ConsumerGroupState::Empty, ConsumerGroupState::Empty);
        assert_ne!(ConsumerGroupState::Empty, ConsumerGroupState::Stable);
    }

    #[tokio::test]
    async fn test_broker_creation() {
        let config = BrokerConfig::default();
        let broker = PrecursorBroker::new(config);
        let stats = broker.get_stats().await;

        assert_eq!(stats.total_messages_produced, 0);
        assert_eq!(stats.total_messages_consumed, 0);
        assert_eq!(stats.active_producers, 0);
        assert_eq!(stats.active_consumers, 0);
        assert_eq!(stats.active_queues, 0);
        assert_eq!(stats.active_topics, 0);
    }
}
