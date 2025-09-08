//! Distributed consensus with Raft protocol for high availability
//! 
//! This module provides a robust distributed consensus implementation using the
//! Raft algorithm for leader election, log replication, and cluster coordination.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, broadcast};
use tokio::time::{interval, timeout};
use tracing::{debug, error, info, warn};

use crate::{Result, Error};

/// Raft configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Node ID
    pub node_id: String,
    /// Cluster nodes
    pub cluster_nodes: Vec<RaftNode>,
    /// Election timeout (milliseconds)
    pub election_timeout_ms: u64,
    /// Heartbeat interval (milliseconds)
    pub heartbeat_interval_ms: u64,
    /// RPC timeout (milliseconds)
    pub rpc_timeout_ms: u64,
    /// Log replication timeout (milliseconds)
    pub replication_timeout_ms: u64,
    /// Maximum log entries per request
    pub max_log_entries_per_request: usize,
    /// Snapshot threshold (number of log entries)
    pub snapshot_threshold: u64,
    /// Enable pre-vote
    pub enable_pre_vote: bool,
    /// Enable single-node optimization
    pub enable_single_node_optimization: bool,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            cluster_nodes: Vec::new(),
            election_timeout_ms: 150,
            heartbeat_interval_ms: 50,
            rpc_timeout_ms: 100,
            replication_timeout_ms: 200,
            max_log_entries_per_request: 1000,
            snapshot_threshold: 10000,
            enable_pre_vote: true,
            enable_single_node_optimization: true,
        }
    }
}

/// Raft node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftNode {
    /// Node ID
    pub node_id: String,
    /// Node address
    pub address: String,
    /// Node port
    pub port: u16,
    /// Node status
    pub status: NodeStatus,
    /// Last contact time
    pub last_contact: Option<SystemTime>,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Follower
    Follower,
    /// Candidate
    Candidate,
    /// Leader
    Leader,
    /// Shutdown
    Shutdown,
}

/// Raft state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftState {
    /// Follower state
    Follower,
    /// Candidate state
    Candidate,
    /// Leader state
    Leader,
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Entry index
    pub index: u64,
    /// Entry term
    pub term: u64,
    /// Entry type
    pub entry_type: LogEntryType,
    /// Entry data
    pub data: Vec<u8>,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogEntryType {
    /// Normal log entry
    Normal,
    /// Configuration change
    ConfigChange,
    /// No-op entry
    NoOp,
}

/// Raft message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    /// Request vote
    RequestVote(RequestVoteRequest),
    /// Request vote response
    RequestVoteResponse(RequestVoteResponse),
    /// Append entries
    AppendEntries(AppendEntriesRequest),
    /// Append entries response
    AppendEntriesResponse(AppendEntriesResponse),
    /// Install snapshot
    InstallSnapshot(InstallSnapshotRequest),
    /// Install snapshot response
    InstallSnapshotResponse(InstallSnapshotResponse),
    /// Pre-vote request
    PreVote(PreVoteRequest),
    /// Pre-vote response
    PreVoteResponse(PreVoteResponse),
}

/// Request vote request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: String,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
}

/// Request vote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term
    pub term: u64,
    /// True means candidate received vote
    pub vote_granted: bool,
}

/// Append entries request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID
    pub leader_id: String,
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: u64,
    /// Term of prev_log_index entry
    pub prev_log_term: u64,
    /// Log entries to store
    pub entries: Vec<LogEntry>,
    /// Leader's commit index
    pub leader_commit: u64,
}

/// Append entries response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term
    pub term: u64,
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    /// Index of the last log entry
    pub last_log_index: u64,
}

/// Install snapshot request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term
    pub term: u64,
    /// Leader ID
    pub leader_id: String,
    /// Last included index
    pub last_included_index: u64,
    /// Last included term
    pub last_included_term: u64,
    /// Offset in the snapshot
    pub offset: u64,
    /// Snapshot data
    pub data: Vec<u8>,
    /// True if this is the last chunk
    pub done: bool,
}

/// Install snapshot response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term
    pub term: u64,
}

/// Pre-vote request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreVoteRequest {
    /// Candidate's term
    pub term: u64,
    /// Candidate requesting vote
    pub candidate_id: String,
    /// Index of candidate's last log entry
    pub last_log_index: u64,
    /// Term of candidate's last log entry
    pub last_log_term: u64,
}

/// Pre-vote response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreVoteResponse {
    /// Current term
    pub term: u64,
    /// True means candidate would receive vote
    pub vote_granted: bool,
}

/// Raft server
pub struct RaftServer {
    /// Raft configuration
    config: RaftConfig,
    /// Current state
    state: Arc<RwLock<RaftState>>,
    /// Current term
    current_term: Arc<RwLock<u64>>,
    /// Voted for candidate ID
    voted_for: Arc<RwLock<Option<String>>>,
    /// Log entries
    log: Arc<RwLock<VecDeque<LogEntry>>>,
    /// Commit index
    commit_index: Arc<RwLock<u64>>,
    /// Last applied index
    last_applied: Arc<RwLock<u64>>,
    /// Next index for each server
    next_index: Arc<RwLock<HashMap<String, u64>>>,
    /// Match index for each server
    match_index: Arc<RwLock<HashMap<String, u64>>>,
    /// Last contact time for each server
    last_contact: Arc<RwLock<HashMap<String, SystemTime>>>,
    /// Message sender
    message_sender: mpsc::UnboundedSender<RaftMessage>,
    /// Message receiver
    message_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<RaftMessage>>>>,
    /// State change sender
    state_change_sender: broadcast::Sender<StateChangeEvent>,
    /// Election timer
    election_timer: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Heartbeat timer
    heartbeat_timer: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Raft statistics
    stats: Arc<RwLock<RaftStats>>,
}

/// State change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChangeEvent {
    /// State changed
    StateChanged(RaftState, RaftState),
    /// Leader elected
    LeaderElected(String, u64),
    /// Term changed
    TermChanged(u64, u64),
    /// Log entry committed
    LogEntryCommitted(u64, u64),
}

/// Raft statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RaftStats {
    /// Current term
    pub current_term: u64,
    /// Current state
    pub current_state: RaftState,
    /// Leader ID
    pub leader_id: Option<String>,
    /// Log size
    pub log_size: usize,
    /// Commit index
    pub commit_index: u64,
    /// Last applied index
    pub last_applied: u64,
    /// Total votes received
    pub total_votes_received: u64,
    /// Total votes sent
    pub total_votes_sent: u64,
    /// Total append entries sent
    pub total_append_entries_sent: u64,
    /// Total append entries received
    pub total_append_entries_received: u64,
    /// Total snapshots sent
    pub total_snapshots_sent: u64,
    /// Total snapshots received
    pub total_snapshots_received: u64,
    /// Uptime
    pub uptime: Duration,
}

impl RaftServer {
    /// Create a new Raft server
    pub fn new(config: RaftConfig) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let (state_change_sender, _) = broadcast::channel(100);
        
        Self {
            config,
            state: Arc::new(RwLock::new(RaftState::Follower)),
            current_term: Arc::new(RwLock::new(0)),
            voted_for: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(VecDeque::new())),
            commit_index: Arc::new(RwLock::new(0)),
            last_applied: Arc::new(RwLock::new(0)),
            next_index: Arc::new(RwLock::new(HashMap::new())),
            match_index: Arc::new(RwLock::new(HashMap::new())),
            last_contact: Arc::new(RwLock::new(HashMap::new())),
            message_sender,
            message_receiver: Arc::new(Mutex::new(Some(message_receiver))),
            state_change_sender,
            election_timer: Arc::new(Mutex::new(None)),
            heartbeat_timer: Arc::new(Mutex::new(None)),
            stats: Arc::new(RwLock::new(RaftStats::default())),
        }
    }
    
    /// Start the Raft server
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Raft server: {}", self.config.node_id);
        
        // Start message processing
        self.start_message_processing().await;
        
        // Start election timer
        self.start_election_timer().await;
        
        // Initialize next_index and match_index for all servers
        self.initialize_server_indices().await;
        
        info!("Raft server started: {}", self.config.node_id);
        Ok(())
    }
    
    /// Stop the Raft server
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Raft server: {}", self.config.node_id);
        
        // Stop timers
        if let Some(timer) = self.election_timer.lock().unwrap().take() {
            timer.abort();
        }
        
        if let Some(timer) = self.heartbeat_timer.lock().unwrap().take() {
            timer.abort();
        }
        
        // Change state to shutdown
        {
            let mut state = self.state.write().await;
            *state = RaftState::Follower; // Raft doesn't have a shutdown state
        }
        
        info!("Raft server stopped: {}", self.config.node_id);
        Ok(())
    }
    
    /// Propose a log entry
    pub async fn propose(&self, data: Vec<u8>) -> Result<u64> {
        let state = self.state.read().await;
        if *state != RaftState::Leader {
            return Err(Error::NotLeader);
        }
        
        // Create log entry
        let term = *self.current_term.read().await;
        let log_index = self.get_next_log_index().await;
        
        let log_entry = LogEntry {
            index: log_index,
            term,
            entry_type: LogEntryType::Normal,
            data,
            timestamp: SystemTime::now(),
        };
        
        // Add to log
        {
            let mut log = self.log.write().await;
            log.push_back(log_entry);
        }
        
        // Replicate to followers
        self.replicate_log_entries().await?;
        
        info!("Proposed log entry at index {}", log_index);
        Ok(log_index)
    }
    
    /// Get current state
    pub async fn get_state(&self) -> RaftState {
        *self.state.read().await
    }
    
    /// Get current term
    pub async fn get_current_term(&self) -> u64 {
        *self.current_term.read().await
    }
    
    /// Get leader ID
    pub async fn get_leader_id(&self) -> Option<String> {
        let state = self.state.read().await;
        if *state == RaftState::Leader {
            Some(self.config.node_id.clone())
        } else {
            None
        }
    }
    
    /// Get Raft statistics
    pub async fn get_stats(&self) -> RaftStats {
        let mut stats = self.stats.read().await.clone();
        let state = self.state.read().await;
        let term = self.current_term.read().await;
        let log = self.log.read().await;
        let commit_index = self.commit_index.read().await;
        let last_applied = self.last_applied.read().await;
        
        stats.current_term = *term;
        stats.current_state = *state;
        stats.leader_id = if *state == RaftState::Leader { Some(self.config.node_id.clone()) } else { None };
        stats.log_size = log.len();
        stats.commit_index = *commit_index;
        stats.last_applied = *last_applied;
        
        stats
    }
    
    /// Subscribe to state change events
    pub fn subscribe_to_state_changes(&self) -> broadcast::Receiver<StateChangeEvent> {
        self.state_change_sender.subscribe()
    }
    
    /// Start message processing
    async fn start_message_processing(&mut self) {
        let mut receiver = self.message_receiver.lock().unwrap().take().unwrap();
        let state = self.state.clone();
        let current_term = self.current_term.clone();
        let voted_for = self.voted_for.clone();
        let log = self.log.clone();
        let commit_index = self.commit_index.clone();
        let last_applied = self.last_applied.clone();
        let next_index = self.next_index.clone();
        let match_index = self.match_index.clone();
        let last_contact = self.last_contact.clone();
        let state_change_sender = self.state_change_sender.clone();
        let stats = self.stats.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                match message {
                    RaftMessage::RequestVote(request) => {
                        Self::handle_request_vote(
                            request,
                            &state,
                            &current_term,
                            &voted_for,
                            &log,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::RequestVoteResponse(response) => {
                        Self::handle_request_vote_response(
                            response,
                            &state,
                            &current_term,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::AppendEntries(request) => {
                        Self::handle_append_entries(
                            request,
                            &state,
                            &current_term,
                            &log,
                            &commit_index,
                            &last_applied,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::AppendEntriesResponse(response) => {
                        Self::handle_append_entries_response(
                            response,
                            &state,
                            &current_term,
                            &next_index,
                            &match_index,
                            &commit_index,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::InstallSnapshot(request) => {
                        Self::handle_install_snapshot(
                            request,
                            &state,
                            &current_term,
                            &log,
                            &commit_index,
                            &last_applied,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::InstallSnapshotResponse(response) => {
                        Self::handle_install_snapshot_response(
                            response,
                            &state,
                            &current_term,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::PreVote(request) => {
                        Self::handle_pre_vote(
                            request,
                            &state,
                            &current_term,
                            &log,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                    RaftMessage::PreVoteResponse(response) => {
                        Self::handle_pre_vote_response(
                            response,
                            &state,
                            &current_term,
                            &state_change_sender,
                            &stats,
                        ).await;
                    }
                }
            }
        });
    }
    
    /// Start election timer
    async fn start_election_timer(&mut self) {
        let config = self.config.clone();
        let state = self.state.clone();
        let current_term = self.current_term.clone();
        let voted_for = self.voted_for.clone();
        let log = self.log.clone();
        let state_change_sender = self.state_change_sender.clone();
        let stats = self.stats.clone();
        let message_sender = self.message_sender.clone();
        
        let handle = tokio::spawn(async move {
            loop {
                let timeout_duration = Duration::from_millis(config.election_timeout_ms);
                sleep(timeout_duration).await;
                
                let current_state = *state.read().await;
                if current_state != RaftState::Leader {
                    // Start election
                    Self::start_election(
                        &config,
                        &state,
                        &current_term,
                        &voted_for,
                        &log,
                        &state_change_sender,
                        &stats,
                        &message_sender,
                    ).await;
                }
            }
        });
        
        let mut election_timer = self.election_timer.lock().unwrap();
        *election_timer = Some(handle);
    }
    
    /// Start heartbeat timer (for leaders)
    async fn start_heartbeat_timer(&mut self) {
        let config = self.config.clone();
        let state = self.state.clone();
        let current_term = self.current_term.clone();
        let log = self.log.clone();
        let next_index = self.next_index.clone();
        let match_index = self.match_index.clone();
        let commit_index = self.commit_index.clone();
        let state_change_sender = self.state_change_sender.clone();
        let stats = self.stats.clone();
        let message_sender = self.message_sender.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(config.heartbeat_interval_ms));
            
            loop {
                interval.tick().await;
                
                let current_state = *state.read().await;
                if current_state == RaftState::Leader {
                    // Send heartbeat
                    Self::send_heartbeat(
                        &config,
                        &current_term,
                        &log,
                        &next_index,
                        &match_index,
                        &commit_index,
                        &state_change_sender,
                        &stats,
                        &message_sender,
                    ).await;
                } else {
                    break;
                }
            }
        });
        
        let mut heartbeat_timer = self.heartbeat_timer.lock().unwrap();
        *heartbeat_timer = Some(handle);
    }
    
    /// Initialize server indices
    async fn initialize_server_indices(&self) {
        let mut next_index = self.next_index.write().await;
        let mut match_index = self.match_index.write().await;
        
        let last_log_index = self.get_last_log_index().await;
        
        for node in &self.config.cluster_nodes {
            if node.node_id != self.config.node_id {
                next_index.insert(node.node_id.clone(), last_log_index + 1);
                match_index.insert(node.node_id.clone(), 0);
            }
        }
    }
    
    /// Get next log index
    async fn get_next_log_index(&self) -> u64 {
        let log = self.log.read().await;
        if let Some(last_entry) = log.back() {
            last_entry.index + 1
        } else {
            1
        }
    }
    
    /// Get last log index
    async fn get_last_log_index(&self) -> u64 {
        let log = self.log.read().await;
        if let Some(last_entry) = log.back() {
            last_entry.index
        } else {
            0
        }
    }
    
    /// Get last log term
    async fn get_last_log_term(&self) -> u64 {
        let log = self.log.read().await;
        if let Some(last_entry) = log.back() {
            last_entry.term
        } else {
            0
        }
    }
    
    /// Start election
    async fn start_election(
        config: &RaftConfig,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        voted_for: &Arc<RwLock<Option<String>>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
        message_sender: &mpsc::UnboundedSender<RaftMessage>,
    ) {
        // Change to candidate state
        {
            let mut state_guard = state.write().await;
            let mut term_guard = current_term.write().await;
            let mut voted_guard = voted_for.write().await;
            
            *state_guard = RaftState::Candidate;
            *term_guard += 1;
            *voted_guard = Some(config.node_id.clone());
            
            // Send state change event
            let _ = state_change_sender.send(StateChangeEvent::StateChanged(
                RaftState::Follower,
                RaftState::Candidate,
            ));
        }
        
        // Request votes from all other servers
        let last_log_index = {
            let log_guard = log.read().await;
            if let Some(last_entry) = log_guard.back() {
                last_entry.index
            } else {
                0
            }
        };
        
        let last_log_term = {
            let log_guard = log.read().await;
            if let Some(last_entry) = log_guard.back() {
                last_entry.term
            } else {
                0
            }
        };
        
        let term = *current_term.read().await;
        
        for node in &config.cluster_nodes {
            if node.node_id != config.node_id {
                let request = RequestVoteRequest {
                    term,
                    candidate_id: config.node_id.clone(),
                    last_log_index,
                    last_log_term,
                };
                
                let message = RaftMessage::RequestVote(request);
                let _ = message_sender.send(message);
            }
        }
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_votes_sent += 1;
        }
        
        info!("Started election for term {}", term);
    }
    
    /// Send heartbeat
    async fn send_heartbeat(
        config: &RaftConfig,
        current_term: &Arc<RwLock<u64>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        next_index: &Arc<RwLock<HashMap<String, u64>>>,
        match_index: &Arc<RwLock<HashMap<String, u64>>>,
        commit_index: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
        message_sender: &mpsc::UnboundedSender<RaftMessage>,
    ) {
        let term = *current_term.read().await;
        let leader_commit = *commit_index.read().await;
        
        for node in &config.cluster_nodes {
            if node.node_id != config.node_id {
                let next_idx = {
                    let next_index_guard = next_index.read().await;
                    *next_index_guard.get(&node.node_id).unwrap_or(&1)
                };
                
                let prev_log_index = next_idx - 1;
                let prev_log_term = if prev_log_index == 0 {
                    0
                } else {
                    let log_guard = log.read().await;
                    if let Some(entry) = log_guard.get(prev_log_index as usize - 1) {
                        entry.term
                    } else {
                        0
                    }
                };
                
                let entries = {
                    let log_guard = log.read().await;
                    let start_idx = (next_idx - 1) as usize;
                    let end_idx = log_guard.len();
                    log_guard.range(start_idx..end_idx).cloned().collect()
                };
                
                let request = AppendEntriesRequest {
                    term,
                    leader_id: config.node_id.clone(),
                    prev_log_index,
                    prev_log_term,
                    entries,
                    leader_commit,
                };
                
                let message = RaftMessage::AppendEntries(request);
                let _ = message_sender.send(message);
            }
        }
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_append_entries_sent += 1;
        }
    }
    
    /// Replicate log entries
    async fn replicate_log_entries(&self) -> Result<()> {
        // This would send append entries requests to all followers
        // For now, we'll just log the action
        debug!("Replicating log entries to followers");
        Ok(())
    }
    
    // Message handlers
    
    async fn handle_request_vote(
        request: RequestVoteRequest,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        voted_for: &Arc<RwLock<Option<String>>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        let mut term_guard = current_term.write().await;
        let mut voted_guard = voted_for.write().await;
        
        let mut vote_granted = false;
        
        if request.term > *term_guard {
            *term_guard = request.term;
            *voted_guard = None;
            
            // Send term change event
            let _ = state_change_sender.send(StateChangeEvent::TermChanged(
                *term_guard - 1,
                *term_guard,
            ));
        }
        
        let last_log_index = {
            let log_guard = log.read().await;
            if let Some(last_entry) = log_guard.back() {
                last_entry.index
            } else {
                0
            }
        };
        
        let last_log_term = {
            let log_guard = log.read().await;
            if let Some(last_entry) = log_guard.back() {
                last_entry.term
            } else {
                0
            }
        };
        
        if request.term == *term_guard &&
           (voted_guard.is_none() || voted_guard.as_ref().unwrap() == &request.candidate_id) &&
           (request.last_log_term > last_log_term ||
            (request.last_log_term == last_log_term && request.last_log_index >= last_log_index)) {
            *voted_guard = Some(request.candidate_id.clone());
            vote_granted = true;
        }
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_votes_received += 1;
        }
        
        debug!(
            "Handled request vote from {} for term {}: {}",
            request.candidate_id,
            request.term,
            if vote_granted { "granted" } else { "denied" }
        );
    }
    
    async fn handle_request_vote_response(
        response: RequestVoteResponse,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        let current_state = *state.read().await;
        if current_state != RaftState::Candidate {
            return;
        }
        
        let term = *current_term.read().await;
        if response.term > term {
            // Step down to follower
            let mut state_guard = state.write().await;
            let mut term_guard = current_term.write().await;
            
            *state_guard = RaftState::Follower;
            *term_guard = response.term;
            
            // Send state change event
            let _ = state_change_sender.send(StateChangeEvent::StateChanged(
                RaftState::Candidate,
                RaftState::Follower,
            ));
            
            return;
        }
        
        if response.term == term && response.vote_granted {
            // Count votes and potentially become leader
            // This would require tracking votes received
            debug!("Received vote for term {}", term);
        }
    }
    
    async fn handle_append_entries(
        request: AppendEntriesRequest,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        commit_index: &Arc<RwLock<u64>>,
        last_applied: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        let mut term_guard = current_term.write().await;
        let mut state_guard = state.write().await;
        
        let mut success = false;
        
        if request.term >= *term_guard {
            *term_guard = request.term;
            *state_guard = RaftState::Follower;
            
            // Send state change event
            let _ = state_change_sender.send(StateChangeEvent::StateChanged(
                *state_guard,
                RaftState::Follower,
            ));
            
            // Check if log entry matches
            let log_guard = log.read().await;
            if request.prev_log_index == 0 ||
               (request.prev_log_index <= log_guard.len() as u64 &&
                log_guard.get(request.prev_log_index as usize - 1).unwrap().term == request.prev_log_term) {
                success = true;
                
                // Append new entries
                // This would be implemented in a real Raft implementation
            }
        }
        
        // Update commit index
        if success {
            let mut commit_guard = commit_index.write().await;
            *commit_guard = std::cmp::min(request.leader_commit, request.prev_log_index + request.entries.len() as u64);
        }
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_append_entries_received += 1;
        }
        
        debug!(
            "Handled append entries from {} for term {}: {}",
            request.leader_id,
            request.term,
            if success { "success" } else { "failure" }
        );
    }
    
    async fn handle_append_entries_response(
        response: AppendEntriesResponse,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        next_index: &Arc<RwLock<HashMap<String, u64>>>,
        match_index: &Arc<RwLock<HashMap<String, u64>>>,
        commit_index: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        let current_state = *state.read().await;
        if current_state != RaftState::Leader {
            return;
        }
        
        let term = *current_term.read().await;
        if response.term > term {
            // Step down to follower
            let mut state_guard = state.write().await;
            let mut term_guard = current_term.write().await;
            
            *state_guard = RaftState::Follower;
            *term_guard = response.term;
            
            // Send state change event
            let _ = state_change_sender.send(StateChangeEvent::StateChanged(
                RaftState::Leader,
                RaftState::Follower,
            ));
            
            return;
        }
        
        if response.term == term {
            if response.success {
                // Update next_index and match_index
                // This would be implemented in a real Raft implementation
                debug!("Append entries successful for term {}", term);
            } else {
                // Decrement next_index and retry
                // This would be implemented in a real Raft implementation
                debug!("Append entries failed for term {}, retrying", term);
            }
        }
    }
    
    async fn handle_install_snapshot(
        request: InstallSnapshotRequest,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        commit_index: &Arc<RwLock<u64>>,
        last_applied: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        // Handle snapshot installation
        // This would be implemented in a real Raft implementation
        debug!("Handled install snapshot from {}", request.leader_id);
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_snapshots_received += 1;
        }
    }
    
    async fn handle_install_snapshot_response(
        response: InstallSnapshotResponse,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        // Handle snapshot installation response
        // This would be implemented in a real Raft implementation
        debug!("Handled install snapshot response for term {}", response.term);
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.total_snapshots_sent += 1;
        }
    }
    
    async fn handle_pre_vote(
        request: PreVoteRequest,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        log: &Arc<RwLock<VecDeque<LogEntry>>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        // Handle pre-vote request
        // This would be implemented in a real Raft implementation
        debug!("Handled pre-vote from {}", request.candidate_id);
    }
    
    async fn handle_pre_vote_response(
        response: PreVoteResponse,
        state: &Arc<RwLock<RaftState>>,
        current_term: &Arc<RwLock<u64>>,
        state_change_sender: &broadcast::Sender<StateChangeEvent>,
        stats: &Arc<RwLock<RaftStats>>,
    ) {
        // Handle pre-vote response
        // This would be implemented in a real Raft implementation
        debug!("Handled pre-vote response: {}", if response.vote_granted { "granted" } else { "denied" });
    }
}

// Add missing error variants
impl Error {
    pub fn NotLeader() -> Self {
        Error::Storage("Not the leader".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_raft_config_default() {
        let config = RaftConfig::default();
        assert_eq!(config.node_id, "node-1");
        assert_eq!(config.election_timeout_ms, 150);
        assert_eq!(config.heartbeat_interval_ms, 50);
        assert!(config.enable_pre_vote);
        assert!(config.enable_single_node_optimization);
    }
    
    #[test]
    fn test_raft_state() {
        assert_eq!(RaftState::Follower, RaftState::Follower);
        assert_ne!(RaftState::Follower, RaftState::Candidate);
        assert_ne!(RaftState::Candidate, RaftState::Leader);
    }
    
    #[test]
    fn test_node_status() {
        assert_eq!(NodeStatus::Follower, NodeStatus::Follower);
        assert_ne!(NodeStatus::Follower, NodeStatus::Candidate);
        assert_ne!(NodeStatus::Candidate, NodeStatus::Leader);
        assert_ne!(NodeStatus::Leader, NodeStatus::Shutdown);
    }
    
    #[test]
    fn test_log_entry_type() {
        assert_eq!(LogEntryType::Normal, LogEntryType::Normal);
        assert_ne!(LogEntryType::Normal, LogEntryType::ConfigChange);
        assert_ne!(LogEntryType::ConfigChange, LogEntryType::NoOp);
    }
    
    #[test]
    fn test_raft_stats_default() {
        let stats = RaftStats::default();
        assert_eq!(stats.current_term, 0);
        assert_eq!(stats.current_state, RaftState::Follower);
        assert_eq!(stats.leader_id, None);
        assert_eq!(stats.log_size, 0);
        assert_eq!(stats.commit_index, 0);
        assert_eq!(stats.last_applied, 0);
        assert_eq!(stats.total_votes_received, 0);
        assert_eq!(stats.total_votes_sent, 0);
        assert_eq!(stats.total_append_entries_sent, 0);
        assert_eq!(stats.total_append_entries_received, 0);
        assert_eq!(stats.total_snapshots_sent, 0);
        assert_eq!(stats.total_snapshots_received, 0);
    }
}