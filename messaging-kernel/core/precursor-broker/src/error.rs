//! Error types for precursor-broker

use thiserror::Error;

/// Error types for precursor-broker
#[derive(Error, Debug)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Queue not found
    #[error("Queue not found: {0}")]
    QueueNotFound(String),
    
    /// Topic not found
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    /// Consumer not found
    #[error("Consumer not found: {0}")]
    ConsumerNotFound(String),
    
    /// Producer not found
    #[error("Producer not found: {0}")]
    ProducerNotFound(String),
    
    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(String),
    
    /// Queue full
    #[error("Queue full: {0}")]
    QueueFull(String),
    
    /// Consumer group not found
    #[error("Consumer group not found: {0}")]
    ConsumerGroupNotFound(String),
    
    /// Partition not found
    #[error("Partition not found: {0}")]
    PartitionNotFound(String),
    
    /// Offset out of range
    #[error("Offset out of range: {0}")]
    OffsetOutOfRange(String),
    
    /// Persistence error
    #[error("Persistence error: {0}")]
    Persistence(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Replication error
    #[error("Replication error: {0}")]
    Replication(String),
    
    /// Leader election error
    #[error("Leader election error: {0}")]
    LeaderElection(String),
    
    /// Cluster error
    #[error("Cluster error: {0}")]
    Cluster(String),
    
    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),
    
    /// Timeout error
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    /// Invalid message format
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    
    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
    
    /// Insufficient data
    #[error("Insufficient data: need {needed} bytes, got {got} bytes")]
    InsufficientData { needed: usize, got: usize },
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
