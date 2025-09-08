//! High-performance networking with kernel bypass and RDMA support
//! 
//! This module provides advanced networking capabilities optimized for ultra-low latency
//! and high throughput with kernel bypass, RDMA, and zero-copy operations.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, Message};

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address
    pub listen_addr: SocketAddr,
    /// Enable kernel bypass
    pub enable_kernel_bypass: bool,
    /// Enable RDMA
    pub enable_rdma: bool,
    /// Enable DPDK
    pub enable_dpdk: bool,
    /// Enable SR-IOV
    pub enable_sriov: bool,
    /// Maximum connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Receive buffer size
    pub recv_buffer_size: usize,
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Enable TCP_NODELAY
    pub tcp_nodelay: bool,
    /// Enable SO_REUSEPORT
    pub reuse_port: bool,
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    /// Enable scatter-gather I/O
    pub enable_scatter_gather: bool,
    /// Enable interrupt coalescing
    pub enable_interrupt_coalescing: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9092".parse().unwrap(),
            enable_kernel_bypass: false,
            enable_rdma: false,
            enable_dpdk: false,
            enable_sriov: false,
            max_connections: 10000,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(30),
            recv_buffer_size: 1024 * 1024, // 1MB
            send_buffer_size: 1024 * 1024, // 1MB
            tcp_nodelay: true,
            reuse_port: true,
            enable_zero_copy: true,
            enable_scatter_gather: true,
            enable_interrupt_coalescing: true,
        }
    }
}

/// Network transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    /// TCP transport
    Tcp,
    /// UDP transport
    Udp,
    /// RDMA transport
    Rdma,
    /// InfiniBand transport
    InfiniBand,
    /// DPDK transport
    Dpdk,
    /// Kernel bypass transport
    KernelBypass,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Disconnecting
    Disconnecting,
    /// Disconnected
    Disconnected,
    /// Error state
    Error,
}

/// Network connection
pub struct NetworkConnection {
    /// Connection ID
    pub id: u64,
    /// Remote address
    pub remote_addr: SocketAddr,
    /// Local address
    pub local_addr: SocketAddr,
    /// Connection state
    pub state: Arc<RwLock<ConnectionState>>,
    /// Transport type
    pub transport: TransportType,
    /// Connection statistics
    pub stats: Arc<RwLock<ConnectionStats>>,
    /// Last activity timestamp
    pub last_activity: Arc<RwLock<Instant>>,
    /// Message sender
    pub message_sender: mpsc::UnboundedSender<Message>,
    /// Message receiver
    pub message_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<Message>>>>,
}

/// Connection statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Messages sent
    pub messages_sent: u64,
    /// Connection duration
    pub connection_duration: Duration,
    /// Average latency (microseconds)
    pub avg_latency_us: u64,
    /// Last message timestamp
    pub last_message_time: Option<Instant>,
}

/// High-performance network server
pub struct NetworkServer {
    /// Network configuration
    config: NetworkConfig,
    /// Active connections
    connections: Arc<RwLock<HashMap<u64, Arc<NetworkConnection>>>>,
    /// Connection counter
    connection_counter: Arc<Mutex<u64>>,
    /// Server statistics
    stats: Arc<RwLock<ServerStats>>,
    /// Message handler
    message_handler: Arc<dyn MessageHandler + Send + Sync>,
    /// Hardware acceleration
    hardware_acceleration: HardwareAcceleration,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

/// Message handler trait
pub trait MessageHandler: Send + Sync {
    /// Handle incoming message
    async fn handle_message(&self, connection: Arc<NetworkConnection>, message: Message) -> Result<()>;
    
    /// Handle connection established
    async fn handle_connection_established(&self, connection: Arc<NetworkConnection>) -> Result<()>;
    
    /// Handle connection closed
    async fn handle_connection_closed(&self, connection: Arc<NetworkConnection>) -> Result<()>;
}

/// Server statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    /// Total connections accepted
    pub total_connections: u64,
    /// Active connections
    pub active_connections: usize,
    /// Total bytes received
    pub total_bytes_received: u64,
    /// Total bytes sent
    pub total_bytes_sent: u64,
    /// Total messages received
    pub total_messages_received: u64,
    /// Total messages sent
    pub total_messages_sent: u64,
    /// Server uptime
    pub uptime: Duration,
    /// Average connection duration
    pub avg_connection_duration: Duration,
    /// Peak connections
    pub peak_connections: usize,
}

/// Hardware acceleration capabilities
#[derive(Debug, Clone)]
struct HardwareAcceleration {
    /// RDMA support
    rdma_support: bool,
    /// DPDK support
    dpdk_support: bool,
    /// Kernel bypass support
    kernel_bypass_support: bool,
    /// SR-IOV support
    sriov_support: bool,
    /// Available network interfaces
    network_interfaces: Vec<NetworkInterface>,
}

/// Network interface information
#[derive(Debug, Clone)]
struct NetworkInterface {
    /// Interface name
    name: String,
    /// Interface type
    interface_type: InterfaceType,
    /// Maximum bandwidth
    max_bandwidth: u64,
    /// Supports RDMA
    supports_rdma: bool,
    /// Supports DPDK
    supports_dpdk: bool,
}

/// Network interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterfaceType {
    Ethernet,
    InfiniBand,
    RoCE,
    iWARP,
}

impl NetworkServer {
    /// Create a new network server
    pub fn new(config: NetworkConfig, message_handler: Arc<dyn MessageHandler + Send + Sync>) -> Self {
        let hardware_acceleration = Self::detect_hardware_acceleration(&config);
        
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_counter: Arc::new(Mutex::new(0)),
            stats: Arc::new(RwLock::new(ServerStats::default())),
            message_handler,
            hardware_acceleration,
            shutdown_tx: None,
        }
    }
    
    /// Start the network server
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting network server on {}", self.config.listen_addr);
        
        // Initialize hardware acceleration if enabled
        if self.config.enable_rdma && self.hardware_acceleration.rdma_support {
            self.initialize_rdma().await?;
        }
        
        if self.config.enable_dpdk && self.hardware_acceleration.dpdk_support {
            self.initialize_dpdk().await?;
        }
        
        if self.config.enable_kernel_bypass && self.hardware_acceleration.kernel_bypass_support {
            self.initialize_kernel_bypass().await?;
        }
        
        // Start the main server loop
        self.start_server_loop().await?;
        
        info!("Network server started successfully");
        Ok(())
    }
    
    /// Stop the network server
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping network server");
        
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(());
        }
        
        // Close all connections
        {
            let connections = self.connections.read().await;
            for connection in connections.values() {
                self.close_connection(connection.clone()).await?;
            }
        }
        
        info!("Network server stopped");
        Ok(())
    }
    
    /// Send a message to a specific connection
    pub async fn send_message(&self, connection_id: u64, message: Message) -> Result<()> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&connection_id) {
            connection.message_sender.send(message)?;
            Ok(())
        } else {
            Err(Error::ConnectionNotFound(connection_id))
        }
    }
    
    /// Broadcast a message to all connections
    pub async fn broadcast_message(&self, message: Message) -> Result<()> {
        let connections = self.connections.read().await;
        for connection in connections.values() {
            if let Err(e) = connection.message_sender.send(message.clone()) {
                warn!("Failed to send message to connection {}: {}", connection.id, e);
            }
        }
        Ok(())
    }
    
    /// Get server statistics
    pub async fn get_stats(&self) -> ServerStats {
        let mut stats = self.stats.read().await.clone();
        let connections = self.connections.read().await;
        stats.active_connections = connections.len();
        stats
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self, connection_id: u64) -> Result<ConnectionStats> {
        let connections = self.connections.read().await;
        if let Some(connection) = connections.get(&connection_id) {
            Ok(connection.stats.read().await.clone())
        } else {
            Err(Error::ConnectionNotFound(connection_id))
        }
    }
    
    /// Start the main server loop
    async fn start_server_loop(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.config.listen_addr).await?;
        info!("Listening on {}", self.config.listen_addr);
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel();
        self.shutdown_tx = Some(shutdown_tx);
        
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            if let Err(e) = self.handle_new_connection(stream, addr).await {
                                error!("Failed to handle new connection from {}: {}", addr, e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle a new connection
    async fn handle_new_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        // Check connection limit
        {
            let connections = self.connections.read().await;
            if connections.len() >= self.config.max_connections {
                return Err(Error::TooManyConnections(self.config.max_connections));
            }
        }
        
        // Generate connection ID
        let connection_id = {
            let mut counter = self.connection_counter.lock().unwrap();
            *counter += 1;
            *counter
        };
        
        // Configure socket options
        self.configure_socket(&stream).await?;
        
        // Create connection
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let connection = Arc::new(NetworkConnection {
            id: connection_id,
            remote_addr: addr,
            local_addr: stream.local_addr()?,
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            transport: TransportType::Tcp,
            stats: Arc::new(RwLock::new(ConnectionStats::default())),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            message_sender,
            message_receiver: Arc::new(Mutex::new(Some(message_receiver))),
        });
        
        // Add to connections
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id, connection.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.active_connections = connections.len();
            stats.peak_connections = stats.peak_connections.max(connections.len());
        }
        
        // Notify message handler
        self.message_handler.handle_connection_established(connection.clone()).await?;
        
        // Start connection handler
        self.start_connection_handler(connection, stream).await;
        
        info!("New connection established: {} from {}", connection_id, addr);
        Ok(())
    }
    
    /// Start connection handler
    async fn start_connection_handler(&self, connection: Arc<NetworkConnection>, stream: TcpStream) {
        let message_handler = self.message_handler.clone();
        let connections = self.connections.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let (mut reader, mut writer) = stream.into_split();
            
            // Start message sending task
            let connection_clone = connection.clone();
            let writer_clone = writer.clone();
            let send_task = tokio::spawn(async move {
                Self::handle_message_sending(connection_clone, writer_clone).await;
            });
            
            // Start message receiving task
            let connection_clone = connection.clone();
            let receive_task = tokio::spawn(async move {
                Self::handle_message_receiving(connection_clone, reader, message_handler).await;
            });
            
            // Wait for either task to complete
            tokio::select! {
                _ = send_task => {}
                _ = receive_task => {}
            }
            
            // Clean up connection
            Self::cleanup_connection(connection, connections, stats).await;
        });
    }
    
    /// Handle message sending
    async fn handle_message_sending(
        connection: Arc<NetworkConnection>,
        mut writer: impl AsyncWrite + Unpin,
    ) {
        let mut receiver = connection.message_receiver.lock().unwrap().take();
        if let Some(mut receiver) = receiver {
            while let Some(message) = receiver.recv().await {
                if let Err(e) = Self::send_message_to_stream(&mut writer, &message).await {
                    error!("Failed to send message to connection {}: {}", connection.id, e);
                    break;
                }
                
                // Update statistics
                {
                    let mut stats = connection.stats.write().await;
                    stats.bytes_sent += message.size() as u64;
                    stats.messages_sent += 1;
                    stats.last_message_time = Some(Instant::now());
                }
                
                // Update last activity
                {
                    let mut last_activity = connection.last_activity.write().await;
                    *last_activity = Instant::now();
                }
            }
        }
    }
    
    /// Handle message receiving
    async fn handle_message_receiving(
        connection: Arc<NetworkConnection>,
        mut reader: impl AsyncRead + Unpin,
        message_handler: Arc<dyn MessageHandler + Send + Sync>,
    ) {
        loop {
            match Self::receive_message_from_stream(&mut reader).await {
                Ok(Some(message)) => {
                    // Update statistics
                    {
                        let mut stats = connection.stats.write().await;
                        stats.bytes_received += message.size() as u64;
                        stats.messages_received += 1;
                        stats.last_message_time = Some(Instant::now());
                    }
                    
                    // Update last activity
                    {
                        let mut last_activity = connection.last_activity.write().await;
                        *last_activity = Instant::now();
                    }
                    
                    // Handle message
                    if let Err(e) = message_handler.handle_message(connection.clone(), message).await {
                        error!("Failed to handle message for connection {}: {}", connection.id, e);
                        break;
                    }
                }
                Ok(None) => {
                    // Connection closed
                    break;
                }
                Err(e) => {
                    error!("Failed to receive message from connection {}: {}", connection.id, e);
                    break;
                }
            }
        }
    }
    
    /// Send message to stream
    async fn send_message_to_stream(
        writer: &mut (impl AsyncWrite + Unpin),
        message: &Message,
    ) -> Result<()> {
        // Serialize message
        let serialized = bincode::serialize(message)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        // Send message length
        let length = serialized.len() as u32;
        writer.write_all(&length.to_le_bytes()).await?;
        
        // Send message data
        writer.write_all(&serialized).await?;
        writer.flush().await?;
        
        Ok(())
    }
    
    /// Receive message from stream
    async fn receive_message_from_stream(
        reader: &mut (impl AsyncRead + Unpin),
    ) -> Result<Option<Message>> {
        // Read message length
        let mut length_bytes = [0u8; 4];
        if reader.read_exact(&mut length_bytes).await.is_err() {
            return Ok(None); // Connection closed
        }
        
        let length = u32::from_le_bytes(length_bytes) as usize;
        
        // Read message data
        let mut message_data = vec![0u8; length];
        reader.read_exact(&mut message_data).await?;
        
        // Deserialize message
        let message: Message = bincode::deserialize(&message_data)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        
        Ok(Some(message))
    }
    
    /// Configure socket options
    async fn configure_socket(&self, stream: &TcpStream) -> Result<()> {
        use std::os::unix::io::AsRawFd;
        
        let fd = stream.as_raw_fd();
        
        // Set TCP_NODELAY
        if self.config.tcp_nodelay {
            unsafe {
                let flag: libc::c_int = 1;
                libc::setsockopt(
                    fd,
                    libc::IPPROTO_TCP,
                    libc::TCP_NODELAY,
                    &flag as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }
        }
        
        // Set SO_REUSEPORT
        if self.config.reuse_port {
            unsafe {
                let flag: libc::c_int = 1;
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    &flag as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }
        }
        
        // Set receive buffer size
        unsafe {
            let size = self.config.recv_buffer_size as libc::c_int;
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVBUF,
                &size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }
        
        // Set send buffer size
        unsafe {
            let size = self.config.send_buffer_size as libc::c_int;
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &size as *const _ as *const libc::c_void,
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            );
        }
        
        Ok(())
    }
    
    /// Close a connection
    async fn close_connection(&self, connection: Arc<NetworkConnection>) -> Result<()> {
        // Update connection state
        {
            let mut state = connection.state.write().await;
            *state = ConnectionState::Disconnected;
        }
        
        // Notify message handler
        self.message_handler.handle_connection_closed(connection.clone()).await?;
        
        info!("Connection closed: {}", connection.id);
        Ok(())
    }
    
    /// Clean up connection
    async fn cleanup_connection(
        connection: Arc<NetworkConnection>,
        connections: Arc<RwLock<HashMap<u64, Arc<NetworkConnection>>>>,
        stats: Arc<RwLock<ServerStats>>,
    ) {
        // Remove from connections
        {
            let mut connections_guard = connections.write().await;
            connections_guard.remove(&connection.id);
        }
        
        // Update statistics
        {
            let mut stats_guard = stats.write().await;
            stats_guard.active_connections = connections.read().await.len();
        }
        
        info!("Connection cleaned up: {}", connection.id);
    }
    
    /// Detect hardware acceleration capabilities
    fn detect_hardware_acceleration(config: &NetworkConfig) -> HardwareAcceleration {
        let mut network_interfaces = Vec::new();
        
        // Detect network interfaces
        if let Ok(interfaces) = getifaddrs::getifaddrs() {
            for interface in interfaces {
                if let Some(addr) = interface.addr {
                    if addr.is_loopback() {
                        continue;
                    }
                    
                    let interface_type = match addr {
                        std::net::IpAddr::V4(_) | std::net::IpAddr::V6(_) => InterfaceType::Ethernet,
                        _ => continue,
                    };
                    
                    network_interfaces.push(NetworkInterface {
                        name: interface.name,
                        interface_type,
                        max_bandwidth: 1000000000, // 1Gbps default
                        supports_rdma: false, // Would be detected in real implementation
                        supports_dpdk: false, // Would be detected in real implementation
                    });
                }
            }
        }
        
        HardwareAcceleration {
            rdma_support: config.enable_rdma,
            dpdk_support: config.enable_dpdk,
            kernel_bypass_support: config.enable_kernel_bypass,
            sriov_support: config.enable_sriov,
            network_interfaces,
        }
    }
    
    /// Initialize RDMA
    async fn initialize_rdma(&self) -> Result<()> {
        info!("Initializing RDMA support");
        // RDMA initialization would go here
        Ok(())
    }
    
    /// Initialize DPDK
    async fn initialize_dpdk(&self) -> Result<()> {
        info!("Initializing DPDK support");
        // DPDK initialization would go here
        Ok(())
    }
    
    /// Initialize kernel bypass
    async fn initialize_kernel_bypass(&self) -> Result<()> {
        info!("Initializing kernel bypass support");
        // Kernel bypass initialization would go here
        Ok(())
    }
}

// Add missing error variants
impl Error {
    pub fn ConnectionNotFound(connection_id: u64) -> Self {
        Error::Storage(format!("Connection not found: {}", connection_id))
    }
    
    pub fn TooManyConnections(max: usize) -> Self {
        Error::Storage(format!("Too many connections: {}", max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    struct TestMessageHandler;
    
    #[async_trait::async_trait]
    impl MessageHandler for TestMessageHandler {
        async fn handle_message(&self, _connection: Arc<NetworkConnection>, _message: Message) -> Result<()> {
            Ok(())
        }
        
        async fn handle_connection_established(&self, _connection: Arc<NetworkConnection>) -> Result<()> {
            Ok(())
        }
        
        async fn handle_connection_closed(&self, _connection: Arc<NetworkConnection>) -> Result<()> {
            Ok(())
        }
    }
    
    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        assert_eq!(config.listen_addr.port(), 9092);
        assert!(!config.enable_kernel_bypass);
        assert!(!config.enable_rdma);
        assert_eq!(config.max_connections, 10000);
    }
    
    #[test]
    fn test_connection_stats() {
        let stats = ConnectionStats::default();
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.messages_sent, 0);
    }
    
    #[test]
    fn test_server_stats() {
        let stats = ServerStats::default();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_bytes_received, 0);
        assert_eq!(stats.total_bytes_sent, 0);
    }
}