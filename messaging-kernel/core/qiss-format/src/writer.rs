//! Qiss format writer implementation

use bytes::{BytesMut, BufMut};
use crate::{Result, Error, types::Timestamp};

/// High-performance Qiss format writer
pub struct QissWriter<'a> {
    buffer: &'a mut BytesMut,
}

impl<'a> QissWriter<'a> {
    /// Create a new writer
    pub fn new(buffer: &'a mut BytesMut) -> Self {
        Self { buffer }
    }

    /// Write a u8 value
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.buffer.put_u8(value);
        Ok(())
    }

    /// Write a u16 value (little-endian)
    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        self.buffer.put_u16_le(value);
        Ok(())
    }

    /// Write a u32 value (little-endian)
    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.buffer.put_u32_le(value);
        Ok(())
    }

    /// Write a u64 value (little-endian)
    pub fn write_u64(&mut self, value: u64) -> Result<()> {
        self.buffer.put_u64_le(value);
        Ok(())
    }

    /// Write an i8 value
    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        self.buffer.put_i8(value);
        Ok(())
    }

    /// Write an i16 value (little-endian)
    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        self.buffer.put_i16_le(value);
        Ok(())
    }

    /// Write an i32 value (little-endian)
    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.buffer.put_i32_le(value);
        Ok(())
    }

    /// Write an i64 value (little-endian)
    pub fn write_i64(&mut self, value: i64) -> Result<()> {
        self.buffer.put_i64_le(value);
        Ok(())
    }

    /// Write an f32 value (little-endian)
    pub fn write_f32(&mut self, value: f32) -> Result<()> {
        self.buffer.put_f32_le(value);
        Ok(())
    }

    /// Write an f64 value (little-endian)
    pub fn write_f64(&mut self, value: f64) -> Result<()> {
        self.buffer.put_f64_le(value);
        Ok(())
    }

    /// Write a boolean value
    pub fn write_bool(&mut self, value: bool) -> Result<()> {
        self.buffer.put_u8(if value { 1 } else { 0 });
        Ok(())
    }

    /// Write a string value
    pub fn write_string(&mut self, value: &str) -> Result<()> {
        let bytes = value.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.buffer.put_slice(bytes);
        Ok(())
    }

    /// Write a byte array
    pub fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        self.write_u32(value.len() as u32)?;
        self.buffer.put_slice(value);
        Ok(())
    }

    /// Write a timestamp
    pub fn write_timestamp(&mut self, timestamp: Timestamp) -> Result<()> {
        self.write_u64(timestamp.as_nanos())
    }

    /// Write a slice of bytes directly
    pub fn write_slice(&mut self, slice: &[u8]) -> Result<()> {
        self.buffer.put_slice(slice);
        Ok(())
    }

    /// Get the current position in the buffer
    pub fn position(&self) -> usize {
        self.buffer.len()
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}