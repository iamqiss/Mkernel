//! Error types for qiss-format

use thiserror::Error;

/// Error types for qiss-format
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
