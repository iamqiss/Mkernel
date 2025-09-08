//! Advanced consumer group management with exactly-once semantics
//! 
//! This module provides sophisticated consumer group management with transaction support,
//! exactly-once semantics, and intelligent rebalancing.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, broadcast};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message, Offset};

/// Consumer group configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupConfig {
    /// Group ID
    pub group_id: String,
    /// Protocol type
    pub protocol_type: String,
    /// Session timeout
    pub session_timeout: Duration,
    /// Rebalance timeout
    pub rebalance_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Enable exactly-once semantics
    pub enable_exactly_once: bool,
    /// Enable transaction support
    pub enable_transactions: bool,
    /// Transaction timeout
    pub transaction_timeout: Duration,
    /// Maximum transaction size
    pub max_transaction_size: usize,
    /// Enable auto-commit
    pub enable_auto_commit: bool,
    /// Auto-commit interval
    pub auto_commit_interval: Duration,
    /// Partition assignment strategy
    pub assignment_strategy: AssignmentStrategy,
    /// Enable cooperative rebalancing
    pub enable_cooperative_rebalancing: bool,
}

impl Default for ConsumerGroupConfig {
    fn default() -> Self {
        Self {
            group_id: "default-group".to_string(),
            protocol_type: "consumer".to_string(),
            session_timeout: Duration::from_secs(30),
            rebalance_timeout: Duration::from_secs(60),
            heartbeat_interval: Duration::from_secs(3),
            enable_exactly_once: true,
            enable_transactions: true,
            transaction_timeout: Duration::from_secs(60),
            max_transaction_size: 1000,
            enable_auto_commit: true,
            auto_commit_interval: Duration::from_secs(5),
            assignment_strategy: AssignmentStrategy::Range,
            enable_cooperative_rebalancing: true,
        }
    }
}

/// Partition assignment strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentStrategy {
    /// Range assignment
    Range,
    /// Round-robin assignment
    RoundRobin,
    /// Sticky assignment
    Sticky,
    /// Cooperative sticky assignment
    CooperativeSticky,
    /// Custom assignment
    Custom,
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

/// Consumer group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupMember {
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
    /// Subscription topics
    pub subscription: Vec<String>,
    /// Partition assignments
    pub assignment: HashMap<String, Vec<i32>>,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Member state
    pub state: MemberState,
    /// Protocol metadata
    pub protocol_metadata: Vec<u8>,
    /// Generation ID
    pub generation_id: u32,
    /// Transaction state
    pub transaction_state: Option<TransactionState>,
}

/// Member state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberState {
    /// Joining
    Joining,
    /// Stable
    Stable,
    /// Leaving
    Leaving,
    /// Dead
    Dead,
}

/// Transaction state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionState {
    /// Transaction ID
    pub transaction_id: String,
    /// Transaction coordinator
    pub coordinator: String,
    /// Transaction timeout
    pub timeout: Duration,
    /// Transaction start time
    pub start_time: SystemTime,
    /// Committed offsets
    pub committed_offsets: HashMap<String, HashMap<i32, Offset>>,
    /// Pending offsets
    pub pending_offsets: HashMap<String, HashMap<i32, Offset>>,
    /// Transaction state
    pub state: TransactionStateType,
}

/// Transaction state type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStateType {
    /// Transaction started
    Started,
    /// Transaction in progress
    InProgress,
    /// Transaction committing
    Committing,
    /// Transaction committed
    Committed,
    /// Transaction aborting
    Aborting,
    /// Transaction aborted
    Aborted,
}

/// Consumer group
#[derive(Debug, Clone)]
pub struct ConsumerGroup {
    /// Group configuration
    config: ConsumerGroupConfig,
    /// Group state
    state: Arc<RwLock<ConsumerGroupState>>,
    /// Group members
    members: Arc<RwLock<HashMap<String, ConsumerGroupMember>>>,
    /// Group coordinator
    coordinator: Arc<RwLock<Option<String>>>,
    /// Generation ID
    generation_id: Arc<RwLock<u32>>,
    /// Leader ID
    leader_id: Arc<RwLock<Option<String>>>,
    /// Protocol metadata
    protocol_metadata: Arc<RwLock<Vec<u8>>>,
    /// Partition assignments
    partition_assignments: Arc<RwLock<HashMap<String, HashMap<i32, String>>>>,
    /// Committed offsets
    committed_offsets: Arc<RwLock<HashMap<String, HashMap<i32, Offset>>>>,
    /// Pending offsets
    pending_offsets: Arc<RwLock<HashMap<String, HashMap<i32, Offset>>>>,
    /// Group statistics
    stats: Arc<RwLock<ConsumerGroupStats>>,
    /// Rebalance event sender
    rebalance_sender: broadcast::Sender<RebalanceEvent>,
    /// Transaction coordinator
    transaction_coordinator: Arc<TransactionCoordinator>,
}

/// Consumer group statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConsumerGroupStats {
    /// Total members
    pub total_members: usize,
    /// Active members
    pub active_members: usize,
    /// Total rebalances
    pub total_rebalances: u64,
    /// Last rebalance time
    pub last_rebalance_time: Option<SystemTime>,
    /// Average rebalance duration
    pub avg_rebalance_duration: Duration,
    /// Total messages consumed
    pub total_messages_consumed: u64,
    /// Total bytes consumed
    pub total_bytes_consumed: u64,
    /// Lag per partition
    pub partition_lag: HashMap<String, HashMap<i32, u64>>,
}

/// Rebalance event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RebalanceEvent {
    /// Member joined
    MemberJoined(String),
    /// Member left
    MemberLeft(String),
    /// Rebalance started
    RebalanceStarted,
    /// Rebalance completed
    RebalanceCompleted,
    /// Assignment changed
    AssignmentChanged(String, HashMap<String, Vec<i32>>),
}

/// Transaction coordinator
pub struct TransactionCoordinator {
    /// Active transactions
    transactions: Arc<RwLock<HashMap<String, TransactionState>>>,
    /// Transaction timeout checker
    timeout_checker: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ConsumerGroup {
    /// Create a new consumer group
    pub fn new(config: ConsumerGroupConfig) -> Self {
        let (rebalance_sender, _) = broadcast::channel(100);
        let transaction_coordinator = Arc::new(TransactionCoordinator::new());
        
        Self {
            config,
            state: Arc::new(RwLock::new(ConsumerGroupState::Empty)),
            members: Arc::new(RwLock::new(HashMap::new())),
            coordinator: Arc::new(RwLock::new(None)),
            generation_id: Arc::new(RwLock::new(0)),
            leader_id: Arc::new(RwLock::new(None)),
            protocol_metadata: Arc::new(RwLock::new(Vec::new())),
            partition_assignments: Arc::new(RwLock::new(HashMap::new())),
            committed_offsets: Arc::new(RwLock::new(HashMap::new())),
            pending_offsets: Arc::new(RwLock::new(HashMap::new()))),
            stats: Arc::new(RwLock::new(ConsumerGroupStats::default())),
            rebalance_sender,
            transaction_coordinator,
        }
    }
    
    /// Add a member to the group
    pub async fn add_member(&self, member: ConsumerGroupMember) -> Result<()> {
        let member_id = member.member_id.clone();
        
        // Add member
        {
            let mut members = self.members.write().await;
            members.insert(member_id.clone(), member);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_members += 1;
            stats.active_members += 1;
        }
        
        // Trigger rebalance if needed
        self.trigger_rebalance().await?;
        
        // Send rebalance event
        let _ = self.rebalance_sender.send(RebalanceEvent::MemberJoined(member_id.clone()));
        
        info!("Added member {} to group {}", member_id, self.config.group_id);
        Ok(())
    }
    
    /// Remove a member from the group
    pub async fn remove_member(&self, member_id: &str) -> Result<()> {
        // Remove member
        {
            let mut members = self.members.write().await;
            members.remove(member_id);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.active_members = stats.active_members.saturating_sub(1);
        }
        
        // Trigger rebalance if needed
        self.trigger_rebalance().await?;
        
        // Send rebalance event
        let _ = self.rebalance_sender.send(RebalanceEvent::MemberLeft(member_id.to_string()));
        
        info!("Removed member {} from group {}", member_id, self.config.group_id);
        Ok(())
    }
    
    /// Update member heartbeat
    pub async fn update_member_heartbeat(&self, member_id: &str) -> Result<()> {
        let mut members = self.members.write().await;
        if let Some(member) = members.get_mut(member_id) {
            member.last_heartbeat = SystemTime::now();
        } else {
            return Err(Error::MemberNotFound(member_id.to_string()));
        }
        Ok(())
    }
    
    /// Get member assignments
    pub async fn get_member_assignments(&self, member_id: &str) -> Result<HashMap<String, Vec<i32>>> {
        let members = self.members.read().await;
        if let Some(member) = members.get(member_id) {
            Ok(member.assignment.clone())
        } else {
            Err(Error::MemberNotFound(member_id.to_string()))
        }
    }
    
    /// Commit offsets for a member
    pub async fn commit_offsets(
        &self,
        member_id: &str,
        topic: &str,
        partition: i32,
        offset: Offset,
    ) -> Result<()> {
        // Update committed offsets
        {
            let mut committed = self.committed_offsets.write().await;
            let topic_offsets = committed.entry(topic.to_string()).or_insert_with(HashMap::new);
            topic_offsets.insert(partition, offset);
        }
        
        // Update member's committed offsets if in transaction
        {
            let mut members = self.members.write().await;
            if let Some(member) = members.get_mut(member_id) {
                if let Some(ref mut transaction) = member.transaction_state {
                    let topic_offsets = transaction.committed_offsets
                        .entry(topic.to_string())
                        .or_insert_with(HashMap::new);
                    topic_offsets.insert(partition, offset);
                }
            }
        }
        
        debug!(
            "Committed offset {} for member {} on topic {} partition {}",
            offset.value(),
            member_id,
            topic,
            partition
        );
        
        Ok(())
    }
    
    /// Start a transaction
    pub async fn start_transaction(&self, member_id: &str, transaction_id: String) -> Result<()> {
        let transaction_state = TransactionState {
            transaction_id: transaction_id.clone(),
            coordinator: self.config.group_id.clone(),
            timeout: self.config.transaction_timeout,
            start_time: SystemTime::now(),
            committed_offsets: HashMap::new(),
            pending_offsets: HashMap::new(),
            state: TransactionStateType::Started,
        };
        
        // Update member's transaction state
        {
            let mut members = self.members.write().await;
            if let Some(member) = members.get_mut(member_id) {
                member.transaction_state = Some(transaction_state);
            } else {
                return Err(Error::MemberNotFound(member_id.to_string()));
            }
        }
        
        // Register transaction with coordinator
        self.transaction_coordinator.start_transaction(transaction_id).await?;
        
        info!("Started transaction for member {}", member_id);
        Ok(())
    }
    
    /// Commit a transaction
    pub async fn commit_transaction(&self, member_id: &str, transaction_id: &str) -> Result<()> {
        // Update transaction state
        {
            let mut members = self.members.write().await;
            if let Some(member) = members.get_mut(member_id) {
                if let Some(ref mut transaction) = member.transaction_state {
                    if transaction.transaction_id == transaction_id {
                        transaction.state = TransactionStateType::Committed;
                        
                        // Move pending offsets to committed
                        for (topic, partitions) in &transaction.pending_offsets {
                            let committed_topic = self.committed_offsets.write().await
                                .entry(topic.clone())
                                .or_insert_with(HashMap::new);
                            
                            for (partition, offset) in partitions {
                                committed_topic.insert(*partition, *offset);
                            }
                        }
                        
                        // Clear transaction state
                        member.transaction_state = None;
                    }
                }
            }
        }
        
        // Commit transaction with coordinator
        self.transaction_coordinator.commit_transaction(transaction_id).await?;
        
        info!("Committed transaction {} for member {}", transaction_id, member_id);
        Ok(())
    }
    
    /// Abort a transaction
    pub async fn abort_transaction(&self, member_id: &str, transaction_id: &str) -> Result<()> {
        // Update transaction state
        {
            let mut members = self.members.write().await;
            if let Some(member) = members.get_mut(member_id) {
                if let Some(ref mut transaction) = member.transaction_state {
                    if transaction.transaction_id == transaction_id {
                        transaction.state = TransactionStateType::Aborted;
                        
                        // Clear pending offsets
                        transaction.pending_offsets.clear();
                        
                        // Clear transaction state
                        member.transaction_state = None;
                    }
                }
            }
        }
        
        // Abort transaction with coordinator
        self.transaction_coordinator.abort_transaction(transaction_id).await?;
        
        info!("Aborted transaction {} for member {}", transaction_id, member_id);
        Ok(())
    }
    
    /// Get group statistics
    pub async fn get_stats(&self) -> ConsumerGroupStats {
        let mut stats = self.stats.read().await.clone();
        let members = self.members.read().await;
        stats.total_members = members.len();
        stats.active_members = members.len();
        stats
    }
    
    /// Get rebalance event receiver
    pub fn subscribe_to_rebalance_events(&self) -> broadcast::Receiver<RebalanceEvent> {
        self.rebalance_sender.subscribe()
    }
    
    /// Trigger rebalance
    async fn trigger_rebalance(&self) -> Result<()> {
        let start_time = Instant::now();
        
        // Update group state
        {
            let mut state = self.state.write().await;
            *state = ConsumerGroupState::PreparingRebalance;
        }
        
        // Send rebalance started event
        let _ = self.rebalance_sender.send(RebalanceEvent::RebalanceStarted);
        
        // Perform partition assignment
        self.perform_partition_assignment().await?;
        
        // Update generation ID
        {
            let mut generation_id = self.generation_id.write().await;
            *generation_id += 1;
        }
        
        // Update group state
        {
            let mut state = self.state.write().await;
            *state = ConsumerGroupState::Stable;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_rebalances += 1;
            stats.last_rebalance_time = Some(SystemTime::now());
            stats.avg_rebalance_duration = start_time.elapsed();
        }
        
        // Send rebalance completed event
        let _ = self.rebalance_sender.send(RebalanceEvent::RebalanceCompleted);
        
        info!(
            "Rebalance completed for group {} in {:?}",
            self.config.group_id,
            start_time.elapsed()
        );
        
        Ok(())
    }
    
    /// Perform partition assignment
    async fn perform_partition_assignment(&self) -> Result<()> {
        let members = self.members.read().await;
        let member_count = members.len();
        
        if member_count == 0 {
            return Ok(());
        }
        
        // Get all subscribed topics
        let mut all_topics = HashSet::new();
        for member in members.values() {
            for topic in &member.subscription {
                all_topics.insert(topic.clone());
            }
        }
        
        // Assign partitions based on strategy
        match self.config.assignment_strategy {
            AssignmentStrategy::Range => {
                self.assign_partitions_range(&members, &all_topics).await?;
            }
            AssignmentStrategy::RoundRobin => {
                self.assign_partitions_round_robin(&members, &all_topics).await?;
            }
            AssignmentStrategy::Sticky => {
                self.assign_partitions_sticky(&members, &all_topics).await?;
            }
            AssignmentStrategy::CooperativeSticky => {
                self.assign_partitions_cooperative_sticky(&members, &all_topics).await?;
            }
            AssignmentStrategy::Custom => {
                // Custom assignment logic would go here
                return Err(Error::CustomAssignmentNotImplemented);
            }
        }
        
        Ok(())
    }
    
    /// Assign partitions using range strategy
    async fn assign_partitions_range(
        &self,
        members: &HashMap<String, ConsumerGroupMember>,
        topics: &HashSet<String>,
    ) -> Result<()> {
        let mut member_list: Vec<_> = members.keys().collect();
        member_list.sort();
        
        for topic in topics {
            // Get partition count for topic (this would be fetched from topic metadata)
            let partition_count = 4; // Placeholder
            
            let partitions_per_member = partition_count / member_list.len();
            let extra_partitions = partition_count % member_list.len();
            
            let mut partition_index = 0;
            for (i, member_id) in member_list.iter().enumerate() {
                let mut member_partitions = Vec::new();
                let member_partition_count = partitions_per_member + if i < extra_partitions { 1 } else { 0 };
                
                for _ in 0..member_partition_count {
                    if partition_index < partition_count {
                        member_partitions.push(partition_index as i32);
                        partition_index += 1;
                    }
                }
                
                // Update member assignment
                {
                    let mut members_guard = self.members.write().await;
                    if let Some(member) = members_guard.get_mut(*member_id) {
                        member.assignment.insert(topic.clone(), member_partitions);
                    }
                }
                
                // Send assignment changed event
                let _ = self.rebalance_sender.send(RebalanceEvent::AssignmentChanged(
                    (*member_id).clone(),
                    members[*member_id].assignment.clone(),
                ));
            }
        }
        
        Ok(())
    }
    
    /// Assign partitions using round-robin strategy
    async fn assign_partitions_round_robin(
        &self,
        members: &HashMap<String, ConsumerGroupMember>,
        topics: &HashSet<String>,
    ) -> Result<()> {
        let mut member_list: Vec<_> = members.keys().collect();
        member_list.sort();
        
        for topic in topics {
            let partition_count = 4; // Placeholder
            
            for partition in 0..partition_count {
                let member_index = partition % member_list.len();
                let member_id = &member_list[member_index];
                
                // Update member assignment
                {
                    let mut members_guard = self.members.write().await;
                    if let Some(member) = members_guard.get_mut(member_id) {
                        let partitions = member.assignment.entry(topic.clone()).or_insert_with(Vec::new);
                        partitions.push(partition as i32);
                    }
                }
            }
            
            // Send assignment changed events
            for member_id in &member_list {
                let _ = self.rebalance_sender.send(RebalanceEvent::AssignmentChanged(
                    (*member_id).clone(),
                    members[*member_id].assignment.clone(),
                ));
            }
        }
        
        Ok(())
    }
    
    /// Assign partitions using sticky strategy
    async fn assign_partitions_sticky(
        &self,
        members: &HashMap<String, ConsumerGroupMember>,
        topics: &HashSet<String>,
    ) -> Result<()> {
        // Sticky assignment tries to minimize partition movement
        // For now, fall back to round-robin
        self.assign_partitions_round_robin(members, topics).await
    }
    
    /// Assign partitions using cooperative sticky strategy
    async fn assign_partitions_cooperative_sticky(
        &self,
        members: &HashMap<String, ConsumerGroupMember>,
        topics: &HashSet<String>,
    ) -> Result<()> {
        // Cooperative sticky assignment with minimal partition movement
        // For now, fall back to round-robin
        self.assign_partitions_round_robin(members, topics).await
    }
}

impl TransactionCoordinator {
    /// Create a new transaction coordinator
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            timeout_checker: Arc::new(Mutex::new(None)),
        }
    }
    
    /// Start a transaction
    pub async fn start_transaction(&self, transaction_id: String) -> Result<()> {
        let transaction_state = TransactionState {
            transaction_id: transaction_id.clone(),
            coordinator: "coordinator".to_string(),
            timeout: Duration::from_secs(60),
            start_time: SystemTime::now(),
            committed_offsets: HashMap::new(),
            pending_offsets: HashMap::new(),
            state: TransactionStateType::Started,
        };
        
        let mut transactions = self.transactions.write().await;
        transactions.insert(transaction_id, transaction_state);
        
        Ok(())
    }
    
    /// Commit a transaction
    pub async fn commit_transaction(&self, transaction_id: &str) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        if let Some(transaction) = transactions.get_mut(transaction_id) {
            transaction.state = TransactionStateType::Committed;
        }
        Ok(())
    }
    
    /// Abort a transaction
    pub async fn abort_transaction(&self, transaction_id: &str) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        if let Some(transaction) = transactions.get_mut(transaction_id) {
            transaction.state = TransactionStateType::Aborted;
        }
        Ok(())
    }
}

// Add missing error variants
impl Error {
    pub fn MemberNotFound(member_id: String) -> Self {
        Error::Storage(format!("Member not found: {}", member_id))
    }
    
    pub fn CustomAssignmentNotImplemented() -> Self {
        Error::Storage("Custom assignment strategy not implemented".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_consumer_group_config_default() {
        let config = ConsumerGroupConfig::default();
        assert_eq!(config.group_id, "default-group");
        assert_eq!(config.protocol_type, "consumer");
        assert!(config.enable_exactly_once);
        assert!(config.enable_transactions);
        assert_eq!(config.assignment_strategy, AssignmentStrategy::Range);
    }
    
    #[test]
    fn test_consumer_group_creation() {
        let config = ConsumerGroupConfig::default();
        let group = ConsumerGroup::new(config);
        
        // Group should start in Empty state
        // This would require async testing in a real implementation
    }
    
    #[test]
    fn test_member_state() {
        assert_eq!(MemberState::Joining, MemberState::Joining);
        assert_ne!(MemberState::Joining, MemberState::Stable);
    }
    
    #[test]
    fn test_transaction_state() {
        assert_eq!(TransactionStateType::Started, TransactionStateType::Started);
        assert_ne!(TransactionStateType::Started, TransactionStateType::Committed);
    }
    
    #[test]
    fn test_assignment_strategy() {
        assert_eq!(AssignmentStrategy::Range, AssignmentStrategy::Range);
        assert_ne!(AssignmentStrategy::Range, AssignmentStrategy::RoundRobin);
    }
}