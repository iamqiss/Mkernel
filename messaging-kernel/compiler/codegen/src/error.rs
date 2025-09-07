//! Error types for code generation

use thiserror::Error;

/// Errors that can occur during code generation
#[derive(Error, Debug)]
pub enum CodegenError {
    /// Invalid service definition
    #[error("Invalid service definition: {0}")]
    InvalidServiceDefinition(String),

    /// Invalid message definition
    #[error("Invalid message definition: {0}")]
    InvalidMessageDefinition(String),

    /// Invalid RPC definition
    #[error("Invalid RPC definition: {0}")]
    InvalidRpcDefinition(String),

    /// Invalid event definition
    #[error("Invalid event definition: {0}")]
    InvalidEventDefinition(String),

    /// Code generation failed
    #[error("Code generation failed: {0}")]
    GenerationFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

impl CodegenError {
    /// Create an invalid service definition error
    pub fn invalid_service_definition(msg: impl Into<String>) -> Self {
        Self::InvalidServiceDefinition(msg.into())
    }

    /// Create an invalid message definition error
    pub fn invalid_message_definition(msg: impl Into<String>) -> Self {
        Self::InvalidMessageDefinition(msg.into())
    }

    /// Create an invalid RPC definition error
    pub fn invalid_rpc_definition(msg: impl Into<String>) -> Self {
        Self::InvalidRpcDefinition(msg.into())
    }

    /// Create an invalid event definition error
    pub fn invalid_event_definition(msg: impl Into<String>) -> Self {
        Self::InvalidEventDefinition(msg.into())
    }

    /// Create a generation failed error
    pub fn generation_failed(msg: impl Into<String>) -> Self {
        Self::GenerationFailed(msg.into())
    }

    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}