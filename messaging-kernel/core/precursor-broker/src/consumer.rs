//! Consumer implementation for Precursor broker

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset, PrecursorBroker};

/// Consumer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Consumer ID
    pub consumer_id: String,
    /// Consumer group
    pub consumer_group: String,
    /// Client ID
    pub client_id: String,
    /// Session timeout
    pub session_timeout: Duration,
    /// Rebalance timeout
    pub rebalance_timeout: Duration,
    /// Auto offset reset
    pub auto_offset_reset: AutoOffsetReset,
    /// Enable auto commit
    pub enable_auto_commit: bool,
    /// Auto commit interval
    pub auto_commit_interval: Duration,
    /// Max poll records
    pub max_poll_records: usize,
    /// Fetch min bytes
    pub fetch_min_bytes: usize,
    /// Fetch max wait
    pub fetch_max_wait: Duration,
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self {
            consumer_id: "consumer-1".to_string(),
            consumer_group: "default-group".to_string(),
            client_id: "client-1".to_string(),
            session_timeout: Duration::from_secs(30),
            rebalance_timeout: Duration::from_secs(60),
            auto_offset_reset: AutoOffsetReset::Latest,
            enable_auto_commit: true,
            auto_commit_interval: Duration::from_secs(5),
            max_poll_records: 500,
            fetch_min_bytes: 1,
            fetch_max_wait: Duration::from_millis(500),
        }
    }
}

/// Auto offset reset policy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AutoOffsetReset {
    /// Start from earliest available offset
    Earliest,
    /// Start from latest available offset
    Latest,
    /// Fail if no offset is available
    None,
}

/// Consumer
pub struct Consumer {
    /// Consumer configuration
    config: ConsumerConfig,
    /// Broker reference
    broker: PrecursorBroker,
    /// Subscribed topics
    subscriptions: Arc<RwLock<Vec<String>>>,
    /// Consumer statistics
    stats: Arc<RwLock<ConsumerStats>>,
    /// Message channel
    message_rx: Option<mpsc::Receiver<Message>>,
    /// Message channel sender
    message_tx: Option<mpsc::Sender<Message>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Consumer statistics
#[derive(Debug, Default)]
pub struct ConsumerStats {
    /// Total messages consumed
    pub total_messages: u64,
    /// Total bytes consumed
    pub total_bytes: u64,
    /// Last poll time
    pub last_poll_time: Option<SystemTime>,
    /// Last commit time
    pub last_commit_time: Option<SystemTime>,
    /// Current subscriptions
    pub subscriptions: Vec<String>,
}

impl Consumer {
    /// Create a new consumer
    pub fn new(config: ConsumerConfig, broker: PrecursorBroker) -> Self {
        let (message_tx, message_rx) = mpsc::channel(1000);
        
        Self {
            config,
            broker,
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ConsumerStats::default())),
            message_rx: Some(message_rx),
            message_tx: Some(message_tx),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get consumer ID
    pub fn id(&self) -> &str {
        &self.config.consumer_id
    }

    /// Get consumer group
    pub fn group(&self) -> &str {
        &self.config.consumer_group
    }

    /// Subscribe to topics
    pub async fn subscribe(&self, topics: Vec<String>) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.clear();
        subscriptions.extend(topics.clone());

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.subscriptions = topics.clone();
        }

        info!("Consumer {} subscribed to topics: {:?}", self.config.consumer_id, topics);
        Ok(())
    }

    /// Unsubscribe from all topics
    pub async fn unsubscribe(&self) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.clear();

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.subscriptions.clear();
        }

        info!("Consumer {} unsubscribed from all topics", self.config.consumer_id);
        Ok(())
    }

    /// Poll for messages
    pub async fn poll(&self, timeout: Option<Duration>) -> Result<Vec<Message>> {
        let _timeout = timeout.unwrap_or(self.config.fetch_max_wait);
        let mut messages = Vec::new();
        
        // Update last poll time
        {
            let mut stats = self.stats.write().await;
            stats.last_poll_time = Some(SystemTime::now());
        }

        // Poll from subscribed topics
        let subscriptions = self.subscriptions.read().await;
        for topic in subscriptions.iter() {
            // For simplicity, we'll poll from partition 0
            // In a real implementation, you'd handle multiple partitions
            if let Ok(Some(message)) = self.broker.consume_from_topic(
                topic,
                0,
                &self.config.consumer_group,
                &self.config.consumer_id,
            ).await {
                let message_size = message.size();
                messages.push(message);
                
                // Update statistics
                {
                    let mut stats = self.stats.write().await;
                    stats.total_messages += 1;
                    stats.total_bytes += message_size as u64;
                }
            }
        }

        debug!(
            "Consumer {} polled {} messages from {} topics",
            self.config.consumer_id,
            messages.len(),
            subscriptions.len()
        );

        Ok(messages)
    }

    /// Commit offsets
    pub async fn commit_offsets(&self) -> Result<()> {
        // In a real implementation, this would commit the current offsets
        // For now, we'll just update the last commit time
        
        let mut stats = self.stats.write().await;
        stats.last_commit_time = Some(SystemTime::now());
        
        debug!("Consumer {} committed offsets", self.config.consumer_id);
        Ok(())
    }

    /// Get consumer statistics
    pub async fn get_stats(&self) -> ConsumerStats {
        let stats = self.stats.read().await;
        let subscriptions = self.subscriptions.read().await;
        
        ConsumerStats {
            total_messages: stats.total_messages,
            total_bytes: stats.total_bytes,
            last_poll_time: stats.last_poll_time,
            last_commit_time: stats.last_commit_time,
            subscriptions: subscriptions.clone(),
        }
    }

    /// Start the consumer
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::Internal("Consumer is already running".to_string()));
        }
        
        *running = true;
        
        // Start auto-commit task if enabled
        if self.config.enable_auto_commit {
            self.start_auto_commit_task().await;
        }
        
        info!("Consumer {} started", self.config.consumer_id);
        Ok(())
    }

    /// Stop the consumer
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(Error::Internal("Consumer is not running".to_string()));
        }
        
        *running = false;
        
        info!("Consumer {} stopped", self.config.consumer_id);
        Ok(())
    }

    /// Start auto-commit task
    async fn start_auto_commit_task(&self) {
        let config = self.config.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.auto_commit_interval);
            
            loop {
                interval.tick().await;
                
                // Check if consumer is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Commit offsets
                // In a real implementation, this would commit the current offsets
                debug!("Auto-committing offsets for consumer {}", config.consumer_id);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_consumer_config_default() {
        let config = ConsumerConfig::default();
        assert_eq!(config.consumer_id, "consumer-1");
        assert_eq!(config.consumer_group, "default-group");
        assert_eq!(config.session_timeout, Duration::from_secs(30));
        assert_eq!(config.auto_offset_reset, AutoOffsetReset::Latest);
        assert!(config.enable_auto_commit);
    }

    #[tokio::test]
    async fn test_consumer_operations() {
        let config = ConsumerConfig::default();
        let broker = PrecursorBroker::new(crate::BrokerConfig::default());
        let consumer = Consumer::new(config, broker);
        
        // Test subscription
        consumer.subscribe(vec!["test_topic".to_string()]).await.unwrap();
        
        // Test stats
        let stats = consumer.get_stats().await;
        assert_eq!(stats.subscriptions.len(), 1);
        assert_eq!(stats.subscriptions[0], "test_topic");
        
        // Test unsubscription
        consumer.unsubscribe().await.unwrap();
        
        let stats = consumer.get_stats().await;
        assert!(stats.subscriptions.is_empty());
    }

    #[tokio::test]
    async fn test_consumer_lifecycle() {
        let config = ConsumerConfig::default();
        let broker = PrecursorBroker::new(crate::BrokerConfig::default());
        let consumer = Consumer::new(config, broker);
        
        // Test start
        consumer.start().await.unwrap();
        
        // Test stop
        consumer.stop().await.unwrap();
    }
}