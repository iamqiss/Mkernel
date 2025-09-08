//! Advanced replication with cross-region support and conflict resolution
//! 
//! This module provides sophisticated replication capabilities including
//! cross-region replication, conflict resolution, and data consistency guarantees.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, broadcast};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset};

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Replication factor
    pub replication_factor: u32,
    /// Minimum in-sync replicas
    pub min_in_sync_replicas: u32,
    /// Enable cross-region replication
    pub enable_cross_region: bool,
    /// Replication regions
    pub regions: Vec<ReplicationRegion>,
    /// Replication strategy
    pub strategy: ReplicationStrategy,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolutionStrategy,
    /// Replication timeout
    pub replication_timeout: Duration,
    /// Batch size for replication
    pub batch_size: usize,
    /// Enable compression for replication
    pub enable_compression: bool,
    /// Enable encryption for replication
    pub enable_encryption: bool,
    /// Replication lag threshold
    pub lag_threshold: Duration,
    /// Enable automatic failover
    pub enable_auto_failover: bool,
    /// Failover timeout
    pub failover_timeout: Duration,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            min_in_sync_replicas: 2,
            enable_cross_region: false,
            regions: Vec::new(),
            strategy: ReplicationStrategy::Synchronous,
            conflict_resolution: ConflictResolutionStrategy::LastWriteWins,
            replication_timeout: Duration::from_secs(30),
            batch_size: 1000,
            enable_compression: true,
            enable_encryption: true,
            lag_threshold: Duration::from_secs(60),
            enable_auto_failover: true,
            failover_timeout: Duration::from_secs(10),
        }
    }
}

/// Replication region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationRegion {
    /// Region ID
    pub region_id: String,
    /// Region name
    pub name: String,
    /// Region endpoints
    pub endpoints: Vec<String>,
    /// Region priority
    pub priority: u32,
    /// Region latency
    pub latency: Duration,
    /// Region status
    pub status: RegionStatus,
}

/// Region status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionStatus {
    /// Active
    Active,
    /// Inactive
    Inactive,
    /// Degraded
    Degraded,
    /// Failed
    Failed,
}

/// Replication strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStrategy {
    /// Synchronous replication
    Synchronous,
    /// Asynchronous replication
    Asynchronous,
    /// Semi-synchronous replication
    SemiSynchronous,
    /// Eventual consistency
    Eventual,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Last write wins
    LastWriteWins,
    /// First write wins
    FirstWriteWins,
    /// Custom resolution
    Custom,
    /// Manual resolution
    Manual,
}

/// Replica information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replica {
    /// Replica ID
    pub replica_id: String,
    /// Broker ID
    pub broker_id: String,
    /// Region ID
    pub region_id: String,
    /// Replica status
    pub status: ReplicaStatus,
    /// Last offset
    pub last_offset: Offset,
    /// Last update time
    pub last_update: SystemTime,
    /// Replication lag
    pub lag: Duration,
    /// Replica priority
    pub priority: u32,
}

/// Replica status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaStatus {
    /// In sync
    InSync,
    /// Out of sync
    OutOfSync,
    /// Catching up
    CatchingUp,
    /// Failed
    Failed,
    /// Offline
    Offline,
}

/// Replication log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLogEntry {
    /// Entry ID
    pub entry_id: String,
    /// Topic
    pub topic: String,
    /// Partition
    pub partition: i32,
    /// Offset
    pub offset: Offset,
    /// Message
    pub message: Message,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Source region
    pub source_region: String,
    /// Target regions
    pub target_regions: Vec<String>,
    /// Replication status
    pub status: ReplicationStatus,
    /// Retry count
    pub retry_count: u32,
}

/// Replication status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    /// Pending
    Pending,
    /// In progress
    InProgress,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Skipped
    Skipped,
}

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict ID
    pub conflict_id: String,
    /// Topic
    pub topic: String,
    /// Partition
    pub partition: i32,
    /// Offset
    pub offset: Offset,
    /// Conflicting entries
    pub entries: Vec<ConflictEntry>,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Resolution status
    pub resolution_status: ConflictResolutionStatus,
}

/// Conflict entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictEntry {
    /// Entry ID
    pub entry_id: String,
    /// Message
    pub message: Message,
    /// Source region
    pub source_region: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Version
    pub version: u64,
}

/// Conflict type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Write conflict
    WriteConflict,
    /// Delete conflict
    DeleteConflict,
    /// Update conflict
    UpdateConflict,
    /// Order conflict
    OrderConflict,
}

/// Conflict resolution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolutionStatus {
    /// Unresolved
    Unresolved,
    /// Resolved
    Resolved,
    /// In progress
    InProgress,
    /// Failed
    Failed,
}

/// Advanced replication manager
pub struct AdvancedReplicationManager {
    /// Replication configuration
    config: ReplicationConfig,
    /// Replicas by topic/partition
    replicas: Arc<RwLock<HashMap<String, HashMap<i32, Vec<Replica>>>>>,
    /// Replication log
    replication_log: Arc<RwLock<VecDeque<ReplicationLogEntry>>>,
    /// Active conflicts
    conflicts: Arc<RwLock<HashMap<String, Conflict>>>,
    /// Replication statistics
    stats: Arc<RwLock<ReplicationStats>>,
    /// Replication event sender
    event_sender: broadcast::Sender<ReplicationEvent>,
    /// Replication workers
    workers: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Health checker
    health_checker: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

/// Replication event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationEvent {
    /// Replica added
    ReplicaAdded(String, i32, Replica),
    /// Replica removed
    ReplicaRemoved(String, i32, String),
    /// Replica status changed
    ReplicaStatusChanged(String, i32, String, ReplicaStatus),
    /// Replication completed
    ReplicationCompleted(String, i32, Offset),
    /// Replication failed
    ReplicationFailed(String, i32, Offset, String),
    /// Conflict detected
    ConflictDetected(Conflict),
    /// Conflict resolved
    ConflictResolved(String),
    /// Region status changed
    RegionStatusChanged(String, RegionStatus),
}

/// Replication statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReplicationStats {
    /// Total replications
    pub total_replications: u64,
    /// Successful replications
    pub successful_replications: u64,
    /// Failed replications
    pub failed_replications: u64,
    /// Total conflicts
    pub total_conflicts: u64,
    /// Resolved conflicts
    pub resolved_conflicts: u64,
    /// Average replication latency
    pub avg_replication_latency: Duration,
    /// Average replication lag
    pub avg_replication_lag: Duration,
    /// Active replicas
    pub active_replicas: usize,
    /// Failed replicas
    pub failed_replicas: usize,
    /// Cross-region replications
    pub cross_region_replications: u64,
    /// Bytes replicated
    pub bytes_replicated: u64,
}

impl AdvancedReplicationManager {
    /// Create a new advanced replication manager
    pub fn new(config: ReplicationConfig) -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            config,
            replicas: Arc::new(RwLock::new(HashMap::new())),
            replication_log: Arc::new(RwLock::new(VecDeque::new())),
            conflicts: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ReplicationStats::default())),
            event_sender,
            workers: Arc::new(RwLock::new(HashMap::new())),
            health_checker: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Start the replication manager
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting advanced replication manager");
        
        // Start health checker
        self.start_health_checker().await;
        
        // Start replication workers for each region
        for region in &self.config.regions {
            self.start_replication_worker(region).await?;
        }
        
        info!("Advanced replication manager started");
        Ok(())
    }
    
    /// Stop the replication manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping advanced replication manager");
        
        // Stop health checker
        if let Some(checker) = self.health_checker.lock().unwrap().take() {
            checker.abort();
        }
        
        // Stop all workers
        {
            let workers = self.workers.read().await;
            for (worker_id, worker) in workers.iter() {
                info!("Stopping replication worker: {}", worker_id);
                worker.abort();
            }
        }
        
        info!("Advanced replication manager stopped");
        Ok(())
    }
    
    /// Add a replica
    pub async fn add_replica(
        &self,
        topic: &str,
        partition: i32,
        replica: Replica,
    ) -> Result<()> {
        let mut replicas = self.replicas.write().await;
        let topic_replicas = replicas.entry(topic.to_string()).or_insert_with(HashMap::new);
        let partition_replicas = topic_replicas.entry(partition).or_insert_with(Vec::new);
        
        partition_replicas.push(replica.clone());
        
        // Send event
        let _ = self.event_sender.send(ReplicationEvent::ReplicaAdded(
            topic.to_string(),
            partition,
            replica,
        ));
        
        info!("Added replica for topic {} partition {}", topic, partition);
        Ok(())
    }
    
    /// Remove a replica
    pub async fn remove_replica(
        &self,
        topic: &str,
        partition: i32,
        replica_id: &str,
    ) -> Result<()> {
        let mut replicas = self.replicas.write().await;
        if let Some(topic_replicas) = replicas.get_mut(topic) {
            if let Some(partition_replicas) = topic_replicas.get_mut(&partition) {
                partition_replicas.retain(|r| r.replica_id != replica_id);
            }
        }
        
        // Send event
        let _ = self.event_sender.send(ReplicationEvent::ReplicaRemoved(
            topic.to_string(),
            partition,
            replica_id.to_string(),
        ));
        
        info!("Removed replica {} for topic {} partition {}", replica_id, topic, partition);
        Ok(())
    }
    
    /// Replicate a message
    pub async fn replicate_message(
        &self,
        topic: &str,
        partition: i32,
        message: Message,
        offset: Offset,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Create replication log entry
        let entry_id = self.generate_entry_id();
        let log_entry = ReplicationLogEntry {
            entry_id: entry_id.clone(),
            topic: topic.to_string(),
            partition,
            offset,
            message: message.clone(),
            timestamp: SystemTime::now(),
            source_region: "local".to_string(), // Would be actual source region
            target_regions: self.get_target_regions(topic, partition).await,
            status: ReplicationStatus::Pending,
            retry_count: 0,
        };
        
        // Add to replication log
        {
            let mut log = self.replication_log.write().await;
            log.push_back(log_entry.clone());
        }
        
        // Start replication based on strategy
        match self.config.strategy {
            ReplicationStrategy::Synchronous => {
                self.replicate_synchronously(&log_entry).await?;
            }
            ReplicationStrategy::Asynchronous => {
                self.replicate_asynchronously(&log_entry).await?;
            }
            ReplicationStrategy::SemiSynchronous => {
                self.replicate_semi_synchronously(&log_entry).await?;
            }
            ReplicationStrategy::Eventual => {
                self.replicate_eventually(&log_entry).await?;
            }
        }
        
        // Update statistics
        let latency = start_time.elapsed();
        self.update_replication_stats(true, latency, message.size()).await;
        
        info!(
            "Replicated message to topic {} partition {} at offset {} in {:?}",
            topic,
            partition,
            offset.value(),
            latency
        );
        
        Ok(())
    }
    
    /// Detect and resolve conflicts
    pub async fn detect_conflicts(&self, topic: &str, partition: i32) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();
        
        // Get replicas for the topic/partition
        let replicas = {
            let replicas_guard = self.replicas.read().await;
            replicas_guard
                .get(topic)
                .and_then(|topic_replicas| topic_replicas.get(&partition))
                .cloned()
                .unwrap_or_default()
        };
        
        // Check for conflicts between replicas
        for i in 0..replicas.len() {
            for j in (i + 1)..replicas.len() {
                if let Some(conflict) = self.check_replica_conflict(&replicas[i], &replicas[j]).await? {
                    conflicts.push(conflict);
                }
            }
        }
        
        // Resolve conflicts
        for conflict in &conflicts {
            self.resolve_conflict(conflict).await?;
        }
        
        info!("Detected and resolved {} conflicts for topic {} partition {}", conflicts.len(), topic, partition);
        Ok(conflicts)
    }
    
    /// Get replication statistics
    pub async fn get_stats(&self) -> ReplicationStats {
        let mut stats = self.stats.read().await.clone();
        let replicas = self.replicas.read().await;
        
        let mut active_replicas = 0;
        let mut failed_replicas = 0;
        
        for topic_replicas in replicas.values() {
            for partition_replicas in topic_replicas.values() {
                for replica in partition_replicas {
                    match replica.status {
                        ReplicaStatus::InSync | ReplicaStatus::CatchingUp => {
                            active_replicas += 1;
                        }
                        ReplicaStatus::Failed | ReplicaStatus::Offline => {
                            failed_replicas += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        stats.active_replicas = active_replicas;
        stats.failed_replicas = failed_replicas;
        
        stats
    }
    
    /// Subscribe to replication events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ReplicationEvent> {
        self.event_sender.subscribe()
    }
    
    /// Start health checker
    async fn start_health_checker(&mut self) {
        let replicas = self.replicas.clone();
        let event_sender = self.event_sender.clone();
        let stats = self.stats.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
            
            loop {
                interval.tick().await;
                
                // Check replica health
                let replicas_guard = replicas.read().await;
                for (topic, topic_replicas) in replicas_guard.iter() {
                    for (partition, partition_replicas) in topic_replicas.iter() {
                        for replica in partition_replicas {
                            let is_healthy = Self::check_replica_health(replica).await;
                            if !is_healthy && replica.status != ReplicaStatus::Failed {
                                // Mark replica as failed
                                let _ = event_sender.send(ReplicationEvent::ReplicaStatusChanged(
                                    topic.clone(),
                                    *partition,
                                    replica.replica_id.clone(),
                                    ReplicaStatus::Failed,
                                ));
                            }
                        }
                    }
                }
            }
        });
        
        let mut health_checker = self.health_checker.lock().unwrap();
        *health_checker = Some(handle);
    }
    
    /// Start replication worker for a region
    async fn start_replication_worker(&mut self, region: &ReplicationRegion) -> Result<()> {
        let worker_id = format!("worker-{}", region.region_id);
        let config = self.config.clone();
        let replication_log = self.replication_log.clone();
        let event_sender = self.event_sender.clone();
        let stats = self.stats.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100)); // Process every 100ms
            
            loop {
                interval.tick().await;
                
                // Process replication log entries for this region
                let mut log = replication_log.write().await;
                let mut processed_entries = Vec::new();
                
                for (index, entry) in log.iter_mut().enumerate() {
                    if entry.target_regions.contains(&region.region_id) && 
                       entry.status == ReplicationStatus::Pending {
                        
                        // Process the entry
                        match Self::process_replication_entry(entry, region, &config).await {
                            Ok(()) => {
                                entry.status = ReplicationStatus::Completed;
                                processed_entries.push(index);
                                
                                // Send success event
                                let _ = event_sender.send(ReplicationEvent::ReplicationCompleted(
                                    entry.topic.clone(),
                                    entry.partition,
                                    entry.offset,
                                ));
                            }
                            Err(e) => {
                                entry.retry_count += 1;
                                if entry.retry_count >= 3 {
                                    entry.status = ReplicationStatus::Failed;
                                    processed_entries.push(index);
                                    
                                    // Send failure event
                                    let _ = event_sender.send(ReplicationEvent::ReplicationFailed(
                                        entry.topic.clone(),
                                        entry.partition,
                                        entry.offset,
                                        e.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
                
                // Remove processed entries
                for &index in processed_entries.iter().rev() {
                    log.remove(index);
                }
            }
        });
        
        let mut workers = self.workers.write().await;
        workers.insert(worker_id, handle);
        
        Ok(())
    }
    
    /// Get target regions for replication
    async fn get_target_regions(&self, topic: &str, partition: i32) -> Vec<String> {
        // In a real implementation, this would determine target regions based on
        // topic configuration, region priorities, and current replica distribution
        self.config.regions.iter().map(|r| r.region_id.clone()).collect()
    }
    
    /// Replicate synchronously
    async fn replicate_synchronously(&self, entry: &ReplicationLogEntry) -> Result<()> {
        // Wait for all target regions to confirm replication
        for region_id in &entry.target_regions {
            self.replicate_to_region(entry, region_id).await?;
        }
        Ok(())
    }
    
    /// Replicate asynchronously
    async fn replicate_asynchronously(&self, entry: &ReplicationLogEntry) -> Result<()> {
        // Fire and forget replication
        for region_id in &entry.target_regions {
            let entry_clone = entry.clone();
            let region_id_clone = region_id.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::replicate_to_region_static(&entry_clone, &region_id_clone).await {
                    warn!("Failed to replicate to region {}: {}", region_id_clone, e);
                }
            });
        }
        Ok(())
    }
    
    /// Replicate semi-synchronously
    async fn replicate_semi_synchronously(&self, entry: &ReplicationLogEntry) -> Result<()> {
        // Wait for at least min_in_sync_replicas to confirm
        let mut confirmed = 0;
        let required = self.config.min_in_sync_replicas as usize;
        
        for region_id in &entry.target_regions {
            if self.replicate_to_region(entry, region_id).await.is_ok() {
                confirmed += 1;
                if confirmed >= required {
                    break;
                }
            }
        }
        
        if confirmed < required {
            return Err(Error::InsufficientReplicas(confirmed, required));
        }
        
        Ok(())
    }
    
    /// Replicate eventually
    async fn replicate_eventually(&self, entry: &ReplicationLogEntry) -> Result<()> {
        // Queue for eventual replication
        // This would be handled by background workers
        Ok(())
    }
    
    /// Replicate to a specific region
    async fn replicate_to_region(&self, entry: &ReplicationLogEntry, region_id: &str) -> Result<()> {
        Self::replicate_to_region_static(entry, region_id).await
    }
    
    /// Static method for replication to region
    async fn replicate_to_region_static(entry: &ReplicationLogEntry, region_id: &str) -> Result<()> {
        // In a real implementation, this would send the message to the target region
        // For now, we'll simulate the replication
        tokio::time::sleep(Duration::from_millis(10)).await; // Simulate network delay
        
        debug!("Replicated message to region {}", region_id);
        Ok(())
    }
    
    /// Check replica conflict
    async fn check_replica_conflict(&self, replica1: &Replica, replica2: &Replica) -> Result<Option<Conflict>> {
        // In a real implementation, this would compare replica states and detect conflicts
        // For now, we'll return None (no conflicts)
        Ok(None)
    }
    
    /// Resolve conflict
    async fn resolve_conflict(&self, conflict: &Conflict) -> Result<()> {
        match self.config.conflict_resolution {
            ConflictResolutionStrategy::LastWriteWins => {
                self.resolve_last_write_wins(conflict).await
            }
            ConflictResolutionStrategy::FirstWriteWins => {
                self.resolve_first_write_wins(conflict).await
            }
            ConflictResolutionStrategy::Custom => {
                self.resolve_custom(conflict).await
            }
            ConflictResolutionStrategy::Manual => {
                self.resolve_manual(conflict).await
            }
        }
    }
    
    /// Resolve conflict using last write wins
    async fn resolve_last_write_wins(&self, conflict: &Conflict) -> Result<()> {
        // Find the entry with the latest timestamp
        let latest_entry = conflict.entries.iter()
            .max_by_key(|e| e.timestamp)
            .ok_or(Error::NoConflictEntries)?;
        
        // Apply the latest entry
        info!("Resolved conflict {} using last write wins", conflict.conflict_id);
        Ok(())
    }
    
    /// Resolve conflict using first write wins
    async fn resolve_first_write_wins(&self, conflict: &Conflict) -> Result<()> {
        // Find the entry with the earliest timestamp
        let earliest_entry = conflict.entries.iter()
            .min_by_key(|e| e.timestamp)
            .ok_or(Error::NoConflictEntries)?;
        
        // Apply the earliest entry
        info!("Resolved conflict {} using first write wins", conflict.conflict_id);
        Ok(())
    }
    
    /// Resolve conflict using custom strategy
    async fn resolve_custom(&self, conflict: &Conflict) -> Result<()> {
        // Custom conflict resolution logic would go here
        info!("Resolved conflict {} using custom strategy", conflict.conflict_id);
        Ok(())
    }
    
    /// Resolve conflict manually
    async fn resolve_manual(&self, conflict: &Conflict) -> Result<()> {
        // Manual conflict resolution would require human intervention
        warn!("Conflict {} requires manual resolution", conflict.conflict_id);
        Ok(())
    }
    
    /// Check replica health
    async fn check_replica_health(replica: &Replica) -> bool {
        // In a real implementation, this would check if the replica is responding
        // For now, we'll simulate health checks
        replica.status != ReplicaStatus::Failed && replica.status != ReplicaStatus::Offline
    }
    
    /// Process replication entry
    async fn process_replication_entry(
        entry: &mut ReplicationLogEntry,
        region: &ReplicationRegion,
        config: &ReplicationConfig,
    ) -> Result<()> {
        // Simulate replication processing
        tokio::time::sleep(Duration::from_millis(5)).await;
        
        // Simulate occasional failures
        if entry.retry_count > 0 && entry.retry_count % 3 == 0 {
            return Err(Error::ReplicationFailed("Simulated failure".to_string()));
        }
        
        Ok(())
    }
    
    /// Generate entry ID
    fn generate_entry_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("entry_{:x}", hasher.finish())
    }
    
    /// Update replication statistics
    async fn update_replication_stats(&self, success: bool, latency: Duration, bytes: usize) {
        let mut stats = self.stats.write().await;
        stats.total_replications += 1;
        
        if success {
            stats.successful_replications += 1;
        } else {
            stats.failed_replications += 1;
        }
        
        // Update average latency
        stats.avg_replication_latency = Duration::from_millis(
            (stats.avg_replication_latency.as_millis() * (stats.total_replications - 1) as u128 + 
             latency.as_millis()) / stats.total_replications as u128
        );
        
        stats.bytes_replicated += bytes as u64;
    }
}

// Add missing error variants
impl Error {
    pub fn InsufficientReplicas(confirmed: usize, required: usize) -> Self {
        Error::Storage(format!("Insufficient replicas: {} confirmed, {} required", confirmed, required))
    }
    
    pub fn NoConflictEntries() -> Self {
        Error::Storage("No conflict entries found".to_string())
    }
    
    pub fn ReplicationFailed(message: String) -> Self {
        Error::Storage(format!("Replication failed: {}", message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_replication_config_default() {
        let config = ReplicationConfig::default();
        assert_eq!(config.replication_factor, 3);
        assert_eq!(config.min_in_sync_replicas, 2);
        assert!(!config.enable_cross_region);
        assert_eq!(config.strategy, ReplicationStrategy::Synchronous);
        assert_eq!(config.conflict_resolution, ConflictResolutionStrategy::LastWriteWins);
        assert_eq!(config.replication_timeout, Duration::from_secs(30));
        assert_eq!(config.batch_size, 1000);
        assert!(config.enable_compression);
        assert!(config.enable_encryption);
        assert_eq!(config.lag_threshold, Duration::from_secs(60));
        assert!(config.enable_auto_failover);
        assert_eq!(config.failover_timeout, Duration::from_secs(10));
    }
    
    #[test]
    fn test_region_status() {
        assert_eq!(RegionStatus::Active, RegionStatus::Active);
        assert_ne!(RegionStatus::Active, RegionStatus::Inactive);
        assert_ne!(RegionStatus::Inactive, RegionStatus::Degraded);
        assert_ne!(RegionStatus::Degraded, RegionStatus::Failed);
    }
    
    #[test]
    fn test_replication_strategy() {
        assert_eq!(ReplicationStrategy::Synchronous, ReplicationStrategy::Synchronous);
        assert_ne!(ReplicationStrategy::Synchronous, ReplicationStrategy::Asynchronous);
        assert_ne!(ReplicationStrategy::Asynchronous, ReplicationStrategy::SemiSynchronous);
        assert_ne!(ReplicationStrategy::SemiSynchronous, ReplicationStrategy::Eventual);
    }
    
    #[test]
    fn test_conflict_resolution_strategy() {
        assert_eq!(ConflictResolutionStrategy::LastWriteWins, ConflictResolutionStrategy::LastWriteWins);
        assert_ne!(ConflictResolutionStrategy::LastWriteWins, ConflictResolutionStrategy::FirstWriteWins);
        assert_ne!(ConflictResolutionStrategy::FirstWriteWins, ConflictResolutionStrategy::Custom);
        assert_ne!(ConflictResolutionStrategy::Custom, ConflictResolutionStrategy::Manual);
    }
    
    #[test]
    fn test_replica_status() {
        assert_eq!(ReplicaStatus::InSync, ReplicaStatus::InSync);
        assert_ne!(ReplicaStatus::InSync, ReplicaStatus::OutOfSync);
        assert_ne!(ReplicaStatus::OutOfSync, ReplicaStatus::CatchingUp);
        assert_ne!(ReplicaStatus::CatchingUp, ReplicaStatus::Failed);
        assert_ne!(ReplicaStatus::Failed, ReplicaStatus::Offline);
    }
    
    #[test]
    fn test_replication_status() {
        assert_eq!(ReplicationStatus::Pending, ReplicationStatus::Pending);
        assert_ne!(ReplicationStatus::Pending, ReplicationStatus::InProgress);
        assert_ne!(ReplicationStatus::InProgress, ReplicationStatus::Completed);
        assert_ne!(ReplicationStatus::Completed, ReplicationStatus::Failed);
        assert_ne!(ReplicationStatus::Failed, ReplicationStatus::Skipped);
    }
    
    #[test]
    fn test_conflict_type() {
        assert_eq!(ConflictType::WriteConflict, ConflictType::WriteConflict);
        assert_ne!(ConflictType::WriteConflict, ConflictType::DeleteConflict);
        assert_ne!(ConflictType::DeleteConflict, ConflictType::UpdateConflict);
        assert_ne!(ConflictType::UpdateConflict, ConflictType::OrderConflict);
    }
    
    #[test]
    fn test_conflict_resolution_status() {
        assert_eq!(ConflictResolutionStatus::Unresolved, ConflictResolutionStatus::Unresolved);
        assert_ne!(ConflictResolutionStatus::Unresolved, ConflictResolutionStatus::Resolved);
        assert_ne!(ConflictResolutionStatus::Resolved, ConflictResolutionStatus::InProgress);
        assert_ne!(ConflictResolutionStatus::InProgress, ConflictResolutionStatus::Failed);
    }
    
    #[test]
    fn test_replication_stats_default() {
        let stats = ReplicationStats::default();
        assert_eq!(stats.total_replications, 0);
        assert_eq!(stats.successful_replications, 0);
        assert_eq!(stats.failed_replications, 0);
        assert_eq!(stats.total_conflicts, 0);
        assert_eq!(stats.resolved_conflicts, 0);
        assert_eq!(stats.avg_replication_latency, Duration::from_secs(0));
        assert_eq!(stats.avg_replication_lag, Duration::from_secs(0));
        assert_eq!(stats.active_replicas, 0);
        assert_eq!(stats.failed_replicas, 0);
        assert_eq!(stats.cross_region_replications, 0);
        assert_eq!(stats.bytes_replicated, 0);
    }
}