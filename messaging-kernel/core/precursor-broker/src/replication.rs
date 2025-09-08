//! Replication manager for Precursor broker

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, BrokerConfig};

/// Replication manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Replication factor
    pub replication_factor: u32,
    /// Sync replication
    pub sync_replication: bool,
    /// Replication timeout
    pub replication_timeout: Duration,
    /// Min in-sync replicas
    pub min_in_sync_replicas: u32,
    /// Unclean leader election
    pub unclean_leader_election: bool,
    /// Replication lag threshold
    pub replication_lag_threshold: Duration,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 1,
            sync_replication: false,
            replication_timeout: Duration::from_secs(10),
            min_in_sync_replicas: 1,
            unclean_leader_election: false,
            replication_lag_threshold: Duration::from_secs(30),
        }
    }
}

/// Replica state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaState {
    /// Replica is offline
    Offline,
    /// Replica is online but not in sync
    Online,
    /// Replica is in sync
    InSync,
    /// Replica is the leader
    Leader,
}

/// Replica information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    /// Broker ID
    pub broker_id: String,
    /// Replica state
    pub state: ReplicaState,
    /// Last heartbeat
    pub last_heartbeat: SystemTime,
    /// Log end offset
    pub log_end_offset: u64,
    /// High watermark
    pub high_watermark: u64,
    /// Replication lag
    pub replication_lag: Duration,
}

/// Partition replica set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionReplicas {
    /// Topic name
    pub topic_name: String,
    /// Partition ID
    pub partition_id: i32,
    /// Replicas
    pub replicas: Vec<Replica>,
    /// Leader replica
    pub leader: Option<String>,
    /// In-sync replicas
    pub in_sync_replicas: Vec<String>,
}

/// Replication manager
pub struct ReplicationManager {
    /// Replication configuration
    config: ReplicationConfig,
    /// Partition replicas
    partition_replicas: Arc<RwLock<HashMap<String, PartitionReplicas>>>,
    /// Replication statistics
    stats: Arc<RwLock<ReplicationStats>>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

/// Replication statistics
#[derive(Debug, Default)]
pub struct ReplicationStats {
    /// Total partitions
    pub total_partitions: usize,
    /// Under-replicated partitions
    pub under_replicated_partitions: usize,
    /// Offline replicas
    pub offline_replicas: usize,
    /// In-sync replicas
    pub in_sync_replicas: usize,
    /// Average replication lag
    pub avg_replication_lag: Duration,
}

impl ReplicationManager {
    /// Create a new replication manager
    pub fn new(config: BrokerConfig) -> Self {
        let replication_config = ReplicationConfig {
            replication_factor: config.replication_factor,
            sync_replication: config.sync_replication,
            replication_timeout: Duration::from_secs(10),
            min_in_sync_replicas: config.replication_factor,
            unclean_leader_election: false,
            replication_lag_threshold: Duration::from_secs(30),
        };
        
        Self {
            config: replication_config,
            partition_replicas: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the replication manager
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::Internal("Replication manager is already running".to_string()));
        }
        
        *running = true;
        
        // Start replication tasks
        self.start_replication_tasks().await;
        
        info!("Replication manager started");
        Ok(())
    }

    /// Stop the replication manager
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Err(Error::Internal("Replication manager is not running".to_string()));
        }
        
        *running = false;
        
        info!("Replication manager stopped");
        Ok(())
    }

    /// Add partition replicas
    pub async fn add_partition_replicas(
        &self,
        topic_name: String,
        partition_id: i32,
        replicas: Vec<String>,
    ) -> Result<()> {
        let partition_key = format!("{}:{}", topic_name, partition_id);
        
        let partition_replicas = PartitionReplicas {
            topic_name: topic_name.clone(),
            partition_id,
            replicas: replicas.iter().map(|broker_id| Replica {
                broker_id: broker_id.clone(),
                state: ReplicaState::Online,
                last_heartbeat: SystemTime::now(),
                log_end_offset: 0,
                high_watermark: 0,
                replication_lag: Duration::from_secs(0),
            }).collect(),
            leader: replicas.first().cloned(),
            in_sync_replicas: replicas.clone(),
        };
        
        let mut partition_replicas_map = self.partition_replicas.write().await;
        partition_replicas_map.insert(partition_key, partition_replicas);
        
        info!(
            "Added partition replicas for topic {} partition {}: {:?}",
            topic_name, partition_id, replicas
        );
        
        Ok(())
    }

    /// Remove partition replicas
    pub async fn remove_partition_replicas(
        &self,
        topic_name: &str,
        partition_id: i32,
    ) -> Result<()> {
        let partition_key = format!("{}:{}", topic_name, partition_id);
        
        let mut partition_replicas_map = self.partition_replicas.write().await;
        partition_replicas_map.remove(&partition_key);
        
        info!(
            "Removed partition replicas for topic {} partition {}",
            topic_name, partition_id
        );
        
        Ok(())
    }

    /// Update replica state
    pub async fn update_replica_state(
        &self,
        topic_name: &str,
        partition_id: i32,
        broker_id: &str,
        state: ReplicaState,
    ) -> Result<()> {
        let partition_key = format!("{}:{}", topic_name, partition_id);
        
        let mut partition_replicas_map = self.partition_replicas.write().await;
        if let Some(partition_replicas) = partition_replicas_map.get_mut(&partition_key) {
            if let Some(replica) = partition_replicas.replicas.iter_mut()
                .find(|r| r.broker_id == broker_id) {
                replica.state = state;
                replica.last_heartbeat = SystemTime::now();
                
                // Update in-sync replicas
                partition_replicas.in_sync_replicas = partition_replicas.replicas.iter()
                    .filter(|r| r.state == ReplicaState::InSync || r.state == ReplicaState::Leader)
                    .map(|r| r.broker_id.clone())
                    .collect();
            }
        }
        
        debug!(
            "Updated replica state for topic {} partition {} broker {} to {:?}",
            topic_name, partition_id, broker_id, state
        );
        
        Ok(())
    }

    /// Update replica offset
    pub async fn update_replica_offset(
        &self,
        topic_name: &str,
        partition_id: i32,
        broker_id: &str,
        log_end_offset: u64,
        high_watermark: u64,
    ) -> Result<()> {
        let partition_key = format!("{}:{}", topic_name, partition_id);
        
        let mut partition_replicas_map = self.partition_replicas.write().await;
        if let Some(partition_replicas) = partition_replicas_map.get_mut(&partition_key) {
            // Get leader log end offset first to avoid borrow conflicts
            let leader_log_end_offset = if let Some(leader) = &partition_replicas.leader {
                partition_replicas.replicas.iter()
                    .find(|r| r.broker_id == *leader)
                    .map(|r| r.log_end_offset)
                    .unwrap_or(0)
            } else {
                0
            };
            
            if let Some(replica) = partition_replicas.replicas.iter_mut()
                .find(|r| r.broker_id == broker_id) {
                replica.log_end_offset = log_end_offset;
                replica.high_watermark = high_watermark;
                replica.last_heartbeat = SystemTime::now();
                
                // Calculate replication lag
                replica.replication_lag = Duration::from_secs(
                    (leader_log_end_offset - replica.log_end_offset) as u64
                );
            }
        }
        
        debug!(
            "Updated replica offset for topic {} partition {} broker {}: log_end_offset={}, high_watermark={}",
            topic_name, partition_id, broker_id, log_end_offset, high_watermark
        );
        
        Ok(())
    }

    /// Get partition replicas
    pub async fn get_partition_replicas(
        &self,
        topic_name: &str,
        partition_id: i32,
    ) -> Result<Option<PartitionReplicas>> {
        let partition_key = format!("{}:{}", topic_name, partition_id);
        let partition_replicas_map = self.partition_replicas.read().await;
        Ok(partition_replicas_map.get(&partition_key).cloned())
    }

    /// Check if partition is under-replicated
    pub async fn is_partition_under_replicated(
        &self,
        topic_name: &str,
        partition_id: i32,
    ) -> Result<bool> {
        let partition_replicas = self.get_partition_replicas(topic_name, partition_id).await?;
        
        Ok(partition_replicas.map_or(true, |pr| {
            pr.in_sync_replicas.len() < self.config.min_in_sync_replicas as usize
        }))
    }

    /// Get replication statistics
    pub async fn get_stats(&self) -> ReplicationStats {
        let _stats = self.stats.read().await;
        let partition_replicas = self.partition_replicas.read().await;
        
        let mut total_partitions = 0;
        let mut under_replicated_partitions = 0;
        let mut offline_replicas = 0;
        let mut in_sync_replicas = 0;
        let mut total_replication_lag = Duration::from_secs(0);
        let mut replica_count = 0;
        
        for partition_replicas in partition_replicas.values() {
            total_partitions += 1;
            
            if partition_replicas.in_sync_replicas.len() < self.config.min_in_sync_replicas as usize {
                under_replicated_partitions += 1;
            }
            
            for replica in &partition_replicas.replicas {
                match replica.state {
                    ReplicaState::Offline => offline_replicas += 1,
                    ReplicaState::InSync | ReplicaState::Leader => in_sync_replicas += 1,
                    _ => {}
                }
                
                total_replication_lag += replica.replication_lag;
                replica_count += 1;
            }
        }
        
        let avg_replication_lag = if replica_count > 0 {
            Duration::from_secs(total_replication_lag.as_secs() / replica_count as u64)
        } else {
            Duration::from_secs(0)
        };
        
        ReplicationStats {
            total_partitions,
            under_replicated_partitions,
            offline_replicas,
            in_sync_replicas,
            avg_replication_lag,
        }
    }

    /// Start replication tasks
    async fn start_replication_tasks(&self) {
        // Start replica health check task
        self.start_replica_health_check_task().await;
        
        // Start replication lag monitoring task
        self.start_replication_lag_monitoring_task().await;
    }

    /// Start replica health check task
    async fn start_replica_health_check_task(&self) {
        let config = self.config.clone();
        let partition_replicas = self.partition_replicas.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check if replication manager is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Check replica health
                let mut partition_replicas_map = partition_replicas.write().await;
                for (partition_key, partition_replicas) in partition_replicas_map.iter_mut() {
                    for replica in &mut partition_replicas.replicas {
                        let time_since_heartbeat = SystemTime::now()
                            .duration_since(replica.last_heartbeat)
                            .unwrap_or_default();
                        
                        if time_since_heartbeat > config.replication_timeout {
                            if replica.state != ReplicaState::Offline {
                                warn!(
                                    "Replica {} for partition {} is offline (no heartbeat for {:?})",
                                    replica.broker_id,
                                    partition_key,
                                    time_since_heartbeat
                                );
                                replica.state = ReplicaState::Offline;
                            }
                        }
                    }
                }
            }
        });
    }

    /// Start replication lag monitoring task
    async fn start_replication_lag_monitoring_task(&self) {
        let config = self.config.clone();
        let partition_replicas = self.partition_replicas.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Check if replication manager is still running
                {
                    let running_guard = running.read().await;
                    if !*running_guard {
                        break;
                    }
                }
                
                // Monitor replication lag
                let partition_replicas_map = partition_replicas.read().await;
                for (partition_key, partition_replicas) in partition_replicas_map.iter() {
                    for replica in &partition_replicas.replicas {
                        if replica.replication_lag > config.replication_lag_threshold {
                            warn!(
                                "Replica {} for partition {} has high replication lag: {:?}",
                                replica.broker_id,
                                partition_key,
                                replica.replication_lag
                            );
                        }
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
    fn test_replication_config_default() {
        let config = ReplicationConfig::default();
        assert_eq!(config.replication_factor, 1);
        assert!(!config.sync_replication);
        assert_eq!(config.replication_timeout, Duration::from_secs(10));
        assert_eq!(config.min_in_sync_replicas, 1);
        assert!(!config.unclean_leader_election);
    }

    #[test]
    fn test_replica_state() {
        assert_eq!(ReplicaState::Offline, ReplicaState::Offline);
        assert_ne!(ReplicaState::Offline, ReplicaState::Online);
    }

    #[tokio::test]
    async fn test_replication_manager_operations() {
        let broker_config = crate::BrokerConfig::default();
        let replication_manager = ReplicationManager::new(broker_config);
        
        // Test start
        replication_manager.start().await.unwrap();
        
        // Test add partition replicas
        replication_manager.add_partition_replicas(
            "test_topic".to_string(),
            0,
            vec!["broker-1".to_string(), "broker-2".to_string()],
        ).await.unwrap();
        
        // Test get partition replicas
        let partition_replicas = replication_manager.get_partition_replicas("test_topic", 0).await.unwrap();
        assert!(partition_replicas.is_some());
        assert_eq!(partition_replicas.unwrap().replicas.len(), 2);
        
        // Test update replica state
        replication_manager.update_replica_state(
            "test_topic",
            0,
            "broker-1",
            ReplicaState::InSync,
        ).await.unwrap();
        
        // Test is partition under-replicated
        let is_under_replicated = replication_manager.is_partition_under_replicated("test_topic", 0).await.unwrap();
        assert!(!is_under_replicated);
        
        // Test stats
        let stats = replication_manager.get_stats().await;
        assert_eq!(stats.total_partitions, 1);
        assert_eq!(stats.under_replicated_partitions, 0);
        
        // Test stop
        replication_manager.stop().await.unwrap();
    }
}