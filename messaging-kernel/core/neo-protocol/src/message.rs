//! Message types and structures for Neo Protocol

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use bytes::{Bytes, BytesMut, BufMut};
use serde::{Deserialize, Serialize};
use crc32fast::Hasher;

use crate::{Result, Error, MessageType, NeoHeader, NeoContext};

/// Neo Protocol message
#[derive(Debug, Clone)]
pub struct NeoMessage {
    /// Message header
    pub header: NeoHeader,
    /// Message payload
    pub payload: Bytes,
}

impl NeoMessage {
    /// Create a new message
    pub fn new(message_type: MessageType, correlation_id: u64, payload: Bytes) -> Self {
        let payload_size = payload.len() as u32;
        let mut header = NeoHeader::new(message_type, correlation_id, payload_size);
        
        // Calculate checksum
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        header.checksum = hasher.finalize();
        
        Self { header, payload }
    }

    /// Validate the message
    pub fn validate(&self) -> Result<()> {
        self.header.validate()?;
        
        // Verify payload size
        if self.payload.len() != self.header.payload_size as usize {
            return Err(Error::InsufficientData {
                needed: self.header.payload_size as usize,
                got: self.payload.len(),
            });
        }
        
        // Verify checksum
        let mut hasher = Hasher::new();
        hasher.update(&self.payload);
        let calculated_checksum = hasher.finalize();
        if calculated_checksum != self.header.checksum {
            return Err(Error::ChecksumMismatch {
                expected: self.header.checksum,
                actual: calculated_checksum,
            });
        }
        
        Ok(())
    }

    /// Serialize the message to bytes
    pub fn serialize(&self) -> Result<Bytes> {
        let mut buffer = BytesMut::with_capacity(
            std::mem::size_of::<NeoHeader>() + self.payload.len()
        );
        
        // Serialize header
        buffer.put_u32(self.header.magic);
        buffer.put_u8(self.header.version);
        buffer.put_u8(self.header.message_type.to_byte());
        buffer.put_u64(self.header.correlation_id);
        buffer.put_u32(self.header.payload_size);
        buffer.put_u64(self.header.timestamp);
        buffer.put_u32(self.header.checksum);
        
        // Add payload
        buffer.put_slice(&self.payload);
        
        Ok(buffer.freeze())
    }

    /// Deserialize a message from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < std::mem::size_of::<NeoHeader>() {
            return Err(Error::InsufficientData {
                needed: std::mem::size_of::<NeoHeader>(),
                got: data.len(),
            });
        }

        let mut offset = 0;
        
        // Parse header
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
        offset += 4;
        
        let header = NeoHeader {
            magic,
            version,
            message_type,
            correlation_id,
            payload_size,
            timestamp,
            checksum,
        };
        
        // Extract payload
        let payload = Bytes::copy_from_slice(&data[offset..offset + payload_size as usize]);
        
        let message = Self { header, payload };
        message.validate()?;
        
        Ok(message)
    }
}

/// RPC request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    /// Service name
    pub service_name: String,
    /// Method name
    pub method_name: String,
    /// Request parameters (serialized)
    pub params: Bytes,
    /// Request timeout
    pub timeout: Option<Duration>,
    /// Client information
    pub client_info: Option<ClientInfo>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// RPC response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    /// Response data (serialized)
    pub data: Bytes,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Processing time
    pub processing_time: Duration,
}

/// RPC error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    /// Error code
    pub code: u32,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<Bytes>,
    /// Error metadata
    pub metadata: HashMap<String, String>,
}

/// Event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    /// Event topic
    pub topic: String,
    /// Event data (serialized)
    pub data: Bytes,
    /// Event metadata
    pub metadata: HashMap<String, String>,
    /// Partition key for routing
    pub partition_key: Option<String>,
    /// Event timestamp
    pub timestamp: SystemTime,
}

/// Heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// Client ID
    pub client_id: String,
    /// Server timestamp
    pub server_time: SystemTime,
    /// Client timestamp
    pub client_time: SystemTime,
    /// Additional heartbeat data
    pub data: Option<Bytes>,
}

/// Authentication request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    /// Authentication method
    pub method: String,
    /// Authentication credentials
    pub credentials: Bytes,
    /// Client capabilities
    pub capabilities: Vec<String>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// Authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Authentication success
    pub success: bool,
    /// Authentication token
    pub token: Option<String>,
    /// Token expiration time
    pub expires_at: Option<SystemTime>,
    /// Server capabilities
    pub server_capabilities: Vec<String>,
    /// Error message if authentication failed
    pub error: Option<String>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client ID
    pub client_id: String,
    /// Client version
    pub version: String,
    /// Client capabilities
    pub capabilities: Vec<String>,
    /// Client metadata
    pub metadata: HashMap<String, String>,
}

/// Message builder for creating Neo messages
pub struct MessageBuilder {
    message_type: MessageType,
    correlation_id: u64,
    payload: BytesMut,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(message_type: MessageType, correlation_id: u64) -> Self {
        Self {
            message_type,
            correlation_id,
            payload: BytesMut::new(),
        }
    }

    /// Add payload data
    pub fn payload(mut self, data: &[u8]) -> Self {
        self.payload.put_slice(data);
        self
    }

    /// Add serialized payload
    pub fn serialized_payload<T: Serialize>(mut self, value: &T) -> Result<Self> {
        let serialized = bincode::serialize(value)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        self.payload.put_slice(&serialized);
        Ok(self)
    }

    /// Build the message
    pub fn build(self) -> Result<NeoMessage> {
        let payload = self.payload.freeze();
        Ok(NeoMessage::new(self.message_type, self.correlation_id, payload))
    }
}

/// Message parser for extracting data from Neo messages
pub struct MessageParser<'a> {
    message: &'a NeoMessage,
}

impl<'a> MessageParser<'a> {
    /// Create a new message parser
    pub fn new(message: &'a NeoMessage) -> Self {
        Self { message }
    }

    /// Parse the payload as a specific type
    pub fn parse<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        bincode::deserialize(&self.message.payload)
            .map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Get the raw payload
    pub fn payload(&self) -> &Bytes {
        &self.message.payload
    }

    /// Get the message header
    pub fn header(&self) -> &NeoHeader {
        &self.message.header
    }
}

/// Message factory for creating common message types
pub struct MessageFactory;

impl MessageFactory {
    /// Create an RPC request message
    pub fn rpc_request(
        correlation_id: u64,
        service_name: String,
        method_name: String,
        params: Bytes,
    ) -> Result<NeoMessage> {
        let request = RpcRequest {
            service_name,
            method_name,
            params,
            timeout: None,
            client_info: None,
            metadata: HashMap::new(),
        };
        
        MessageBuilder::new(MessageType::RpcRequest, correlation_id)
            .serialized_payload(&request)?
            .build()
    }

    /// Create an RPC response message
    pub fn rpc_response(
        correlation_id: u64,
        data: Bytes,
        processing_time: Duration,
    ) -> Result<NeoMessage> {
        let response = RpcResponse {
            data,
            metadata: HashMap::new(),
            processing_time,
        };
        
        MessageBuilder::new(MessageType::RpcResponse, correlation_id)
            .serialized_payload(&response)?
            .build()
    }

    /// Create an RPC error message
    pub fn rpc_error(
        correlation_id: u64,
        code: u32,
        message: String,
        details: Option<Bytes>,
    ) -> Result<NeoMessage> {
        let error = RpcError {
            code,
            message,
            details,
            metadata: HashMap::new(),
        };
        
        MessageBuilder::new(MessageType::RpcError, correlation_id)
            .serialized_payload(&error)?
            .build()
    }

    /// Create an event message
    pub fn event(
        topic: String,
        data: Bytes,
        partition_key: Option<String>,
    ) -> Result<NeoMessage> {
        let event = EventMessage {
            topic,
            data,
            metadata: HashMap::new(),
            partition_key,
            timestamp: SystemTime::now(),
        };
        
        MessageBuilder::new(MessageType::Event, 0) // Events don't need correlation ID
            .serialized_payload(&event)?
            .build()
    }

    /// Create a heartbeat message
    pub fn heartbeat(client_id: String) -> Result<NeoMessage> {
        let heartbeat = HeartbeatMessage {
            client_id,
            server_time: SystemTime::now(),
            client_time: SystemTime::now(),
            data: None,
        };
        
        MessageBuilder::new(MessageType::Heartbeat, 0)
            .serialized_payload(&heartbeat)?
            .build()
    }

    /// Create an authentication request
    pub fn auth_request(
        method: String,
        credentials: Bytes,
        capabilities: Vec<String>,
    ) -> Result<NeoMessage> {
        let auth_request = AuthRequest {
            method,
            credentials,
            capabilities,
            metadata: HashMap::new(),
        };
        
        MessageBuilder::new(MessageType::AuthRequest, 0)
            .serialized_payload(&auth_request)?
            .build()
    }

    /// Create an authentication response
    pub fn auth_response(
        success: bool,
        token: Option<String>,
        expires_at: Option<SystemTime>,
        server_capabilities: Vec<String>,
        error: Option<String>,
    ) -> Result<NeoMessage> {
        let auth_response = AuthResponse {
            success,
            token,
            expires_at,
            server_capabilities,
            error,
        };
        
        MessageBuilder::new(MessageType::AuthResponse, 0)
            .serialized_payload(&auth_response)?
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation_and_validation() {
        let payload = Bytes::from("test payload");
        let message = NeoMessage::new(MessageType::RpcRequest, 123, payload.clone());
        
        assert_eq!(message.header.message_type, MessageType::RpcRequest);
        assert_eq!(message.header.correlation_id, 123);
        assert_eq!(message.payload, payload);
        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_message_serialization() {
        let payload = Bytes::from("test payload");
        let message = NeoMessage::new(MessageType::RpcRequest, 123, payload);
        
        let serialized = message.serialize().unwrap();
        let deserialized = NeoMessage::deserialize(&serialized).unwrap();
        
        assert_eq!(message.header, deserialized.header);
        assert_eq!(message.payload, deserialized.payload);
    }

    #[test]
    fn test_rpc_request_creation() {
        let params = Bytes::from("test params");
        let message = MessageFactory::rpc_request(
            123,
            "test_service".to_string(),
            "test_method".to_string(),
            params,
        ).unwrap();
        
        assert_eq!(message.header.message_type, MessageType::RpcRequest);
        assert_eq!(message.header.correlation_id, 123);
    }

    #[test]
    fn test_rpc_response_creation() {
        let data = Bytes::from("test response");
        let message = MessageFactory::rpc_response(123, data, Duration::from_millis(100)).unwrap();
        
        assert_eq!(message.header.message_type, MessageType::RpcResponse);
        assert_eq!(message.header.correlation_id, 123);
    }

    #[test]
    fn test_event_creation() {
        let data = Bytes::from("test event");
        let message = MessageFactory::event(
            "test_topic".to_string(),
            data,
            Some("partition_key".to_string()),
        ).unwrap();
        
        assert_eq!(message.header.message_type, MessageType::Event);
    }

    #[test]
    fn test_heartbeat_creation() {
        let message = MessageFactory::heartbeat("test_client".to_string()).unwrap();
        
        assert_eq!(message.header.message_type, MessageType::Heartbeat);
    }

    #[test]
    fn test_message_parser() {
        let request = RpcRequest {
            service_name: "test_service".to_string(),
            method_name: "test_method".to_string(),
            params: Bytes::from("test params"),
            timeout: None,
            client_info: None,
            metadata: HashMap::new(),
        };
        
        let message = MessageBuilder::new(MessageType::RpcRequest, 123)
            .serialized_payload(&request)
            .unwrap()
            .build()
            .unwrap();
        
        let parser = MessageParser::new(&message);
        let parsed_request: RpcRequest = parser.parse().unwrap();
        
        assert_eq!(parsed_request.service_name, "test_service");
        assert_eq!(parsed_request.method_name, "test_method");
    }
}