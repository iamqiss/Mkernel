//! Producer implementation for Precursor broker

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset, PrecursorBroker};

/// Producer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducerConfig {
    /// Producer ID
    pub producer_id: String,
    /// Client ID
    pub client_id: String,
    /// Acks configuration
    pub acks: AcksConfig,
    /// Retries
    pub retries: u32,
    /// Retry backoff
    pub retry_backoff: Duration,
    /// Batch size
    pub batch_size: usize,
    /// Linger time
    pub linger: Duration,
    /// Buffer memory
    pub buffer_memory: usize,
    /// Compression type
    pub compression_type: Option<CompressionType>,
    /// Max request size
    pub max_request_size: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// Delivery timeout
    pub delivery_timeout: Duration,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            producer_id: "producer-1".to_string(),
            client_id: "client-1".to_string(),
            acks: AcksConfig::All,
            retries: 3,
            retry_backoff: Duration::from_millis(100),
            batch_size: 16384,
            linger: Duration::from_millis(0),
            buffer_memory: 32 * 1024 * 1024, // 32MB
            compression_type: None,
            max_request_size: 1024 * 1024, // 1MB
            request_timeout: Duration::from_secs(30),
            delivery_timeout: Duration::from_secs(120),
        }
    }
}

/// Acks configuration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AcksConfig {
    /// No acknowledgment required
    None,
    /// Acknowledgment from leader only
    Leader,
    /// Acknowledgment from all replicas
    All,
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

/// Producer
pub struct Producer {
    /// Producer configuration
    config: ProducerConfig,
    /// Broker reference
    broker: PrecursorBroker,
    /// Producer statistics
    stats: Arc<RwLock<ProducerStats>>,
    /// Message batch
    batch: Arc<RwLock<Vec<Message>>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Producer statistics
#[derive(Debug, Default)]
pub struct ProducerStats {
    /// Total messages sent
    pub total_messages: u64,
    /// Total bytes sent
    pub total_bytes: u64,
    /// Total errors
    pub total_errors: u64,
    /// Last send time
    pub last_send_time: Option<SystemTime>,
    /// Current batch size
    pub current_batch_size: usize,
}

impl Producer {
    /// Create a new producer
    pub fn new(config: ProducerConfig, broker: PrecursorBroker) -> Self {
        Self {
            config,
            broker,
            stats: Arc::new(RwLock::new(ProducerStats::default())),
            batch: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get producer ID
    pub fn id(&self) -> &str {
        &self.config.producer_id
    }

    /// Send a message to a queue
    pub async fn send_to_queue(&self, queue_name: &str, message: Message) -> Result<Offset> {
        let offset = self.broker.produce_to_queue(queue_name, message.clone()).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += 1;
            stats.total_bytes += message.size() as u64;
            stats.last_send_time = Some(SystemTime::now());
        }
        
        debug!(
            "Producer {} sent message to queue {} at offset {}",
            self.config.producer_id,
            queue_name,
            offset.value()
        );
        
        Ok(offset)
    }

    /// Send a message to a topic
    pub async fn send_to_topic(&self, topic_name: &str, message: Message) -> Result<Offset> {
        let offset = self.broker.produce_to_topic(topic_name, message.clone()).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_messages += 1;
            stats.total_bytes += message.size() as u64;
            stats.last_send_time = Some(SystemTime::now());
        }
        
        debug!(
            "Producer {} sent message to topic {} at offset {}",
            self.config.producer_id,
            topic_name,
            offset.value()
        );
        
        Ok(offset)
    }

    /// Send a batch of messages
    pub async fn send_batch(&self, messages: Vec<Message>) -> Result<Vec<Offset>> {
        let mut offsets = Vec::new();
        
        for message in messages {
            // For simplicity, we'll send each message individually
            // In a real implementation, you'd batch them together
            let offset = self.send_to_topic("default", message).await?;
            offsets.push(offset);
        }
        
        debug!(
            "Producer {} sent batch of {} messages",
            self.config.producer_id,
            offsets.len()
        );
        
        Ok(offsets)
    }

    /// Add message to batch
    pub async fn add_to_batch(&self, message: Message) -> Result<()> {
        let mut batch = self.batch.write().await;
        batch.push(message);
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.current_batch_size = batch.len();
        }
        
        // Send batch if it reaches the configured size
        if batch.len() >= self.config.batch_size {
            self.flush_batch().await?;
        }
        
        Ok(())
    }

    /// Flush the current batch
    pub async fn flush_batch(&self) -> Result<()> {
        let mut batch = self.batch.write().await;
        if batch.is_empty() {
            return Ok(());
        }
        
        let messages = batch.clone();
        let message_count = messages.len();
        batch.clear();
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.current_batch_size = 0;
        }
        
        // Send batch
        self.send_batch(messages).await?;
        
        debug!(
            "Producer {} flushed batch of {} messages",
            self.config.producer_id,
            message_count
        );
        
        Ok(())
    }

    /// Get producer statistics
    pub async fn get_stats(&self) -> ProducerStats {
        let stats = self.stats.read().await;
        let batch = self.batch.read().await;
        
        ProducerStats {
            total_messages: stats.total_messages,
            total_bytes: stats.total_bytes,
            total_errors: stats.total_errors,
            last_send_time: stats.last_send_time,
            current_batch_size: batch.len(),
        }
    }

    /// Start the producer
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::Internal("Producer is already running".to_string()));
        }
        
        *running = true;
        
        // Start batch flush task if linger is configured
        if self.config.linger > Duration::from_millis(0) {
            self.start_batch_flush_task().await;
        }
        
        info!("Producer {} started", self.config.producer_id);
        Ok(())
    }

    /// Stop the producer
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(Error::Internal("Producer is not running".to_string()));
        }
        
        *running = false;
        
        // Flush any remaining messages in the batch
        self.flush_batch().await?;
        
        info!("Producer {} stopped", self.config.producer_id);
        Ok(())
    }

    /// Start batch flush task
    async fn start_batch_flush_task(&self) {
        let config = self.config.clone();
        let running = self.running.clone();
        let batch = self.batch.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.linger);
            
            loop {
                interval.tick().await;
                
                // Check if producer is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Flush batch if it has messages
                {
                    let batch_guard = batch.read().await;
                    if !batch_guard.is_empty() {
                        drop(batch_guard);
                        // In a real implementation, this would flush the batch
                        debug!("Flushing batch for producer {}", config.producer_id);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_producer_config_default() {
        let config = ProducerConfig::default();
        assert_eq!(config.producer_id, "producer-1");
        assert_eq!(config.acks, AcksConfig::All);
        assert_eq!(config.retries, 3);
        assert_eq!(config.batch_size, 16384);
        assert_eq!(config.buffer_memory, 32 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_producer_operations() {
        let config = ProducerConfig::default();
        let broker = PrecursorBroker::new(crate::BrokerConfig::default());
        let producer = Producer::new(config, broker);
        
        // Test stats
        let stats = producer.get_stats().await;
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.current_batch_size, 0);
    }

    #[tokio::test]
    async fn test_producer_lifecycle() {
        let config = ProducerConfig::default();
        let broker = PrecursorBroker::new(crate::BrokerConfig::default());
        let producer = Producer::new(config, broker);
        
        // Test start
        producer.start().await.unwrap();
        
        // Test stop
        producer.stop().await.unwrap();
    }
}