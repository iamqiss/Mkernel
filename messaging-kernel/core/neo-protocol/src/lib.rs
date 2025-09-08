//! High-performance RPC protocol implementation
//! 
//! This crate is part of the Neo Messaging Kernel, providing high-performance rpc protocol implementation.
//! 
//! The Neo Protocol is designed for maximum throughput and minimum latency, supporting both synchronous
//! RPC calls and asynchronous messaging from a single service definition.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc, oneshot};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

pub mod error;
pub mod message;
pub mod service;
pub mod client;
pub mod server;
pub mod codec;
pub mod security;
pub mod monitoring;

pub use error::Error;
pub use message::*;
pub use service::*;
pub use client::*;
pub use server::*;
pub use codec::*;
pub use security::*;
pub use monitoring::*;

/// Result type for this crate
pub type Result<T> = std::result::Result<T, Error>;

/// Neo Protocol version
pub const NEO_PROTOCOL_VERSION: u8 = 1;

/// Magic number for Neo Protocol identification
pub const NEO_MAGIC: u32 = 0x4E454F00; // "NEO\0"

/// Maximum message size (10MB)
pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Default timeout for RPC calls
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for connection establishment
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Neo Protocol header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeoHeader {
    /// Magic number for protocol identification
    pub magic: u32,
    /// Protocol version
    pub version: u8,
    /// Message type and flags
    pub message_type: MessageType,
    /// Request/Response ID for correlation
    pub correlation_id: u64,
    /// Message payload size
    pub payload_size: u32,
    /// Timestamp (nanoseconds since Unix epoch)
    pub timestamp: u64,
    /// CRC32 checksum of the payload
    pub checksum: u32,
}

impl NeoHeader {
    /// Create a new Neo header
    pub fn new(message_type: MessageType, correlation_id: u64, payload_size: u32) -> Self {
        Self {
            magic: NEO_MAGIC,
            version: NEO_PROTOCOL_VERSION,
            message_type,
            correlation_id,
            payload_size,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            checksum: 0, // Will be calculated when serializing
        }
    }

    /// Validate the header
    pub fn validate(&self) -> Result<()> {
        if self.magic != NEO_MAGIC {
            return Err(Error::InvalidMagic(self.magic));
        }
        if self.version != NEO_PROTOCOL_VERSION {
            return Err(Error::UnsupportedVersion(self.version));
        }
        if self.payload_size > MAX_MESSAGE_SIZE as u32 {
            return Err(Error::MessageTooLarge(self.payload_size as usize, MAX_MESSAGE_SIZE));
        }
        Ok(())
    }
}

/// Message type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    /// RPC request
    RpcRequest = 0x01,
    /// RPC response
    RpcResponse = 0x02,
    /// RPC error response
    RpcError = 0x03,
    /// Event message
    Event = 0x04,
    /// Heartbeat message
    Heartbeat = 0x05,
    /// Authentication request
    AuthRequest = 0x06,
    /// Authentication response
    AuthResponse = 0x07,
    /// Connection close
    Close = 0x08,
}

impl MessageType {
    /// Convert to byte representation
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Parse from byte representation
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0x01 => Ok(MessageType::RpcRequest),
            0x02 => Ok(MessageType::RpcResponse),
            0x03 => Ok(MessageType::RpcError),
            0x04 => Ok(MessageType::Event),
            0x05 => Ok(MessageType::Heartbeat),
            0x06 => Ok(MessageType::AuthRequest),
            0x07 => Ok(MessageType::AuthResponse),
            0x08 => Ok(MessageType::Close),
            _ => Err(Error::InvalidMessageType(byte)),
        }
    }
}

/// Neo Protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoConfig {
    /// Maximum message size
    pub max_message_size: usize,
    /// Default RPC timeout
    pub rpc_timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Enable compression
    pub enable_compression: bool,
    /// Security configuration
    pub security: SecurityConfig,
}

impl Default for NeoConfig {
    fn default() -> Self {
        Self {
            max_message_size: MAX_MESSAGE_SIZE,
            rpc_timeout: DEFAULT_RPC_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            heartbeat_interval: Duration::from_secs(30),
            max_concurrent_requests: 1000,
            enable_compression: false,
            security: SecurityConfig::default(),
        }
    }
}

/// Neo Protocol context
#[derive(Debug, Clone)]
pub struct NeoContext {
    /// Request correlation ID
    pub correlation_id: u64,
    /// Service name
    pub service_name: String,
    /// Method name
    pub method_name: String,
    /// Request timestamp
    pub timestamp: SystemTime,
    /// Client information
    pub client_info: Option<ClientInfo>,
    /// Security context
    pub security_context: Option<SecurityContext>,
}

impl NeoContext {
    /// Create a new context
    pub fn new(correlation_id: u64, service_name: String, method_name: String) -> Self {
        Self {
            correlation_id,
            service_name,
            method_name,
            timestamp: SystemTime::now(),
            client_info: None,
            security_context: None,
        }
    }
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
}

/// Neo Protocol statistics
#[derive(Debug, Default)]
pub struct NeoStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Total responses sent
    pub total_responses: u64,
    /// Total errors
    pub total_errors: u64,
    /// Total events published
    pub total_events: u64,
    /// Average request latency (microseconds)
    pub avg_latency_us: u64,
    /// Current active connections
    pub active_connections: usize,
    /// Current pending requests
    pub pending_requests: usize,
}

impl NeoStats {
    /// Update statistics with a new request
    pub fn record_request(&mut self, latency_us: u64) {
        self.total_requests += 1;
        self.avg_latency_us = (self.avg_latency_us * (self.total_requests - 1) + latency_us) / self.total_requests;
    }

    /// Record a response
    pub fn record_response(&mut self) {
        self.total_responses += 1;
    }

    /// Record an error
    pub fn record_error(&mut self) {
        self.total_errors += 1;
    }

    /// Record an event
    pub fn record_event(&mut self) {
        self.total_events += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_creation() {
        let header = NeoHeader::new(MessageType::RpcRequest, 123, 1000);
        assert_eq!(header.magic, NEO_MAGIC);
        assert_eq!(header.version, NEO_PROTOCOL_VERSION);
        assert_eq!(header.message_type, MessageType::RpcRequest);
        assert_eq!(header.correlation_id, 123);
        assert_eq!(header.payload_size, 1000);
    }

    #[test]
    fn test_header_validation() {
        let mut header = NeoHeader::new(MessageType::RpcRequest, 123, 1000);
        assert!(header.validate().is_ok());

        header.magic = 0x12345678;
        assert!(header.validate().is_err());

        header.magic = NEO_MAGIC;
        header.payload_size = MAX_MESSAGE_SIZE as u32 + 1;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_message_type_conversion() {
        for msg_type in [
            MessageType::RpcRequest,
            MessageType::RpcResponse,
            MessageType::RpcError,
            MessageType::Event,
            MessageType::Heartbeat,
            MessageType::AuthRequest,
            MessageType::AuthResponse,
            MessageType::Close,
        ] {
            let byte = msg_type.to_byte();
            let parsed = MessageType::from_byte(byte).unwrap();
            assert_eq!(parsed, msg_type);
        }
    }

    #[test]
    fn test_config_default() {
        let config = NeoConfig::default();
        assert_eq!(config.max_message_size, MAX_MESSAGE_SIZE);
        assert_eq!(config.rpc_timeout, DEFAULT_RPC_TIMEOUT);
        assert_eq!(config.connect_timeout, DEFAULT_CONNECT_TIMEOUT);
    }

    #[test]
    fn test_context_creation() {
        let ctx = NeoContext::new(123, "test_service".to_string(), "test_method".to_string());
        assert_eq!(ctx.correlation_id, 123);
        assert_eq!(ctx.service_name, "test_service");
        assert_eq!(ctx.method_name, "test_method");
    }

    #[test]
    fn test_stats_recording() {
        let mut stats = NeoStats::default();
        stats.record_request(1000);
        stats.record_response();
        stats.record_error();
        stats.record_event();

        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.total_responses, 1);
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.avg_latency_us, 1000);
    }
}
