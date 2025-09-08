//! Codec for Neo Protocol message encoding/decoding

use std::io::{self, Read, Write};
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use bytes::{Bytes, BytesMut, BufMut};
use tracing::{debug, error, warn};

use crate::{Result, Error, NeoMessage, MessageType, NeoHeader};

/// Neo Protocol codec for synchronous operations
pub struct NeoCodec;

impl NeoCodec {
    /// Encode a message to bytes
    pub fn encode(message: &NeoMessage) -> Result<Bytes> {
        message.serialize()
    }

    /// Decode a message from bytes
    pub fn decode(data: &[u8]) -> Result<NeoMessage> {
        NeoMessage::deserialize(data)
    }

    /// Encode a message to a writer
    pub fn encode_to_writer<W: Write>(message: &NeoMessage, writer: &mut W) -> Result<()> {
        let data = Self::encode(message)?;
        writer.write_all(&data)?;
        writer.flush()?;
        Ok(())
    }

    /// Decode a message from a reader
    pub fn decode_from_reader<R: Read>(reader: &mut R) -> Result<NeoMessage> {
        // Read header first
        let mut header_bytes = [0u8; std::mem::size_of::<NeoHeader>()];
        reader.read_exact(&mut header_bytes)?;
        
        // Parse header to get payload size
        let header = Self::parse_header(&header_bytes)?;
        let total_size = std::mem::size_of::<NeoHeader>() + header.payload_size as usize;
        
        // Read the complete message
        let mut message_bytes = vec![0u8; total_size];
        message_bytes[..std::mem::size_of::<NeoHeader>()].copy_from_slice(&header_bytes);
        reader.read_exact(&mut message_bytes[std::mem::size_of::<NeoHeader>()..])?;
        
        Self::decode(&message_bytes)
    }

    /// Parse header from bytes
    fn parse_header(data: &[u8]) -> Result<NeoHeader> {
        if data.len() < std::mem::size_of::<NeoHeader>() {
            return Err(Error::InsufficientData {
                needed: std::mem::size_of::<NeoHeader>(),
                got: data.len(),
            });
        }

        let mut offset = 0;
        
        let magic = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]);
        offset += 4;
        
        let version = data[offset];
        offset += 1;
        
        let message_type = MessageType::from_byte(data[offset])?;
        offset += 1;
        
        let correlation_id = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        let payload_size = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]);
        offset += 4;
        
        let timestamp = u64::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
            data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]
        ]);
        offset += 8;
        
        let checksum = u32::from_le_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]);
        
        Ok(NeoHeader {
            magic,
            version,
            message_type,
            correlation_id,
            payload_size,
            timestamp,
            checksum,
        })
    }
}

/// Async Neo Protocol codec
pub struct AsyncNeoCodec;

impl AsyncNeoCodec {
    /// Encode a message to bytes asynchronously
    pub async fn encode(message: &NeoMessage) -> Result<Bytes> {
        NeoCodec::encode(message)
    }

    /// Decode a message from bytes asynchronously
    pub async fn decode(data: &[u8]) -> Result<NeoMessage> {
        NeoCodec::decode(data)
    }

    /// Encode a message to an async writer
    pub async fn encode_to_writer<W: AsyncWrite + Unpin>(
        message: &NeoMessage,
        writer: &mut W,
    ) -> Result<()> {
        let data = Self::encode(message).await?;
        writer.write_all(&data).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Decode a message from an async reader
    pub async fn decode_from_reader<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<NeoMessage> {
        // Read header first
        let mut header_bytes = [0u8; std::mem::size_of::<NeoHeader>()];
        reader.read_exact(&mut header_bytes).await?;
        
        // Parse header to get payload size
        let header = NeoCodec::parse_header(&header_bytes)?;
        let total_size = std::mem::size_of::<NeoHeader>() + header.payload_size as usize;
        
        // Read the complete message
        let mut message_bytes = vec![0u8; total_size];
        message_bytes[..std::mem::size_of::<NeoHeader>()].copy_from_slice(&header_bytes);
        reader.read_exact(&mut message_bytes[std::mem::size_of::<NeoHeader>()..]).await?;
        
        Self::decode(&message_bytes).await
    }

    /// Encode multiple messages in a batch
    pub async fn encode_batch<W: AsyncWrite + Unpin>(
        messages: &[NeoMessage],
        writer: &mut W,
    ) -> Result<()> {
        // Write batch size
        writer.write_all(&(messages.len() as u32).to_le_bytes()).await?;
        
        // Encode each message
        for message in messages {
            Self::encode_to_writer(message, writer).await?;
        }
        
        Ok(())
    }

    /// Decode multiple messages from a batch
    pub async fn decode_batch<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Vec<NeoMessage>> {
        // Read batch size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes).await?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        // Decode each message
        let mut messages = Vec::with_capacity(size);
        for _ in 0..size {
            messages.push(Self::decode_from_reader(reader).await?);
        }
        
        Ok(messages)
    }
}

/// Framed codec that handles message framing
pub struct FramedNeoCodec;

impl FramedNeoCodec {
    /// Encode a message with length prefix
    pub fn encode_framed(message: &NeoMessage) -> Result<Bytes> {
        let data = NeoCodec::encode(message)?;
        let mut framed = BytesMut::with_capacity(4 + data.len());
        framed.put_u32(data.len() as u32);
        framed.put_slice(&data);
        Ok(framed.freeze())
    }

    /// Decode a framed message
    pub fn decode_framed(data: &[u8]) -> Result<NeoMessage> {
        if data.len() < 4 {
            return Err(Error::InsufficientData {
                needed: 4,
                got: data.len(),
            });
        }

        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 4 + length {
            return Err(Error::InsufficientData {
                needed: 4 + length,
                got: data.len(),
            });
        }

        NeoCodec::decode(&data[4..4 + length])
    }

    /// Encode a framed message to a writer
    pub fn encode_framed_to_writer<W: Write>(message: &NeoMessage, writer: &mut W) -> Result<()> {
        let framed_data = Self::encode_framed(message)?;
        writer.write_all(&framed_data)?;
        writer.flush()?;
        Ok(())
    }

    /// Decode a framed message from a reader
    pub fn decode_framed_from_reader<R: Read>(reader: &mut R) -> Result<NeoMessage> {
        // Read length prefix
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes)?;
        let length = u32::from_le_bytes(length_bytes) as usize;

        // Read the message data
        let mut message_data = vec![0u8; length];
        reader.read_exact(&mut message_data)?;

        NeoCodec::decode(&message_data)
    }
}

/// Async framed codec
pub struct AsyncFramedNeoCodec;

impl AsyncFramedNeoCodec {
    /// Encode a message with length prefix asynchronously
    pub async fn encode_framed(message: &NeoMessage) -> Result<Bytes> {
        FramedNeoCodec::encode_framed(message)
    }

    /// Decode a framed message asynchronously
    pub async fn decode_framed(data: &[u8]) -> Result<NeoMessage> {
        FramedNeoCodec::decode_framed(data)
    }

    /// Encode a framed message to an async writer
    pub async fn encode_framed_to_writer<W: AsyncWrite + Unpin>(
        message: &NeoMessage,
        writer: &mut W,
    ) -> Result<()> {
        let framed_data = Self::encode_framed(message).await?;
        writer.write_all(&framed_data).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Decode a framed message from an async reader
    pub async fn decode_framed_from_reader<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<NeoMessage> {
        // Read length prefix
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes).await?;
        let length = u32::from_le_bytes(length_bytes) as usize;

        // Read the message data
        let mut message_data = vec![0u8; length];
        reader.read_exact(&mut message_data).await?;

        AsyncNeoCodec::decode(&message_data).await
    }

    /// Encode multiple framed messages in a batch
    pub async fn encode_framed_batch<W: AsyncWrite + Unpin>(
        messages: &[NeoMessage],
        writer: &mut W,
    ) -> Result<()> {
        // Write batch size
        writer.write_all(&(messages.len() as u32).to_le_bytes()).await?;
        
        // Encode each message with framing
        for message in messages {
            Self::encode_framed_to_writer(message, writer).await?;
        }
        
        Ok(())
    }

    /// Decode multiple framed messages from a batch
    pub async fn decode_framed_batch<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Vec<NeoMessage>> {
        // Read batch size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes).await?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        // Decode each framed message
        let mut messages = Vec::with_capacity(size);
        for _ in 0..size {
            messages.push(Self::decode_framed_from_reader(reader).await?);
        }
        
        Ok(messages)
    }
}

/// Codec with compression support
pub struct CompressedNeoCodec {
    /// Compression level (0-9, where 0 is no compression and 9 is maximum compression)
    pub compression_level: u32,
}

impl CompressedNeoCodec {
    /// Create a new compressed codec
    pub fn new(compression_level: u32) -> Self {
        Self {
            compression_level: compression_level.min(9),
        }
    }

    /// Encode a message with compression
    pub fn encode_compressed(message: &NeoMessage) -> Result<Bytes> {
        let data = NeoCodec::encode(message)?;
        
        // For now, we'll use a simple compression flag
        // In a real implementation, you'd use a compression library like flate2
        let mut compressed = BytesMut::with_capacity(data.len() + 1);
        compressed.put_u8(0); // Compression flag: 0 = no compression
        compressed.put_slice(&data);
        
        Ok(compressed.freeze())
    }

    /// Decode a compressed message
    pub fn decode_compressed(data: &[u8]) -> Result<NeoMessage> {
        if data.is_empty() {
            return Err(Error::InsufficientData {
                needed: 1,
                got: 0,
            });
        }

        let compression_flag = data[0];
        let message_data = &data[1..];

        match compression_flag {
            0 => NeoCodec::decode(message_data), // No compression
            _ => Err(Error::Compression(format!("Unsupported compression flag: {}", compression_flag))),
        }
    }
}

/// Codec with encryption support
pub struct EncryptedNeoCodec;

impl EncryptedNeoCodec {
    /// Encode a message with encryption
    pub fn encode_encrypted(message: &NeoMessage, key: &[u8]) -> Result<Bytes> {
        let data = NeoCodec::encode(message)?;
        
        // For now, we'll use a simple XOR encryption as a placeholder
        // In a real implementation, you'd use a proper encryption library
        let mut encrypted = BytesMut::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            encrypted.put_u8(byte ^ key[i % key.len()]);
        }
        
        Ok(encrypted.freeze())
    }

    /// Decode an encrypted message
    pub fn decode_encrypted(data: &[u8], key: &[u8]) -> Result<NeoMessage> {
        // Simple XOR decryption (same as encryption for XOR)
        let mut decrypted = BytesMut::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            decrypted.put_u8(byte ^ key[i % key.len()]);
        }
        
        NeoCodec::decode(&decrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageFactory;
    use std::io::Cursor;

    #[test]
    fn test_basic_codec() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let encoded = NeoCodec::encode(&message).unwrap();
        let decoded = NeoCodec::decode(&encoded).unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }

    #[test]
    fn test_framed_codec() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let encoded = FramedNeoCodec::encode_framed(&message).unwrap();
        let decoded = FramedNeoCodec::decode_framed(&encoded).unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }

    #[test]
    fn test_codec_with_writer_reader() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let mut buffer = Vec::new();
        NeoCodec::encode_to_writer(&message, &mut buffer).unwrap();

        let mut cursor = Cursor::new(&buffer);
        let decoded = NeoCodec::decode_from_reader(&mut cursor).unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }

    #[tokio::test]
    async fn test_async_codec() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let encoded = AsyncNeoCodec::encode(&message).await.unwrap();
        let decoded = AsyncNeoCodec::decode(&encoded).await.unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }

    #[test]
    fn test_compressed_codec() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let encoded = CompressedNeoCodec::encode_compressed(&message).unwrap();
        let decoded = CompressedNeoCodec::decode_compressed(&encoded).unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }

    #[test]
    fn test_encrypted_codec() {
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            bytes::Bytes::from("test params"),
        ).unwrap();

        let key = b"test_key_123456789012345678901234567890";
        let encoded = EncryptedNeoCodec::encode_encrypted(&message, key).unwrap();
        let decoded = EncryptedNeoCodec::decode_encrypted(&encoded, key).unwrap();

        assert_eq!(message.header, decoded.header);
        assert_eq!(message.payload, decoded.payload);
    }
}