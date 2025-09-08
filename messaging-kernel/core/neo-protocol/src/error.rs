//! Error types for neo-protocol

use thiserror::Error;

/// Error types for neo-protocol
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
    
    /// Invalid magic number
    #[error("Invalid magic number: {0:#x}")]
    InvalidMagic(u32),
    
    /// Unsupported protocol version
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(u8),
    
    /// Invalid message type
    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),
    
    /// Message too large
    #[error("Message too large: {0} bytes (max: {1} bytes)")]
    MessageTooLarge(usize, usize),
    
    /// RPC timeout
    #[error("RPC timeout after {0:?}")]
    RpcTimeout(std::time::Duration),
    
    /// Service not found
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    /// Method not found
    #[error("Method not found: {0}.{1}")]
    MethodNotFound(String, String),
    
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    /// Authorization failed
    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),
    
    /// Connection closed
    #[error("Connection closed")]
    ConnectionClosed,
    
    /// Invalid correlation ID
    #[error("Invalid correlation ID: {0}")]
    InvalidCorrelationId(u64),
    
    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
    
    /// Insufficient data
    #[error("Insufficient data: need {needed} bytes, got {got} bytes")]
    InsufficientData { needed: usize, got: usize },
    
    /// Invalid header
    #[error("Invalid header")]
    InvalidHeader,
    
    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),
    
    /// Security error
    #[error("Security error: {0}")]
    Security(String),
    
    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
    
    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
