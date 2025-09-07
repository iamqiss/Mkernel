//! Qiss format codec for async operations

use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use crate::{Result, QissSerializable, QissDeserializable, QissSerializer, QissDeserializer};

/// Async codec for Qiss format
pub struct QissCodec;

impl QissCodec {
    /// Encode a value to Qiss format asynchronously
    pub async fn encode<T: QissSerializable>(
        &self,
        value: &T,
        writer: &mut (impl AsyncWrite + Unpin),
    ) -> Result<()> {
        let mut serializer = QissSerializer::new();
        let data = serializer.serialize(value)?;
        
        // Write length prefix
        writer.write_all(&(data.len() as u32).to_le_bytes()).await?;
        writer.write_all(&data).await?;
        writer.flush().await?;
        
        Ok(())
    }

    /// Decode a value from Qiss format asynchronously
    pub async fn decode<T: QissDeserializable>(
        &self,
        reader: &mut (impl AsyncRead + Unpin),
    ) -> Result<T> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Read the data
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;

        // Deserialize
        QissDeserializer::deserialize(&data)
    }

    /// Encode multiple values in a batch
    pub async fn encode_batch<T: QissSerializable>(
        &self,
        values: &[T],
        writer: &mut (impl AsyncWrite + Unpin),
    ) -> Result<()> {
        // Write batch size
        writer.write_all(&(values.len() as u32).to_le_bytes()).await?;
        
        // Encode each value
        for value in values {
            self.encode(value, writer).await?;
        }
        
        Ok(())
    }

    /// Decode multiple values from a batch
    pub async fn decode_batch<T: QissDeserializable>(
        &self,
        reader: &mut (impl AsyncRead + Unpin),
    ) -> Result<Vec<T>> {
        // Read batch size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes).await?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        // Decode each value
        let mut values = Vec::with_capacity(size);
        for _ in 0..size {
            values.push(self.decode(reader).await?);
        }
        
        Ok(values)
    }
}

/// Synchronous codec for Qiss format
pub struct QissSyncCodec;

impl QissSyncCodec {
    /// Encode a value to Qiss format synchronously
    pub fn encode<T: QissSerializable>(
        &self,
        value: &T,
        writer: &mut impl std::io::Write,
    ) -> Result<()> {
        let mut serializer = QissSerializer::new();
        let data = serializer.serialize(value)?;
        
        // Write length prefix
        writer.write_all(&(data.len() as u32).to_le_bytes())?;
        writer.write_all(&data)?;
        writer.flush()?;
        
        Ok(())
    }

    /// Decode a value from Qiss format synchronously
    pub fn decode<T: QissDeserializable>(
        &self,
        reader: &mut impl std::io::Read,
    ) -> Result<T> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes) as usize;

        // Read the data
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data)?;

        // Deserialize
        QissDeserializer::deserialize(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_async_codec() {
        let codec = QissCodec;
        let original = vec![1u32, 2, 3, 4, 5];
        
        let mut buffer = Vec::new();
        codec.encode(&original, &mut buffer).await.unwrap();
        
        let mut cursor = Cursor::new(&buffer);
        let decoded: Vec<u32> = codec.decode(&mut cursor).await.unwrap();
        
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_sync_codec() {
        let codec = QissSyncCodec;
        let original = "Hello, Qiss!".to_string();
        
        let mut buffer = Vec::new();
        codec.encode(&original, &mut buffer).unwrap();
        
        let mut cursor = Cursor::new(&buffer);
        let decoded: String = codec.decode(&mut cursor).unwrap();
        
        assert_eq!(decoded, original);
    }
}