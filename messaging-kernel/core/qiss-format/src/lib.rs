//! Binary serialization format optimized for speed
//! 
//! This crate is part of the Neo Messaging Kernel, providing binary serialization format optimized for speed.
//! 
//! Qiss (Quick Inter-Service Serialization) is designed for maximum performance with zero-copy operations
//! where possible. It uses a binary-first approach with careful bit-packing and fixed-size optimizations.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::mem;
use bytes::{Bytes, BytesMut, BufMut};
use bytemuck::{Pod, Zeroable};
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned};

pub mod error;
pub mod types;
pub mod writer;
pub mod reader;
pub mod codec;

pub use error::Error;
pub use types::*;
pub use writer::QissWriter;
pub use reader::QissReader;
pub use codec::QissCodec;

/// Result type for this crate
pub type Result<T> = std::result::Result<T, Error>;

/// Magic number for Qiss format identification
pub const QISS_MAGIC: u32 = 0x51535300; // "QSS\0"

/// Current version of the Qiss format
pub const QISS_VERSION: u8 = 1;

/// Maximum size for a single Qiss message (10MB)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Header for Qiss messages
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct QissHeader {
    /// Magic number for format identification
    pub magic: u32,
    /// Format version
    pub version: u8,
    /// Message type flags
    pub flags: u8,
    /// Reserved bytes for future use
    pub reserved: u16,
    /// Total message size including header
    pub size: u32,
    /// CRC32 checksum of the message body
    pub checksum: u32,
}

unsafe impl AsBytes for QissHeader {
    fn only_derive_is_allowed_to_implement_this_trait() { todo!() }
}
unsafe impl FromBytes for QissHeader {
    fn only_derive_is_allowed_to_implement_this_trait() { todo!() }
}
unsafe impl FromZeroes for QissHeader {
    fn only_derive_is_allowed_to_implement_this_trait() { todo!() }
}
unsafe impl Unaligned for QissHeader {
    fn only_derive_is_allowed_to_implement_this_trait() { todo!() }
}

impl QissHeader {
    /// Create a new Qiss header
    pub fn new(size: u32, checksum: u32, flags: u8) -> Self {
        Self {
            magic: QISS_MAGIC,
            version: QISS_VERSION,
            flags,
            reserved: 0,
            size,
            checksum,
        }
    }

    /// Validate the header
    pub fn validate(&self) -> Result<()> {
        if self.magic != QISS_MAGIC {
            return Err(Error::InvalidMagic(self.magic));
        }
        if self.version != QISS_VERSION {
            return Err(Error::UnsupportedVersion(self.version));
        }
        if self.size > MAX_MESSAGE_SIZE as u32 {
            return Err(Error::MessageTooLarge(self.size as usize));
        }
        Ok(())
    }
}

/// Qiss message type flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Request message
    Request = 0x01,
    /// Response message
    Response = 0x02,
    /// Event message
    Event = 0x04,
    /// Error message
    Error = 0x08,
}

impl MessageType {
    /// Convert to byte representation
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Parse from byte representation
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0x01 => Ok(MessageType::Request),
            0x02 => Ok(MessageType::Response),
            0x04 => Ok(MessageType::Event),
            0x08 => Ok(MessageType::Error),
            _ => Err(Error::InvalidMessageType(byte)),
        }
    }
}

/// High-performance Qiss serializer
pub struct QissSerializer {
    buffer: BytesMut,
}

impl QissSerializer {
    /// Create a new serializer with default capacity
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a new serializer with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
        }
    }

    /// Serialize a value into Qiss format
    pub fn serialize<T: QissSerializable>(&mut self, value: &T) -> Result<Bytes> {
        self.buffer.clear();
        
        // Reserve space for header
        self.buffer.reserve(mem::size_of::<QissHeader>());
        self.buffer.put_u32(0); // magic
        self.buffer.put_u8(0);  // version
        self.buffer.put_u8(0);  // flags
        self.buffer.put_u16(0); // reserved
        self.buffer.put_u32(0); // size
        self.buffer.put_u32(0); // checksum

        // Serialize the value
        let start_pos = self.buffer.len();
        value.serialize(&mut QissWriter::new(&mut self.buffer))?;
        let end_pos = self.buffer.len();

        // Calculate checksum
        let body = &self.buffer[start_pos..end_pos];
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(body);
        let checksum = hasher.finalize();

        // Update header
        let total_size = self.buffer.len() as u32;
        let header = QissHeader::new(total_size, checksum, 0);
        
        // Write header at the beginning
        let header_bytes = header.as_bytes();
        self.buffer[0..header_bytes.len()].copy_from_slice(header_bytes);

        Ok(self.buffer.split().freeze())
    }
}

impl Default for QissSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// High-performance Qiss deserializer
pub struct QissDeserializer;

impl QissDeserializer {
    /// Deserialize a value from Qiss format
    pub fn deserialize<T: QissDeserializable>(data: &[u8]) -> Result<T> {
        if data.len() < mem::size_of::<QissHeader>() {
            return Err(Error::InsufficientData);
        }

        // Parse header
        let header = QissHeader::read_from_prefix(data)
            .ok_or(Error::InvalidHeader)?;
        header.validate()?;

        // Verify checksum
        let body_start = mem::size_of::<QissHeader>();
        let body = &data[body_start..];
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(body);
        let calculated_checksum = hasher.finalize();
        if calculated_checksum != header.checksum {
            return Err(Error::ChecksumMismatch {
                expected: header.checksum,
                actual: calculated_checksum,
            });
        }

        // Deserialize the value
        let mut reader = QissReader::new(body);
        T::deserialize(&mut reader)
    }
}

/// Trait for types that can be serialized to Qiss format
pub trait QissSerializable {
    /// Serialize this value to Qiss format
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()>;
}

/// Trait for types that can be deserialized from Qiss format
pub trait QissDeserializable: Sized {
    /// Deserialize this value from Qiss format
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self>;
}

// Implement QissSerializable for basic types
impl QissSerializable for u8 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_u8(*self)
    }
}

impl QissDeserializable for u8 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_u8()
    }
}

impl QissSerializable for u16 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_u16(*self)
    }
}

impl QissDeserializable for u16 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_u16()
    }
}

impl QissSerializable for u32 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_u32(*self)
    }
}

impl QissDeserializable for u32 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_u32()
    }
}

impl QissSerializable for u64 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_u64(*self)
    }
}

impl QissDeserializable for u64 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_u64()
    }
}

impl QissSerializable for i8 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_i8(*self)
    }
}

impl QissDeserializable for i8 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_i8()
    }
}

impl QissSerializable for i16 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_i16(*self)
    }
}

impl QissDeserializable for i16 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_i16()
    }
}

impl QissSerializable for i32 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_i32(*self)
    }
}

impl QissDeserializable for i32 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_i32()
    }
}

impl QissSerializable for i64 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_i64(*self)
    }
}

impl QissDeserializable for i64 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_i64()
    }
}

impl QissSerializable for f32 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_f32(*self)
    }
}

impl QissDeserializable for f32 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_f32()
    }
}

impl QissSerializable for f64 {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_f64(*self)
    }
}

impl QissDeserializable for f64 {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_f64()
    }
}

impl QissSerializable for bool {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_bool(*self)
    }
}

impl QissDeserializable for bool {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_bool()
    }
}

impl QissSerializable for String {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_string(self)
    }
}

impl QissDeserializable for String {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        reader.read_string()
    }
}

// Byte array type to avoid conflicts
pub struct ByteArray(pub Vec<u8>);

impl QissSerializable for ByteArray {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_bytes(&self.0)
    }
}

impl QissDeserializable for ByteArray {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        Ok(ByteArray(reader.read_bytes()?))
    }
}

impl<T: QissSerializable> QissSerializable for Vec<T> 
where
    T: QissSerializable,
{
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        writer.write_u32(self.len() as u32)?;
        for item in self {
            item.serialize(writer)?;
        }
        Ok(())
    }
}

impl<T: QissDeserializable> QissDeserializable for Vec<T> 
where
    T: QissDeserializable,
{
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        let len = reader.read_u32()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::deserialize(reader)?);
        }
        Ok(vec)
    }
}

impl<T: QissSerializable> QissSerializable for Option<T> {
    fn serialize(&self, writer: &mut QissWriter<'_>) -> Result<()> {
        match self {
            Some(value) => {
                writer.write_bool(true)?;
                value.serialize(writer)
            }
            None => writer.write_bool(false),
        }
    }
}

impl<T: QissDeserializable> QissDeserializable for Option<T> {
    fn deserialize(reader: &mut QissReader<'_>) -> Result<Self> {
        if reader.read_bool()? {
            Ok(Some(T::deserialize(reader)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_serialization() {
        let mut serializer = QissSerializer::new();
        let data = serializer.serialize(&42u32).unwrap();
        
        let value: u32 = QissDeserializer::deserialize(&data).unwrap();
        
        assert_eq!(value, 42);
    }

    #[test]
    fn test_string_serialization() {
        let mut serializer = QissSerializer::new();
        let original = "Hello, Qiss!".to_string();
        let data = serializer.serialize(&original).unwrap();
        
        let deserialized: String = QissDeserializer::deserialize(&data).unwrap();
        
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_vec_serialization() {
        let mut serializer = QissSerializer::new();
        let original = vec![1u32, 2, 3, 4, 5];
        let data = serializer.serialize(&original).unwrap();
        
        let deserialized: Vec<u32> = QissDeserializer::deserialize(&data).unwrap();
        
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_option_serialization() {
        let mut serializer = QissSerializer::new();
        
        // Test Some
        let some_value = Some(42u32);
        let data = serializer.serialize(&some_value).unwrap();
        let deserialized: Option<u32> = QissDeserializer::deserialize(&data).unwrap();
        assert_eq!(deserialized, some_value);
        
        // Test None
        let none_value: Option<u32> = None;
        let data = serializer.serialize(&none_value).unwrap();
        let deserialized: Option<u32> = QissDeserializer::deserialize(&data).unwrap();
        assert_eq!(deserialized, none_value);
    }
}
