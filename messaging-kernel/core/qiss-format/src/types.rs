//! Type definitions for Qiss format

use std::time::{SystemTime, UNIX_EPOCH};

/// Timestamp type for Qiss format (nanoseconds since Unix epoch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create a timestamp from the current time
    pub fn now() -> Self {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch");
        Self(duration.as_nanos() as u64)
    }

    /// Create a timestamp from nanoseconds since Unix epoch
    pub fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Get nanoseconds since Unix epoch
    pub fn as_nanos(self) -> u64 {
        self.0
    }

    /// Convert to SystemTime
    pub fn to_system_time(self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_nanos(self.0)
    }
}

impl From<SystemTime> for Timestamp {
    fn from(time: SystemTime) -> Self {
        let duration = time.duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch");
        Self(duration.as_nanos() as u64)
    }
}

impl From<Timestamp> for SystemTime {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.to_system_time()
    }
}

/// Qiss field type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit unsigned integer
    U64,
    /// 8-bit signed integer
    I8,
    /// 16-bit signed integer
    I16,
    /// 32-bit signed integer
    I32,
    /// 64-bit signed integer
    I64,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
    /// Boolean
    Bool,
    /// String
    String,
    /// Byte array
    Bytes,
    /// Timestamp
    Timestamp,
    /// Array
    Array,
    /// Optional field
    Optional,
    /// Map
    Map,
}

impl FieldType {
    /// Get the byte representation of the field type
    pub fn to_byte(self) -> u8 {
        match self {
            FieldType::U8 => 0x01,
            FieldType::U16 => 0x02,
            FieldType::U32 => 0x03,
            FieldType::U64 => 0x04,
            FieldType::I8 => 0x05,
            FieldType::I16 => 0x06,
            FieldType::I32 => 0x07,
            FieldType::I64 => 0x08,
            FieldType::F32 => 0x09,
            FieldType::F64 => 0x0A,
            FieldType::Bool => 0x0B,
            FieldType::String => 0x0C,
            FieldType::Bytes => 0x0D,
            FieldType::Timestamp => 0x0E,
            FieldType::Array => 0x0F,
            FieldType::Optional => 0x10,
            FieldType::Map => 0x11,
        }
    }

    /// Parse field type from byte representation
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(FieldType::U8),
            0x02 => Some(FieldType::U16),
            0x03 => Some(FieldType::U32),
            0x04 => Some(FieldType::U64),
            0x05 => Some(FieldType::I8),
            0x06 => Some(FieldType::I16),
            0x07 => Some(FieldType::I32),
            0x08 => Some(FieldType::I64),
            0x09 => Some(FieldType::F32),
            0x0A => Some(FieldType::F64),
            0x0B => Some(FieldType::Bool),
            0x0C => Some(FieldType::String),
            0x0D => Some(FieldType::Bytes),
            0x0E => Some(FieldType::Timestamp),
            0x0F => Some(FieldType::Array),
            0x10 => Some(FieldType::Optional),
            0x11 => Some(FieldType::Map),
            _ => None,
        }
    }
}

/// Qiss field descriptor
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Whether the field is optional
    pub optional: bool,
    /// Nested type for arrays and maps
    pub nested_type: Option<Box<FieldDescriptor>>,
}

impl FieldDescriptor {
    /// Create a new field descriptor
    pub fn new(name: String, field_type: FieldType) -> Self {
        Self {
            name,
            field_type,
            optional: false,
            nested_type: None,
        }
    }

    /// Create an optional field descriptor
    pub fn optional(name: String, field_type: FieldType) -> Self {
        Self {
            name,
            field_type,
            optional: true,
            nested_type: None,
        }
    }

    /// Create an array field descriptor
    pub fn array(name: String, element_type: FieldDescriptor) -> Self {
        Self {
            name,
            field_type: FieldType::Array,
            optional: false,
            nested_type: Some(Box::new(element_type)),
        }
    }

    /// Create a map field descriptor
    pub fn map(name: String, key_type: FieldDescriptor, value_type: FieldDescriptor) -> Self {
        // For simplicity, we'll use a single nested type for the value
        // In a more complex implementation, we'd need to handle both key and value types
        Self {
            name,
            field_type: FieldType::Map,
            optional: false,
            nested_type: Some(Box::new(value_type)),
        }
    }
}

/// Qiss message descriptor
#[derive(Debug, Clone)]
pub struct MessageDescriptor {
    /// Message name
    pub name: String,
    /// Message fields
    pub fields: Vec<FieldDescriptor>,
}

impl MessageDescriptor {
    /// Create a new message descriptor
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: Vec::new(),
        }
    }

    /// Add a field to the message
    pub fn add_field(mut self, field: FieldDescriptor) -> Self {
        self.fields.push(field);
        self
    }
}