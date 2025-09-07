//! Abstract Syntax Tree (AST) definitions for Neo protocol

use std::collections::HashMap;
use neo_lexer::Span;

/// A complete service definition
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceDefinition {
    /// The name of the service
    pub name: String,
    /// The version of the service
    pub version: String,
    /// The namespace of the service
    pub namespace: String,
    /// The messages defined in the service
    pub messages: Vec<MessageDefinition>,
    /// The RPC methods defined in the service
    pub rpcs: Vec<RpcDefinition>,
    /// The events defined in the service
    pub events: Vec<EventDefinition>,
    /// The configuration for the service
    pub config: Option<ServiceConfig>,
    /// The span of the service definition
    pub span: Span,
}

/// A message definition
#[derive(Debug, Clone, PartialEq)]
pub struct MessageDefinition {
    /// The name of the message
    pub name: String,
    /// The fields of the message
    pub fields: Vec<FieldDefinition>,
    /// The span of the message definition
    pub span: Span,
}

/// A field definition within a message
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefinition {
    /// The name of the field
    pub name: String,
    /// The type of the field
    pub field_type: FieldType,
    /// Whether the field is optional
    pub optional: bool,
    /// The span of the field definition
    pub span: Span,
}

/// A field type
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    /// Primitive types
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Bool,
    String,
    Bytes,
    Timestamp,
    /// Reference to another message type
    Message(String),
    /// Array type
    Array(Box<FieldType>),
    /// Map type
    Map {
        key_type: Box<FieldType>,
        value_type: Box<FieldType>,
    },
}

impl FieldType {
    /// Get the string representation of the field type
    pub fn as_str(&self) -> &str {
        match self {
            FieldType::U8 => "u8",
            FieldType::U16 => "u16",
            FieldType::U32 => "u32",
            FieldType::U64 => "u64",
            FieldType::I8 => "i8",
            FieldType::I16 => "i16",
            FieldType::I32 => "i32",
            FieldType::I64 => "i64",
            FieldType::F32 => "f32",
            FieldType::F64 => "f64",
            FieldType::Bool => "bool",
            FieldType::String => "string",
            FieldType::Bytes => "bytes",
            FieldType::Timestamp => "timestamp",
            FieldType::Message(name) => name,
            FieldType::Array(_) => "array",
            FieldType::Map { .. } => "map",
        }
    }

    /// Check if this is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            FieldType::U8
                | FieldType::U16
                | FieldType::U32
                | FieldType::U64
                | FieldType::I8
                | FieldType::I16
                | FieldType::I32
                | FieldType::I64
                | FieldType::F32
                | FieldType::F64
                | FieldType::Bool
                | FieldType::String
                | FieldType::Bytes
                | FieldType::Timestamp
        )
    }

    /// Check if this is a message reference
    pub fn is_message(&self) -> bool {
        matches!(self, FieldType::Message(_))
    }

    /// Check if this is an array type
    pub fn is_array(&self) -> bool {
        matches!(self, FieldType::Array(_))
    }

    /// Check if this is a map type
    pub fn is_map(&self) -> bool {
        matches!(self, FieldType::Map { .. })
    }
}

/// An RPC method definition
#[derive(Debug, Clone, PartialEq)]
pub struct RpcDefinition {
    /// The name of the RPC method
    pub name: String,
    /// The input message type
    pub input_type: String,
    /// The output message type
    pub output_type: String,
    /// The queue configuration
    pub queue: Option<String>,
    /// The timeout configuration
    pub timeout: Option<Duration>,
    /// The retry policy configuration
    pub retry_policy: Option<RetryPolicy>,
    /// The span of the RPC definition
    pub span: Span,
}

/// An event definition
#[derive(Debug, Clone, PartialEq)]
pub struct EventDefinition {
    /// The name of the event
    pub name: String,
    /// The event message type
    pub message_type: String,
    /// The topic configuration
    pub topic: Option<String>,
    /// The partition key configuration
    pub partition_key: Option<String>,
    /// The retention configuration
    pub retention: Option<Duration>,
    /// The span of the event definition
    pub span: Span,
}

/// A duration value
#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    /// The numeric value
    pub value: u64,
    /// The unit (s, m, h, d, w, y)
    pub unit: DurationUnit,
}

impl Duration {
    /// Create a new duration
    pub fn new(value: u64, unit: DurationUnit) -> Self {
        Self { value, unit }
    }

    /// Convert to nanoseconds
    pub fn as_nanos(&self) -> u64 {
        match self.unit {
            DurationUnit::Nanoseconds => self.value,
            DurationUnit::Microseconds => self.value * 1_000,
            DurationUnit::Milliseconds => self.value * 1_000_000,
            DurationUnit::Seconds => self.value * 1_000_000_000,
            DurationUnit::Minutes => self.value * 60 * 1_000_000_000,
            DurationUnit::Hours => self.value * 3600 * 1_000_000_000,
            DurationUnit::Days => self.value * 86400 * 1_000_000_000,
            DurationUnit::Weeks => self.value * 604800 * 1_000_000_000,
            DurationUnit::Years => self.value * 31536000 * 1_000_000_000,
        }
    }

    /// Convert to seconds
    pub fn as_seconds(&self) -> u64 {
        self.as_nanos() / 1_000_000_000
    }
}

/// Duration units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationUnit {
    Nanoseconds,
    Microseconds,
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Years,
}

impl DurationUnit {
    /// Parse a duration unit from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ns" => Some(DurationUnit::Nanoseconds),
            "us" => Some(DurationUnit::Microseconds),
            "ms" => Some(DurationUnit::Milliseconds),
            "s" => Some(DurationUnit::Seconds),
            "m" => Some(DurationUnit::Minutes),
            "h" => Some(DurationUnit::Hours),
            "d" => Some(DurationUnit::Days),
            "w" => Some(DurationUnit::Weeks),
            "y" => Some(DurationUnit::Years),
            _ => None,
        }
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            DurationUnit::Nanoseconds => "ns",
            DurationUnit::Microseconds => "us",
            DurationUnit::Milliseconds => "ms",
            DurationUnit::Seconds => "s",
            DurationUnit::Minutes => "m",
            DurationUnit::Hours => "h",
            DurationUnit::Days => "d",
            DurationUnit::Weeks => "w",
            DurationUnit::Years => "y",
        }
    }
}

/// A retry policy configuration
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// The type of retry policy
    pub policy_type: RetryPolicyType,
    /// The maximum number of attempts
    pub max_attempts: Option<u32>,
    /// The initial delay
    pub initial_delay: Option<Duration>,
    /// The maximum delay
    pub max_delay: Option<Duration>,
    /// The backoff multiplier
    pub multiplier: Option<f64>,
}

/// Retry policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryPolicyType {
    ExponentialBackoff,
    LinearBackoff,
    FixedDelay,
}

impl RetryPolicyType {
    /// Parse a retry policy type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "exponential_backoff" => Some(RetryPolicyType::ExponentialBackoff),
            "linear_backoff" => Some(RetryPolicyType::LinearBackoff),
            "fixed_delay" => Some(RetryPolicyType::FixedDelay),
            _ => None,
        }
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            RetryPolicyType::ExponentialBackoff => "exponential_backoff",
            RetryPolicyType::LinearBackoff => "linear_backoff",
            RetryPolicyType::FixedDelay => "fixed_delay",
        }
    }
}

/// Service configuration
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceConfig {
    /// Maximum concurrent requests
    pub max_concurrent_requests: Option<u32>,
    /// Queue buffer size
    pub queue_buffer_size: Option<u32>,
    /// Compression algorithm
    pub compression: Option<CompressionType>,
    /// Serialization format
    pub serialization: Option<SerializationType>,
}

/// Compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    Lz4,
    Gzip,
    Zstd,
}

impl CompressionType {
    /// Parse a compression type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "lz4" => Some(CompressionType::Lz4),
            "gzip" => Some(CompressionType::Gzip),
            "zstd" => Some(CompressionType::Zstd),
            _ => None,
        }
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            CompressionType::Lz4 => "lz4",
            CompressionType::Gzip => "gzip",
            CompressionType::Zstd => "zstd",
        }
    }
}

/// Serialization types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationType {
    Qiss,
    Protobuf,
    Json,
}

impl SerializationType {
    /// Parse a serialization type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "qiss" => Some(SerializationType::Qiss),
            "protobuf" => Some(SerializationType::Protobuf),
            "json" => Some(SerializationType::Json),
            _ => None,
        }
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            SerializationType::Qiss => "qiss",
            SerializationType::Protobuf => "protobuf",
            SerializationType::Json => "json",
        }
    }
}

/// A size value
#[derive(Debug, Clone, PartialEq)]
pub struct Size {
    /// The numeric value
    pub value: u64,
    /// The unit (B, KB, MB, GB, TB)
    pub unit: SizeUnit,
}

impl Size {
    /// Create a new size
    pub fn new(value: u64, unit: SizeUnit) -> Self {
        Self { value, unit }
    }

    /// Convert to bytes
    pub fn as_bytes(&self) -> u64 {
        match self.unit {
            SizeUnit::Bytes => self.value,
            SizeUnit::Kilobytes => self.value * 1024,
            SizeUnit::Megabytes => self.value * 1024 * 1024,
            SizeUnit::Gigabytes => self.value * 1024 * 1024 * 1024,
            SizeUnit::Terabytes => self.value * 1024 * 1024 * 1024 * 1024,
        }
    }
}

/// Size units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Terabytes,
}

impl SizeUnit {
    /// Parse a size unit from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "B" => Some(SizeUnit::Bytes),
            "KB" => Some(SizeUnit::Kilobytes),
            "MB" => Some(SizeUnit::Megabytes),
            "GB" => Some(SizeUnit::Gigabytes),
            "TB" => Some(SizeUnit::Terabytes),
            _ => None,
        }
    }

    /// Get the string representation
    pub fn as_str(self) -> &'static str {
        match self {
            SizeUnit::Bytes => "B",
            SizeUnit::Kilobytes => "KB",
            SizeUnit::Megabytes => "MB",
            SizeUnit::Gigabytes => "GB",
            SizeUnit::Terabytes => "TB",
        }
    }
}