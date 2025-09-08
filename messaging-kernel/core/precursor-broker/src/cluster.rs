//! Cluster manager for Precursor broker

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, BrokerConfig};

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Cluster ID
    pub cluster_id: String,
    /// Broker ID
    pub broker_id: String,
    /// Broker address
    pub broker_address: String,
    /// Controller address
    pub controller_address: Option<String>,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Session timeout
    pub session_timeout: Duration,
    /// Rebalance timeout
    pub rebalance_timeout: Duration,
    /// Enable auto rebalance
    pub enable_auto_rebalance: bool,
    /// Min in-sync replicas
    pub min_in_sync_replicas: u32,
    /// Unclean leader election
    pub unclean_leader_election: bool,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_id: "default-cluster".to_string(),
            broker_id: "broker-1".to_string(),
            broker_address: "localhost:9092".to_string(),
            controller_address: None,
            heartbeat_interval: Duration::from_secs(3),
            session_timeout: Duration::from_secs(30),
            rebalance_timeout: Duration::from_secs(60),
            enable_auto_rebalance: true,
            min_in_sync_replicas: 1,
            unclean_leader_election: false,
        }
    }
}

/// Broker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrokerState {
    /// Broker is offline
    Offline,
    /// Broker is online
    Online,
    /// Broker is the controller
    Controller,
}

/// Broker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Broker {
    /// Broker ID
    pub broker_id: String,
    /// Broker address
    pub address: String,
    /// Broker state
    pub state: BrokerState,
    /// Last heartbeat
    pub last_heartbeat: SystemTime,
    /// Rack ID
    pub rack_id: Option<String>,
    /// Broker version
    pub version: String,
    /// Broker capabilities
    pub capabilities: Vec<String>,
}

/// Cluster metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetadata {
    /// Cluster ID
    pub cluster_id: String,
    /// Controller broker ID
    pub controller_broker_id: Option<String>,
    /// Brokers
    pub brokers: HashMap<String, Broker>,
    /// Topics
    pub topics: HashMap<String, TopicMetadata>,
    /// Last updated
    pub last_updated: SystemTime,
}

/// Topic metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicMetadata {
    /// Topic name
    pub topic_name: String,
    /// Partitions
    pub partitions: HashMap<i32, PartitionMetadata>,
    /// Replication factor
    pub replication_factor: u32,
    /// Min in-sync replicas
    pub min_in_sync_replicas: u32,
    /// Created timestamp
    pub created_at: SystemTime,
}

/// Partition metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    /// Topic name
    pub topic_name: String,
    /// Partition ID
    pub partition_id: i32,
    /// Leader broker ID
    pub leader: Option<String>,
    /// Replicas
    pub replicas: Vec<String>,
    /// In-sync replicas
    pub in_sync_replicas: Vec<String>,
    /// Offline replicas
    pub offline_replicas: Vec<String>,
}

/// Cluster manager
pub struct ClusterManager {
    /// Cluster configuration
    config: ClusterConfig,
    /// Cluster metadata
    metadata: Arc<RwLock<ClusterMetadata>>,
    /// Cluster statistics
    stats: Arc<RwLock<ClusterStats>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Cluster statistics
#[derive(Debug, Default)]
pub struct ClusterStats {
    /// Total brokers
    pub total_brokers: usize,
    /// Online brokers
    pub online_brokers: usize,
    /// Offline brokers
    pub offline_brokers: usize,
    /// Total topics
    pub total_topics: usize,
    /// Total partitions
    pub total_partitions: usize,
    /// Under-replicated partitions
    pub under_replicated_partitions: usize,
    /// Leaderless partitions
    pub leaderless_partitions: usize,
}

impl ClusterManager {
    /// Create a new cluster manager
    pub fn new(config: ClusterConfig) -> Self {
        let metadata = ClusterMetadata {
            cluster_id: config.cluster_id.clone(),
            controller_broker_id: None,
            brokers: HashMap::new(),
            topics: HashMap::new(),
            last_updated: SystemTime::now(),
        };
        
        Self {
            config,
            metadata: Arc::new(RwLock::new(metadata)),
            stats: Arc::new(RwLock::new(ClusterStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the cluster manager
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::Internal("Cluster manager is already running".to_string()));
        }
        
        *running = true;
        
        // Register this broker
        self.register_broker().await?;
        
        // Start cluster tasks
        self.start_cluster_tasks().await;
        
        info!("Cluster manager started");
        Ok(())
    }

    /// Stop the cluster manager
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(Error::Internal("Cluster manager is not running".to_string()));
        }
        
        *running = false;
        
        // Unregister this broker
        self.unregister_broker().await?;
        
        info!("Cluster manager stopped");
        Ok(())
    }

    /// Register a broker
    pub async fn register_broker(&self) -> Result<()> {
        let broker = Broker {
            broker_id: self.config.broker_id.clone(),
            address: self.config.broker_address.clone(),
            state: BrokerState::Online,
            last_heartbeat: SystemTime::now(),
            rack_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["neo-protocol".to_string()],
        };
        
        let mut metadata = self.metadata.write().await;
        metadata.brokers.insert(self.config.broker_id.clone(), broker);
        metadata.last_updated = SystemTime::now();
        
        info!("Registered broker: {}", self.config.broker_id);
        Ok(())
    }

    /// Unregister a broker
    pub async fn unregister_broker(&self) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        metadata.brokers.remove(&self.config.broker_id);
        metadata.last_updated = SystemTime::now();
        
        info!("Unregistered broker: {}", self.config.broker_id);
        Ok(())
    }

    /// Add a broker
    pub async fn add_broker(&self, broker: Broker) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        metadata.brokers.insert(broker.broker_id.clone(), broker.clone());
        metadata.last_updated = SystemTime::now();
        
        info!("Added broker: {}", broker.broker_id);
        Ok(())
    }

    /// Remove a broker
    pub async fn remove_broker(&self, broker_id: &str) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        metadata.brokers.remove(broker_id);
        metadata.last_updated = SystemTime::now();
        
        info!("Removed broker: {}", broker_id);
        Ok(())
    }

    /// Update broker heartbeat
    pub async fn update_broker_heartbeat(&self, broker_id: &str) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        if let Some(broker) = metadata.brokers.get_mut(broker_id) {
            broker.last_heartbeat = SystemTime::now();
            broker.state = BrokerState::Online;
        }
        
        debug!("Updated heartbeat for broker: {}", broker_id);
        Ok(())
    }

    /// Get broker information
    pub async fn get_broker(&self, broker_id: &str) -> Result<Option<Broker>> {
        let metadata = self.metadata.read().await;
        Ok(metadata.brokers.get(broker_id).cloned())
    }

    /// Get all brokers
    pub async fn get_brokers(&self) -> Vec<Broker> {
        let metadata = self.metadata.read().await;
        metadata.brokers.values().cloned().collect()
    }

    /// Add topic metadata
    pub async fn add_topic_metadata(&self, topic_metadata: TopicMetadata) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        metadata.topics.insert(topic_metadata.topic_name.clone(), topic_metadata.clone());
        metadata.last_updated = SystemTime::now();
        
        info!("Added topic metadata: {}", topic_metadata.topic_name);
        Ok(())
    }

    /// Remove topic metadata
    pub async fn remove_topic_metadata(&self, topic_name: &str) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        metadata.topics.remove(topic_name);
        metadata.last_updated = SystemTime::now();
        
        info!("Removed topic metadata: {}", topic_name);
        Ok(())
    }

    /// Update partition metadata
    pub async fn update_partition_metadata(
        &self,
        topic_name: &str,
        partition_id: i32,
        partition_metadata: PartitionMetadata,
    ) -> Result<()> {
        let mut metadata = self.metadata.write().await;
        if let Some(topic_metadata) = metadata.topics.get_mut(topic_name) {
            topic_metadata.partitions.insert(partition_id, partition_metadata);
            metadata.last_updated = SystemTime::now();
        }
        
        debug!(
            "Updated partition metadata: topic={}, partition={}",
            topic_name, partition_id
        );
        Ok(())
    }

    /// Get topic metadata
    pub async fn get_topic_metadata(&self, topic_name: &str) -> Result<Option<TopicMetadata>> {
        let metadata = self.metadata.read().await;
        Ok(metadata.topics.get(topic_name).cloned())
    }

    /// Get partition metadata
    pub async fn get_partition_metadata(
        &self,
        topic_name: &str,
        partition_id: i32,
    ) -> Result<Option<PartitionMetadata>> {
        let metadata = self.metadata.read().await;
        Ok(metadata.topics.get(topic_name)
            .and_then(|topic| topic.partitions.get(&partition_id).cloned()))
    }

    /// Get cluster metadata
    pub async fn get_cluster_metadata(&self) -> ClusterMetadata {
        let metadata = self.metadata.read().await;
        metadata.clone()
    }

    /// Get cluster statistics
    pub async fn get_stats(&self) -> ClusterStats {
        let _stats = self.stats.read().await;
        let metadata = self.metadata.read().await;
        
        let mut total_brokers = 0;
        let mut online_brokers = 0;
        let mut offline_brokers = 0;
        let mut total_topics = 0;
        let mut total_partitions = 0;
        let mut under_replicated_partitions = 0;
        let mut leaderless_partitions = 0;
        
        for broker in metadata.brokers.values() {
            total_brokers += 1;
            match broker.state {
                BrokerState::Online | BrokerState::Controller => online_brokers += 1,
                BrokerState::Offline => offline_brokers += 1,
            }
        }
        
        for topic_metadata in metadata.topics.values() {
            total_topics += 1;
            total_partitions += topic_metadata.partitions.len();
            
            for partition_metadata in topic_metadata.partitions.values() {
                if partition_metadata.in_sync_replicas.len() < topic_metadata.min_in_sync_replicas as usize {
                    under_replicated_partitions += 1;
                }
                
                if partition_metadata.leader.is_none() {
                    leaderless_partitions += 1;
                }
            }
        }
        
        ClusterStats {
            total_brokers,
            online_brokers,
            offline_brokers,
            total_topics,
            total_partitions,
            under_replicated_partitions,
            leaderless_partitions,
        }
    }

    /// Start cluster tasks
    async fn start_cluster_tasks(&self) {
        // Start broker health check task
        self.start_broker_health_check_task().await;
        
        // Start metadata update task
        self.start_metadata_update_task().await;
    }

    /// Start broker health check task
    async fn start_broker_health_check_task(&self) {
        let config = self.config.clone();
        let metadata = self.metadata.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.heartbeat_interval);
            
            loop {
                interval.tick().await;
                
                // Check if cluster manager is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Check broker health
                let mut metadata_guard = metadata.write().await;
                let now = SystemTime::now();
                
                for broker in metadata_guard.brokers.values_mut() {
                    let time_since_heartbeat = now.duration_since(broker.last_heartbeat)
                        .unwrap_or_default();
                    
                    if time_since_heartbeat > config.session_timeout {
                        if broker.state != BrokerState::Offline {
                            warn!(
                                "Broker {} is offline (no heartbeat for {:?})",
                                broker.broker_id,
                                time_since_heartbeat
                            );
                            broker.state = BrokerState::Offline;
                        }
                    }
                }
            }
        });
    }

    /// Start metadata update task
    async fn start_metadata_update_task(&self) {
        let metadata = self.metadata.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Check if cluster manager is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Update metadata timestamp
                {
                    let mut metadata_guard = metadata.write().await;
                    metadata_guard.last_updated = SystemTime::now();
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
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.cluster_id, "default-cluster");
        assert_eq!(config.broker_id, "broker-1");
        assert_eq!(config.broker_address, "localhost:9092");
        assert_eq!(config.heartbeat_interval, Duration::from_secs(3));
        assert_eq!(config.session_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_broker_state() {
        assert_eq!(BrokerState::Offline, BrokerState::Offline);
        assert_ne!(BrokerState::Offline, BrokerState::Online);
    }

    #[tokio::test]
    async fn test_cluster_manager_operations() {
        let config = ClusterConfig::default();
        let cluster_manager = ClusterManager::new(config);
        
        // Test start
        cluster_manager.start().await.unwrap();
        
        // Test get brokers
        let brokers = cluster_manager.get_brokers().await;
        assert_eq!(brokers.len(), 1);
        assert_eq!(brokers[0].broker_id, "broker-1");
        
        // Test add broker
        let broker = Broker {
            broker_id: "broker-2".to_string(),
            address: "localhost:9093".to_string(),
            state: BrokerState::Online,
            last_heartbeat: SystemTime::now(),
            rack_id: None,
            version: "1.0.0".to_string(),
            capabilities: vec!["neo-protocol".to_string()],
        };
        
        cluster_manager.add_broker(broker).await.unwrap();
        
        let brokers = cluster_manager.get_brokers().await;
        assert_eq!(brokers.len(), 2);
        
        // Test update heartbeat
        cluster_manager.update_broker_heartbeat("broker-2").await.unwrap();
        
        // Test get broker
        let broker = cluster_manager.get_broker("broker-2").await.unwrap();
        assert!(broker.is_some());
        assert_eq!(broker.unwrap().broker_id, "broker-2");
        
        // Test stats
        let stats = cluster_manager.get_stats().await;
        assert_eq!(stats.total_brokers, 2);
        assert_eq!(stats.online_brokers, 2);
        assert_eq!(stats.offline_brokers, 0);
        
        // Test stop
        cluster_manager.stop().await.unwrap();
    }
}