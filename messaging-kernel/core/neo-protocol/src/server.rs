//! Neo Protocol server implementation

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{interval, timeout};
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{
    Result, Error, NeoConfig, NeoMessage, MessageType, NeoContext, SecurityContext,
    message::{MessageFactory, RpcRequest, RpcResponse, RpcError, AuthRequest, AuthResponse},
    codec::AsyncFramedNeoCodec,
    security::{SecurityManager, AuthCredentials, CredentialType},
    service::{ServiceRegistry, MethodHandler, AsyncMethodHandler, EventHandler},
};

/// Neo Protocol server
pub struct NeoServer {
    /// Server configuration
    config: NeoConfig,
    /// Security manager
    security_manager: Arc<SecurityManager>,
    /// Service registry
    service_registry: Arc<ServiceRegistry>,
    /// Server statistics
    stats: Arc<RwLock<ServerStats>>,
    /// Active connections
    connections: Arc<RwLock<HashMap<String, Arc<ServerConnection>>>>,
    /// Server shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Server connection
pub struct ServerConnection {
    /// Connection ID
    pub id: String,
    /// Client address
    pub client_addr: SocketAddr,
    /// Connection stream
    pub stream: Arc<RwLock<TcpStream>>,
    /// Connection metadata
    pub metadata: HashMap<String, String>,
    /// Security context
    pub security_context: Arc<RwLock<Option<SecurityContext>>>,
    /// Last activity
    pub last_activity: Arc<RwLock<SystemTime>>,
    /// Connection statistics
    pub stats: Arc<RwLock<ConnectionStats>>,
}

/// Server statistics
#[derive(Debug, Default)]
pub struct ServerStats {
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
    /// Active connections
    pub active_connections: usize,
    /// Uptime
    pub uptime: Duration,
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
    /// Requests processed
    pub requests_processed: u64,
}

impl NeoServer {
    /// Create a new Neo server
    pub fn new(config: NeoConfig) -> Self {
        Self {
            security_manager: Arc::new(SecurityManager::new(config.security.clone())),
            service_registry: Arc::new(ServiceRegistry::new()),
            config,
            stats: Arc::new(RwLock::new(ServerStats::default())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        }
    }

    /// Start the server
    pub async fn start(&mut self, addr: SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        info!("Neo server started on {}", addr);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start heartbeat task
        self.start_heartbeat_task().await;

        // Start statistics task
        self.start_stats_task().await;

        loop {
            tokio::select! {
                // Accept new connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, client_addr)) => {
                            self.handle_new_connection(stream, client_addr).await;
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Shutting down server");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&self) -> Result<()> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    /// Register a service
    pub async fn register_service(
        &self,
        definition: crate::service::ServiceDefinition,
        method_handlers: HashMap<String, MethodHandler>,
        async_method_handlers: HashMap<String, AsyncMethodHandler>,
        event_handlers: HashMap<String, EventHandler>,
    ) -> Result<()> {
        self.service_registry.register_service(
            definition,
            method_handlers,
            async_method_handlers,
            event_handlers,
        ).await
    }

    /// Unregister a service
    pub async fn unregister_service(&self, service_name: &str) -> Result<()> {
        self.service_registry.unregister_service(service_name).await
    }

    /// Get server statistics
    pub async fn get_stats(&self) -> ServerStats {
        let stats = self.stats.read().await;
        let connections = self.connections.read().await;
        
        ServerStats {
            total_requests: stats.total_requests,
            total_responses: stats.total_responses,
            total_errors: stats.total_errors,
            total_events: stats.total_events,
            avg_latency_us: stats.avg_latency_us,
            active_connections: connections.len(),
            uptime: stats.uptime,
        }
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self, connection_id: &str) -> Result<ConnectionStats> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(connection_id) {
            let stats = connection.stats.read().await;
            Ok(stats.clone())
        } else {
            Err(Error::Protocol(format!("Connection not found: {}", connection_id)))
        }
    }

    /// Handle a new connection
    async fn handle_new_connection(&self, stream: TcpStream, client_addr: SocketAddr) {
        let connection_id = format!("{}:{}", client_addr.ip(), client_addr.port());
        
        let connection = Arc::new(ServerConnection {
            id: connection_id.clone(),
            client_addr,
            stream: Arc::new(RwLock::new(stream)),
            metadata: HashMap::new(),
            security_context: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(RwLock::new(SystemTime::now())),
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
        });

        // Add to connections
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), connection.clone());
        }

        // Update server stats
        {
            let mut stats = self.stats.write().await;
            stats.active_connections += 1;
        }

        info!("New connection from {}", client_addr);

        // Start connection handler
        self.start_connection_handler(connection).await;
    }

    /// Start connection handler for a client
    async fn start_connection_handler(&self, connection: Arc<ServerConnection>) {
        let service_registry = self.service_registry.clone();
        let security_manager = self.security_manager.clone();
        let stats = self.stats.clone();
        let connections = self.connections.clone();
        let connection_id = connection.id.clone();

        tokio::spawn(async move {
            let mut stream_guard = connection.stream.write().await;
            
            loop {
                match AsyncFramedNeoCodec::decode_framed_from_reader(&mut *stream_guard).await {
                    Ok(message) => {
                        // Update connection statistics
                        {
                            let mut conn_stats = connection.stats.write().await;
                            conn_stats.messages_received += 1;
                            conn_stats.bytes_received += message.payload.len() as u64;
                        }

                        // Update last activity
                        {
                            let mut last_activity = connection.last_activity.write().await;
                            *last_activity = SystemTime::now();
                        }

                        // Handle message
                        match message.header.message_type {
                            MessageType::RpcRequest => {
                                if let Err(e) = Self::handle_rpc_request(
                                    &service_registry,
                                    &security_manager,
                                    &connection,
                                    message,
                                ).await {
                                    error!("Error handling RPC request: {}", e);
                                    
                                    // Update error statistics
                                    {
                                        let mut conn_stats = connection.stats.write().await;
                                        conn_stats.connection_errors += 1;
                                    }
                                    
                                    let mut server_stats = stats.write().await;
                                    server_stats.total_errors += 1;
                                }
                            }
                            MessageType::Event => {
                                if let Err(e) = Self::handle_event(
                                    &service_registry,
                                    &connection,
                                    message,
                                ).await {
                                    error!("Error handling event: {}", e);
                                }
                            }
                            MessageType::Heartbeat => {
                                if let Err(e) = Self::handle_heartbeat(&connection, message).await {
                                    error!("Error handling heartbeat: {}", e);
                                }
                            }
                            MessageType::AuthRequest => {
                                if let Err(e) = Self::handle_auth_request(
                                    &security_manager,
                                    &connection,
                                    message,
                                ).await {
                                    error!("Error handling auth request: {}", e);
                                }
                            }
                            MessageType::Close => {
                                info!("Connection {} requested close", connection_id);
                                break;
                            }
                            _ => {
                                warn!("Unknown message type: {:?}", message.header.message_type);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error receiving message from {}: {}", connection_id, e);
                        
                        // Update error statistics
                        {
                            let mut conn_stats = connection.stats.write().await;
                            conn_stats.connection_errors += 1;
                        }
                        
                        let mut server_stats = stats.write().await;
                        server_stats.total_errors += 1;
                        
                        break;
                    }
                }
            }

            // Remove connection
            {
                let mut connections = connections.write().await;
                connections.remove(&connection_id);
            }

            // Update server stats
            {
                let mut stats = stats.write().await;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }

            info!("Connection {} closed", connection_id);
        });
    }

    /// Handle RPC request
    async fn handle_rpc_request(
        service_registry: &ServiceRegistry,
        security_manager: &SecurityManager,
        connection: &ServerConnection,
        message: NeoMessage,
    ) -> Result<()> {
        let start_time = SystemTime::now();
        
        // Parse RPC request
        let rpc_request: RpcRequest = bincode::deserialize(&message.payload)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Create context
        let mut context = NeoContext::new(
            message.header.correlation_id,
            rpc_request.service_name.clone(),
            rpc_request.method_name.clone(),
        );

        // Check authentication if required
        if security_manager.config.enable_auth {
            if let Some(security_context) = connection.security_context.read().await.as_ref() {
                context.security_context = Some(security_context.clone());
            } else {
                return Self::send_error_response(
                    connection,
                    message.header.correlation_id,
                    Error::AuthenticationFailed("Authentication required".to_string()),
                ).await;
            }
        }

        // Call service method
        let result = service_registry.call_method(
            &rpc_request.service_name,
            &rpc_request.method_name,
            context,
            rpc_request.params.to_vec(),
        ).await;

        // Send response
        match result {
            Ok(response_data) => {
                let processing_time = start_time.elapsed().unwrap_or_default();
                let response = RpcResponse {
                    data: Bytes::from(response_data),
                    metadata: HashMap::new(),
                    processing_time,
                };

                let response_message = MessageFactory::rpc_response(
                    message.header.correlation_id,
                    response.data,
                    processing_time,
                )?;

                Self::send_message(connection, response_message).await?;
            }
            Err(e) => {
                Self::send_error_response(connection, message.header.correlation_id, e).await?;
            }
        }

        Ok(())
    }

    /// Handle event
    async fn handle_event(
        service_registry: &ServiceRegistry,
        connection: &ServerConnection,
        message: NeoMessage,
    ) -> Result<()> {
        // Parse event message
        let event_message: crate::message::EventMessage = bincode::deserialize(&message.payload)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Create context
        let context = NeoContext::new(
            message.header.correlation_id,
            "event".to_string(),
            event_message.topic.clone(),
        );

        // Publish event (this would typically go to a message broker)
        // For now, we'll just log it
        info!("Received event on topic: {}", event_message.topic);

        Ok(())
    }

    /// Handle heartbeat
    async fn handle_heartbeat(
        connection: &ServerConnection,
        message: NeoMessage,
    ) -> Result<()> {
        // Parse heartbeat message
        let heartbeat: crate::message::HeartbeatMessage = bincode::deserialize(&message.payload)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        debug!("Received heartbeat from client: {}", heartbeat.client_id);

        // Update last activity
        {
            let mut last_activity = connection.last_activity.write().await;
            *last_activity = SystemTime::now();
        }

        Ok(())
    }

    /// Handle authentication request
    async fn handle_auth_request(
        security_manager: &SecurityManager,
        connection: &ServerConnection,
        message: NeoMessage,
    ) -> Result<()> {
        // Parse auth request
        let auth_request: AuthRequest = bincode::deserialize(&message.payload)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Create credentials
        let credentials = AuthCredentials {
            credential_type: CredentialType::Token,
            data: auth_request.credentials,
            metadata: auth_request.metadata,
        };

        // Authenticate
        let auth_result = security_manager.authenticate(&credentials);

        let auth_response = match auth_result {
            Ok(security_context) => {
                // Store security context
                {
                    let mut ctx_guard = connection.security_context.write().await;
                    *ctx_guard = Some(security_context);
                }

                AuthResponse {
                    success: true,
                    token: Some("dummy_token".to_string()),
                    expires_at: Some(SystemTime::now() + Duration::from_secs(3600)),
                    server_capabilities: vec!["neo-protocol".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                AuthResponse {
                    success: false,
                    token: None,
                    expires_at: None,
                    server_capabilities: vec![],
                    error: Some(e.to_string()),
                }
            }
        };

        let response_message = MessageFactory::auth_response(
            auth_response.success,
            auth_response.token,
            auth_response.expires_at,
            auth_response.server_capabilities,
            auth_response.error,
        )?;

        Self::send_message(connection, response_message).await?;

        Ok(())
    }

    /// Send a message to a connection
    async fn send_message(connection: &ServerConnection, message: NeoMessage) -> Result<()> {
        let mut stream_guard = connection.stream.write().await;
        AsyncFramedNeoCodec::encode_framed_to_writer(&message, &mut *stream_guard).await?;

        // Update connection statistics
        {
            let mut conn_stats = connection.stats.write().await;
            conn_stats.messages_sent += 1;
            conn_stats.bytes_sent += message.payload.len() as u64;
        }

        Ok(())
    }

    /// Send error response
    async fn send_error_response(
        connection: &ServerConnection,
        correlation_id: u64,
        error: Error,
    ) -> Result<()> {
        let error_message = MessageFactory::rpc_error(
            correlation_id,
            1, // Generic error code
            error.to_string(),
            None,
        )?;

        Self::send_message(connection, error_message).await?;

        Ok(())
    }

    /// Start heartbeat task
    async fn start_heartbeat_task(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.heartbeat_interval);
            
            loop {
                interval.tick().await;
                
                let connections_guard = connections.read().await;
                let now = SystemTime::now();
                
                for (connection_id, connection) in connections_guard.iter() {
                    let last_activity = *connection.last_activity.read().await;
                    if now.duration_since(last_activity).unwrap_or_default() > config.heartbeat_interval * 2 {
                        warn!("Connection {} appears to be dead, closing", connection_id);
                        // In a real implementation, you would close the connection here
                    }
                }
            }
        });
    }

    /// Start statistics task
    async fn start_stats_task(&self) {
        let stats = self.stats.clone();
        let start_time = SystemTime::now();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Update every minute
            
            loop {
                interval.tick().await;
                
                let mut stats_guard = stats.write().await;
                stats_guard.uptime = start_time.elapsed().unwrap_or_default();
            }
        });
    }
}

/// Server builder for creating Neo servers
pub struct NeoServerBuilder {
    config: NeoConfig,
}

impl NeoServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: NeoConfig::default(),
        }
    }

    /// Set server configuration
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

    /// Build the server
    pub fn build(self) -> NeoServer {
        NeoServer::new(self.config)
    }
}

impl Default for NeoServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_server_builder() {
        let server = NeoServerBuilder::new()
            .rpc_timeout(Duration::from_secs(60))
            .max_message_size(1024 * 1024)
            .enable_compression()
            .build();
        
        assert_eq!(server.config.rpc_timeout, Duration::from_secs(60));
        assert_eq!(server.config.max_message_size, 1024 * 1024);
        assert!(server.config.enable_compression);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = NeoServer::new(NeoConfig::default());
        let stats = server.get_stats().await;
        
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.total_responses, 0);
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.connection_errors, 0);
        assert_eq!(stats.requests_processed, 0);
    }
}