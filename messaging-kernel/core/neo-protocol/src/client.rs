//! Neo Protocol client implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{timeout, Instant};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{
    Result, Error, NeoConfig, NeoMessage, MessageType, NeoContext, SecurityContext,
    message::{MessageFactory, RpcRequest, RpcResponse, RpcError},
    codec::AsyncFramedNeoCodec,
    security::SecurityManager,
};

/// Neo Protocol client
pub struct NeoClient {
    /// Client configuration
    config: NeoConfig,
    /// Security manager
    security_manager: Arc<SecurityManager>,
    /// Connection pool
    connections: Arc<RwLock<HashMap<String, Arc<ClientConnection>>>>,
    /// Request ID counter
    request_id_counter: Arc<RwLock<u64>>,
    /// Pending requests
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<NeoMessage>>>>>,
    /// Client statistics
    stats: Arc<RwLock<ClientStats>>,
}

/// Client connection
pub struct ClientConnection {
    /// Connection stream
    stream: Arc<RwLock<Option<TcpStream>>>,
    /// Connection address
    address: String,
    /// Connection metadata
    metadata: HashMap<String, String>,
    /// Last activity
    last_activity: Arc<RwLock<SystemTime>>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

/// Client statistics
#[derive(Debug, Default)]
pub struct ClientStats {
    /// Total requests sent
    pub total_requests: u64,
    /// Total responses received
    pub total_responses: u64,
    /// Total errors
    pub total_errors: u64,
    /// Average request latency (microseconds)
    pub avg_latency_us: u64,
    /// Active connections
    pub active_connections: usize,
    /// Pending requests
    pub pending_requests: usize,
}

/// Connection statistics
#[derive(Debug, Default, Clone)]
pub struct ConnectionStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Connection errors
    pub connection_errors: u64,
}

impl NeoClient {
    /// Create a new Neo client
    pub fn new(config: NeoConfig) -> Self {
        Self {
            security_manager: Arc::new(SecurityManager::new(config.security.clone())),
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            request_id_counter: Arc::new(RwLock::new(0)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ClientStats::default())),
        }
    }

    /// Connect to a service
    pub async fn connect(&self, address: &str) -> Result<()> {
        let stream = TcpStream::connect(address).await?;
        
        let connection = Arc::new(ClientConnection {
            stream: Arc::new(RwLock::new(Some(stream))),
            address: address.to_string(),
            metadata: HashMap::new(),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        });

        let mut connections = self.connections.write().await;
        connections.insert(address.to_string(), connection);

        // Start connection handler
        self.start_connection_handler(address.to_string()).await;

        info!("Connected to service at {}", address);
        Ok(())
    }

    /// Disconnect from a service
    pub async fn disconnect(&self, address: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        if connections.remove(address).is_some() {
            info!("Disconnected from service at {}", address);
            Ok(())
        } else {
            Err(Error::Protocol(format!("Not connected to {}", address)))
        }
    }

    /// Make an RPC call
    pub async fn call(
        &self,
        address: &str,
        service_name: &str,
        method_name: &str,
        params: Vec<u8>,
        timeout_duration: Option<Duration>,
    ) -> Result<Vec<u8>> {
        let start_time = Instant::now();
        
        // Get or create connection
        let connection = self.get_or_create_connection(address).await?;
        
        // Generate request ID
        let request_id = self.generate_request_id().await;
        
        // Create RPC request
        let request = RpcRequest {
            service_name: service_name.to_string(),
            method_name: method_name.to_string(),
            params: Bytes::from(params),
            timeout: timeout_duration,
            client_info: None,
            metadata: HashMap::new(),
        };
        
        let message = MessageFactory::rpc_request(request_id, service_name.to_string(), method_name.to_string(), request.params)?;
        
        // Send request
        self.send_message(&connection, message).await?;
        
        // Wait for response
        let timeout_duration = timeout_duration.unwrap_or(self.config.rpc_timeout);
        let response = timeout(timeout_duration, self.wait_for_response(request_id)).await
            .map_err(|_| Error::RpcTimeout(timeout_duration))??;
        
        // Record statistics
        let latency_us = start_time.elapsed().as_micros() as u64;
        self.record_request_stats(latency_us).await;
        
        // Parse response
        match response.header.message_type {
            MessageType::RpcResponse => {
                let rpc_response: RpcResponse = bincode::deserialize(&response.payload)
                    .map_err(|e| Error::Serialization(e.to_string()))?;
                Ok(rpc_response.data.to_vec())
            }
            MessageType::RpcError => {
                let rpc_error: RpcError = bincode::deserialize(&response.payload)
                    .map_err(|e| Error::Serialization(e.to_string()))?;
                Err(Error::Protocol(format!("RPC error: {} (code: {})", rpc_error.message, rpc_error.code)))
            }
            _ => Err(Error::Protocol("Invalid response message type".to_string())),
        }
    }

    /// Publish an event
    pub async fn publish_event(
        &self,
        address: &str,
        topic: &str,
        data: Vec<u8>,
        partition_key: Option<String>,
    ) -> Result<()> {
        let connection = self.get_or_create_connection(address).await?;
        
        let message = MessageFactory::event(topic.to_string(), Bytes::from(data), partition_key)?;
        
        self.send_message(&connection, message).await?;
        
        // Record event statistics
        self.record_event_stats().await;
        
        Ok(())
    }

    /// Send a heartbeat
    pub async fn send_heartbeat(&self, address: &str, client_id: &str) -> Result<()> {
        let connection = self.get_or_create_connection(address).await?;
        
        let message = MessageFactory::heartbeat(client_id.to_string())?;
        
        self.send_message(&connection, message).await?;
        
        Ok(())
    }

    /// Get client statistics
    pub async fn get_stats(&self) -> ClientStats {
        let stats = self.stats.read().await;
        let connections = self.connections.read().await;
        let pending = self.pending_requests.read().await;
        
        ClientStats {
            total_requests: stats.total_requests,
            total_responses: stats.total_responses,
            total_errors: stats.total_errors,
            avg_latency_us: stats.avg_latency_us,
            active_connections: connections.len(),
            pending_requests: pending.len(),
        }
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self, address: &str) -> Result<ConnectionStats> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(address) {
            let stats = connection.stats.read().await;
            Ok(stats.clone())
        } else {
            Err(Error::Protocol(format!("No connection to {}", address)))
        }
    }

    /// Authenticate with a service
    pub async fn authenticate(
        &self,
        address: &str,
        credentials: crate::security::AuthCredentials,
    ) -> Result<SecurityContext> {
        let connection = self.get_or_create_connection(address).await?;
        
        let message = MessageFactory::auth_request(
            "token".to_string(),
            credentials.data,
            vec!["neo-protocol".to_string()],
        )?;
        
        self.send_message(&connection, message).await?;
        
        // Wait for auth response
        let response = self.wait_for_response(0).await?;
        
        match response.header.message_type {
            MessageType::AuthResponse => {
                let auth_response: crate::message::AuthResponse = bincode::deserialize(&response.payload)
                    .map_err(|e| Error::Serialization(e.to_string()))?;
                
                if auth_response.success {
                    let mut context = SecurityContext::new("authenticated".to_string());
                    context.token = auth_response.token;
                    context.token_expires_at = auth_response.expires_at;
                    Ok(context)
                } else {
                    Err(Error::AuthenticationFailed(auth_response.error.unwrap_or("Authentication failed".to_string())))
                }
            }
            _ => Err(Error::Protocol("Invalid authentication response".to_string())),
        }
    }

    /// Get or create a connection
    async fn get_or_create_connection(&self, address: &str) -> Result<Arc<ClientConnection>> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(address) {
            return Ok(connection.clone());
        }
        drop(connections);
        
        // Create new connection
        self.connect(address).await?;
        
        let connections = self.connections.read().await;
        connections.get(address)
            .cloned()
            .ok_or_else(|| Error::Protocol(format!("Failed to create connection to {}", address)))
    }

    /// Generate a unique request ID
    async fn generate_request_id(&self) -> u64 {
        let mut counter = self.request_id_counter.write().await;
        *counter += 1;
        *counter
    }

    /// Send a message through a connection
    async fn send_message(&self, connection: &ClientConnection, message: NeoMessage) -> Result<()> {
        let mut stream_guard = connection.stream.write().await;
        if let Some(stream) = stream_guard.as_mut() {
            AsyncFramedNeoCodec::encode_framed_to_writer(&message, stream).await?;
            
            // Update connection statistics
            let mut stats = connection.stats.write().await;
            stats.messages_sent += 1;
            stats.bytes_sent += message.payload.len() as u64;
            
            // Update last activity
            let mut last_activity = connection.last_activity.write().await;
            *last_activity = SystemTime::now();
            
            Ok(())
        } else {
            Err(Error::ConnectionClosed)
        }
    }

    /// Wait for a response with the given request ID
    async fn wait_for_response(&self, request_id: u64) -> Result<NeoMessage> {
        let (tx, rx) = oneshot::channel();
        
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id, tx);
        }
        
        rx.await.map_err(|_| Error::ConnectionClosed)?
    }

    /// Start connection handler for receiving messages
    async fn start_connection_handler(&self, address: String) {
        let connections = self.connections.clone();
        let pending_requests = self.pending_requests.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            if let Some(connection) = connections.read().await.get(&address) {
                let stream = connection.stream.clone();
                let connection_stats = connection.stats.clone();
                
                loop {
                    let mut stream_guard = stream.write().await;
                    if let Some(stream) = stream_guard.as_mut() {
                        match AsyncFramedNeoCodec::decode_framed_from_reader(stream).await {
                            Ok(message) => {
                                // Update connection statistics
                                let mut stats_guard = connection_stats.write().await;
                                stats_guard.messages_received += 1;
                                stats_guard.bytes_received += message.payload.len() as u64;
                                
                                // Handle response
                                if let Some(tx) = pending_requests.write().await.remove(&message.header.correlation_id) {
                                    let _ = tx.send(Ok(message));
                                }
                                
                                // Update client statistics
                                let mut client_stats = stats.write().await;
                                client_stats.total_responses += 1;
                            }
                            Err(e) => {
                                error!("Error receiving message from {}: {}", address, e);
                                
                                // Update connection statistics
                                let mut stats_guard = connection_stats.write().await;
                                stats_guard.connection_errors += 1;
                                
                                // Update client statistics
                                let mut client_stats = stats.write().await;
                                client_stats.total_errors += 1;
                                
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        });
    }

    /// Record request statistics
    async fn record_request_stats(&self, latency_us: u64) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.avg_latency_us = (stats.avg_latency_us * (stats.total_requests - 1) + latency_us) / stats.total_requests;
    }

    /// Record event statistics
    async fn record_event_stats(&self) {
        // Events don't have responses, so we just track that they were sent
        // In a real implementation, you might want to track event delivery confirmations
    }
}

/// Client builder for creating Neo clients
pub struct NeoClientBuilder {
    config: NeoConfig,
}

impl NeoClientBuilder {
    /// Create a new client builder
    pub fn new() -> Self {
        Self {
            config: NeoConfig::default(),
        }
    }

    /// Set client configuration
    pub fn config(mut self, config: NeoConfig) -> Self {
        self.config = config;
        self
    }

    /// Set RPC timeout
    pub fn rpc_timeout(mut self, timeout: Duration) -> Self {
        self.config.rpc_timeout = timeout;
        self
    }

    /// Set connection timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set maximum message size
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = size;
        self
    }

    /// Enable compression
    pub fn enable_compression(mut self) -> Self {
        self.config.enable_compression = true;
        self
    }

    /// Build the client
    pub fn build(self) -> NeoClient {
        NeoClient::new(self.config)
    }
}

impl Default for NeoClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_client_builder() {
        let client = NeoClientBuilder::new()
            .rpc_timeout(Duration::from_secs(60))
            .max_message_size(1024 * 1024)
            .enable_compression()
            .build();
        
        assert_eq!(client.config.rpc_timeout, Duration::from_secs(60));
        assert_eq!(client.config.max_message_size, 1024 * 1024);
        assert!(client.config.enable_compression);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = NeoClient::new(NeoConfig::default());
        let stats = client.get_stats().await;
        
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_responses, 0);
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.pending_requests, 0);
    }

    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.connection_errors, 0);
    }
}