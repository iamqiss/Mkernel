//! Error types for Qiss format operations

use thiserror::Error;

/// Errors that can occur during Qiss serialization/deserialization
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid magic number in header
    #[error("Invalid magic number: {0:#x}")]
    InvalidMagic(u32),

    /// Unsupported version
    #[error("Unsupported Qiss version: {0}")]
    UnsupportedVersion(u8),

    /// Message too large
    #[error("Message too large: {0} bytes (max: {})", crate::MAX_MESSAGE_SIZE)]
    MessageTooLarge(usize),

    /// Invalid message type
    #[error("Invalid message type: {0:#x}")]
    InvalidMessageType(u8),

    /// Insufficient data for deserialization
    #[error("Insufficient data for deserialization")]
    InsufficientData,

    /// Invalid header
    #[error("Invalid Qiss header")]
    InvalidHeader,

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch { expected: u32, actual: u32 },

    /// Buffer overflow
    #[error("Buffer overflow")]
    BufferOverflow,

    /// Invalid string encoding
    #[error("Invalid string encoding")]
    InvalidStringEncoding,

    /// Invalid UTF-8 sequence
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,

    /// Invalid array length
    #[error("Invalid array length: {0}")]
    InvalidArrayLength(usize),

    /// Invalid field type
    #[error("Invalid field type")]
    InvalidFieldType,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

impl Error {
    /// Create a serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a deserialization error
    pub fn deserialization(msg: impl Into<String>) -> Self {
        Self::Deserialization(msg.into())
    }

    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}