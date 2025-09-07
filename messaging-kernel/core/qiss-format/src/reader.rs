//! Qiss format reader implementation

use bytes::{Bytes, Buf};
use crate::{Result, Error, types::Timestamp};

/// High-performance Qiss format reader
pub struct QissReader<'a> {
    buffer: &'a [u8],
    position: usize,
}

impl<'a> QissReader<'a> {
    /// Create a new reader
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }

    /// Read a u8 value
    pub fn read_u8(&mut self) -> Result<u8> {
        if self.position >= self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = self.buffer[self.position];
        self.position += 1;
        Ok(value)
    }

    /// Read a u16 value (little-endian)
    pub fn read_u16(&mut self) -> Result<u16> {
        if self.position + 2 > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = u16::from_le_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
        ]);
        self.position += 2;
        Ok(value)
    }

    /// Read a u32 value (little-endian)
    pub fn read_u32(&mut self) -> Result<u32> {
        if self.position + 4 > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = u32::from_le_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);
        self.position += 4;
        Ok(value)
    }

    /// Read a u64 value (little-endian)
    pub fn read_u64(&mut self) -> Result<u64> {
        if self.position + 8 > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = u64::from_le_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
            self.buffer[self.position + 4],
            self.buffer[self.position + 5],
            self.buffer[self.position + 6],
            self.buffer[self.position + 7],
        ]);
        self.position += 8;
        Ok(value)
    }

    /// Read an i8 value
    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    /// Read an i16 value (little-endian)
    pub fn read_i16(&mut self) -> Result<i16> {
        Ok(self.read_u16()? as i16)
    }

    /// Read an i32 value (little-endian)
    pub fn read_i32(&mut self) -> Result<i32> {
        Ok(self.read_u32()? as i32)
    }

    /// Read an i64 value (little-endian)
    pub fn read_i64(&mut self) -> Result<i64> {
        Ok(self.read_u64()? as i64)
    }

    /// Read an f32 value (little-endian)
    pub fn read_f32(&mut self) -> Result<f32> {
        if self.position + 4 > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = f32::from_le_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ]);
        self.position += 4;
        Ok(value)
    }

    /// Read an f64 value (little-endian)
    pub fn read_f64(&mut self) -> Result<f64> {
        if self.position + 8 > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let value = f64::from_le_bytes([
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
            self.buffer[self.position + 4],
            self.buffer[self.position + 5],
            self.buffer[self.position + 6],
            self.buffer[self.position + 7],
        ]);
        self.position += 8;
        Ok(value)
    }

    /// Read a boolean value
    pub fn read_bool(&mut self) -> Result<bool> {
        let value = self.read_u8()?;
        match value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::InvalidFieldType),
        }
    }

    /// Read a string value
    pub fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        if self.position + len > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let bytes = &self.buffer[self.position..self.position + len];
        self.position += len;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| Error::InvalidUtf8)
    }

    /// Read a byte array
    pub fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.read_u32()? as usize;
        if self.position + len > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let bytes = self.buffer[self.position..self.position + len].to_vec();
        self.position += len;
        Ok(bytes)
    }

    /// Read a timestamp
    pub fn read_timestamp(&mut self) -> Result<Timestamp> {
        let nanos = self.read_u64()?;
        Ok(Timestamp::from_nanos(nanos))
    }

    /// Read a slice of bytes
    pub fn read_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.position + len > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        let slice = &self.buffer[self.position..self.position + len];
        self.position += len;
        Ok(slice)
    }

    /// Get the current position in the buffer
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get the remaining bytes in the buffer
    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.position
    }

    /// Check if there are more bytes to read
    pub fn has_remaining(&self) -> bool {
        self.position < self.buffer.len()
    }

    /// Skip a number of bytes
    pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.position + len > self.buffer.len() {
            return Err(Error::InsufficientData);
        }
        self.position += len;
        Ok(())
    }
}