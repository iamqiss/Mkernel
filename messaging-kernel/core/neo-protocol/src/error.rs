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
}
