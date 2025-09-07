//! Error types for the Neo protocol parser

use thiserror::Error;
use neo_lexer::{LexerError, TokenKind, Span};

/// Errors that can occur during parsing
#[derive(Error, Debug)]
pub enum ParserError {
    /// Lexer error
    #[error("Lexer error: {0}")]
    LexerError(#[from] LexerError),

    /// Unexpected token
    #[error("Unexpected token: expected '{expected}', found '{found:?}' at {span}")]
    UnexpectedToken {
        /// The expected token or description
        expected: String,
        /// The actual token kind found
        found: TokenKind,
        /// The span of the unexpected token
        span: Span,
    },

    /// Unexpected end of file
    #[error("Unexpected end of file at {span}")]
    UnexpectedEof {
        /// The span where EOF was encountered
        span: Span,
    },

    /// Invalid numeric literal
    #[error("Invalid numeric literal '{text}' at {span}")]
    InvalidNumericLiteral {
        /// The invalid numeric text
        text: String,
        /// The span of the invalid numeric
        span: Span,
    },

    /// Invalid duration unit
    #[error("Invalid duration unit '{unit}' at {span}")]
    InvalidDurationUnit {
        /// The invalid duration unit
        unit: String,
        /// The span of the invalid unit
        span: Span,
    },

    /// Invalid retry policy type
    #[error("Invalid retry policy type '{policy_type}' at {span}")]
    InvalidRetryPolicyType {
        /// The invalid retry policy type
        policy_type: String,
        /// The span of the invalid type
        span: Span,
    },

    /// Invalid retry policy parameter
    #[error("Invalid retry policy parameter '{parameter}' at {span}")]
    InvalidRetryPolicyParameter {
        /// The invalid parameter name
        parameter: String,
        /// The span of the invalid parameter
        span: Span,
    },

    /// Invalid compression type
    #[error("Invalid compression type '{compression_type}' at {span}")]
    InvalidCompressionType {
        /// The invalid compression type
        compression_type: String,
        /// The span of the invalid type
        span: Span,
    },

    /// Invalid serialization type
    #[error("Invalid serialization type '{serialization_type}' at {span}")]
    InvalidSerializationType {
        /// The invalid serialization type
        serialization_type: String,
        /// The span of the invalid type
        span: Span,
    },

    /// Duplicate field name
    #[error("Duplicate field name '{field_name}' at {span}")]
    DuplicateFieldName {
        /// The duplicate field name
        field_name: String,
        /// The span of the duplicate field
        span: Span,
    },

    /// Duplicate RPC name
    #[error("Duplicate RPC name '{rpc_name}' at {span}")]
    DuplicateRpcName {
        /// The duplicate RPC name
        rpc_name: String,
        /// The span of the duplicate RPC
        span: Span,
    },

    /// Duplicate event name
    #[error("Duplicate event name '{event_name}' at {span}")]
    DuplicateEventName {
        /// The duplicate event name
        event_name: String,
        /// The span of the duplicate event
        span: Span,
    },

    /// Undefined message type
    #[error("Undefined message type '{message_type}' at {span}")]
    UndefinedMessageType {
        /// The undefined message type
        message_type: String,
        /// The span of the undefined type
        span: Span,
    },

    /// Circular dependency
    #[error("Circular dependency detected for message type '{message_type}' at {span}")]
    CircularDependency {
        /// The message type with circular dependency
        message_type: String,
        /// The span of the circular dependency
        span: Span,
    },

    /// Invalid field type
    #[error("Invalid field type '{field_type}' at {span}")]
    InvalidFieldType {
        /// The invalid field type
        field_type: String,
        /// The span of the invalid type
        span: Span,
    },

    /// Missing required field
    #[error("Missing required field '{field_name}' in message '{message_name}' at {span}")]
    MissingRequiredField {
        /// The missing field name
        field_name: String,
        /// The message name
        message_name: String,
        /// The span of the missing field
        span: Span,
    },

    /// Invalid service configuration
    #[error("Invalid service configuration: {message} at {span}")]
    InvalidServiceConfig {
        /// The error message
        message: String,
        /// The span of the invalid configuration
        span: Span,
    },

    /// Invalid RPC configuration
    #[error("Invalid RPC configuration: {message} at {span}")]
    InvalidRpcConfig {
        /// The error message
        message: String,
        /// The span of the invalid configuration
        span: Span,
    },

    /// Invalid event configuration
    #[error("Invalid event configuration: {message} at {span}")]
    InvalidEventConfig {
        /// The error message
        message: String,
        /// The span of the invalid configuration
        span: Span,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

impl ParserError {
    /// Create an unexpected token error
    pub fn unexpected_token(expected: impl Into<String>, found: TokenKind, span: Span) -> Self {
        Self::UnexpectedToken {
            expected: expected.into(),
            found,
            span,
        }
    }

    /// Create an unexpected EOF error
    pub fn unexpected_eof(span: Span) -> Self {
        Self::UnexpectedEof { span }
    }

    /// Create an invalid numeric literal error
    pub fn invalid_numeric_literal(text: impl Into<String>, span: Span) -> Self {
        Self::InvalidNumericLiteral {
            text: text.into(),
            span,
        }
    }

    /// Create an invalid duration unit error
    pub fn invalid_duration_unit(unit: impl Into<String>, span: Span) -> Self {
        Self::InvalidDurationUnit {
            unit: unit.into(),
            span,
        }
    }

    /// Create an invalid retry policy type error
    pub fn invalid_retry_policy_type(policy_type: impl Into<String>, span: Span) -> Self {
        Self::InvalidRetryPolicyType {
            policy_type: policy_type.into(),
            span,
        }
    }

    /// Create an invalid retry policy parameter error
    pub fn invalid_retry_policy_parameter(parameter: impl Into<String>, span: Span) -> Self {
        Self::InvalidRetryPolicyParameter {
            parameter: parameter.into(),
            span,
        }
    }

    /// Create an invalid compression type error
    pub fn invalid_compression_type(compression_type: impl Into<String>, span: Span) -> Self {
        Self::InvalidCompressionType {
            compression_type: compression_type.into(),
            span,
        }
    }

    /// Create an invalid serialization type error
    pub fn invalid_serialization_type(serialization_type: impl Into<String>, span: Span) -> Self {
        Self::InvalidSerializationType {
            serialization_type: serialization_type.into(),
            span,
        }
    }

    /// Create a duplicate field name error
    pub fn duplicate_field_name(field_name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateFieldName {
            field_name: field_name.into(),
            span,
        }
    }

    /// Create a duplicate RPC name error
    pub fn duplicate_rpc_name(rpc_name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateRpcName {
            rpc_name: rpc_name.into(),
            span,
        }
    }

    /// Create a duplicate event name error
    pub fn duplicate_event_name(event_name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateEventName {
            event_name: event_name.into(),
            span,
        }
    }

    /// Create an undefined message type error
    pub fn undefined_message_type(message_type: impl Into<String>, span: Span) -> Self {
        Self::UndefinedMessageType {
            message_type: message_type.into(),
            span,
        }
    }

    /// Create a circular dependency error
    pub fn circular_dependency(message_type: impl Into<String>, span: Span) -> Self {
        Self::CircularDependency {
            message_type: message_type.into(),
            span,
        }
    }

    /// Create an invalid field type error
    pub fn invalid_field_type(field_type: impl Into<String>, span: Span) -> Self {
        Self::InvalidFieldType {
            field_type: field_type.into(),
            span,
        }
    }

    /// Create a missing required field error
    pub fn missing_required_field(field_name: impl Into<String>, message_name: impl Into<String>, span: Span) -> Self {
        Self::MissingRequiredField {
            field_name: field_name.into(),
            message_name: message_name.into(),
            span,
        }
    }

    /// Create an invalid service configuration error
    pub fn invalid_service_config(message: impl Into<String>, span: Span) -> Self {
        Self::InvalidServiceConfig {
            message: message.into(),
            span,
        }
    }

    /// Create an invalid RPC configuration error
    pub fn invalid_rpc_config(message: impl Into<String>, span: Span) -> Self {
        Self::InvalidRpcConfig {
            message: message.into(),
            span,
        }
    }

    /// Create an invalid event configuration error
    pub fn invalid_event_config(message: impl Into<String>, span: Span) -> Self {
        Self::InvalidEventConfig {
            message: message.into(),
            span,
        }
    }

    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        Self::Custom(msg.into())
    }
}

/// Result type for parser operations
pub type Result<T> = std::result::Result<T, ParserError>;