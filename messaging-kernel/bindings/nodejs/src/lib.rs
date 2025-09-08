//! Node.js bindings for Neo Messaging Kernel
//! 
//! This module provides high-performance Node.js bindings for the Neo Protocol,
//! enabling JavaScript/TypeScript applications to use the Neo Messaging Kernel.

use napi::{
    bindgen_prelude::*,
    JsFunction, JsString, JsNumber, JsBoolean, JsObject, JsArray,
    Result as NapiResult, Error as NapiError, Status,
};
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use anyhow::Result;

// Re-export the core Neo Protocol types
use neo_protocol::{
    NeoConfig, NeoContext, MessageType, NeoHeader,
    message::{RpcRequest, RpcResponse, EventMessage, HeartbeatMessage},
    monitoring::{MetricsCollector, MetricsConfig, MetricsSnapshot, HealthStatus},
    Error as NeoError,
};

/// Node.js error types
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Method not found: {0}")]
    MethodNotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<NeoError> for NodeError {
    fn from(err: NeoError) -> Self {
        match err {
            NeoError::InvalidMagic(_) => NodeError::InvalidArgument("Invalid magic number".to_string()),
            NeoError::UnsupportedVersion(_) => NodeError::InvalidArgument("Unsupported protocol version".to_string()),
            NeoError::MessageTooLarge(_, _) => NodeError::InvalidArgument("Message too large".to_string()),
            NeoError::InvalidMessageType(_) => NodeError::InvalidArgument("Invalid message type".to_string()),
            NeoError::SerializationError(msg) => NodeError::SerializationError(msg),
            NeoError::DeserializationError(msg) => NodeError::SerializationError(msg),
            NeoError::NetworkError(msg) => NodeError::NetworkError(msg),
            NeoError::TimeoutError => NodeError::TimeoutError("Operation timed out".to_string()),
            NeoError::AuthenticationError(msg) => NodeError::AuthenticationError(msg),
            NeoError::AuthorizationError(msg) => NodeError::AuthorizationError(msg),
            NeoError::ServiceNotFound(service) => NodeError::ServiceNotFound(service),
            NeoError::MethodNotFound(method) => NodeError::MethodNotFound(method),
            NeoError::InternalError(msg) => NodeError::InternalError(msg),
        }
    }
}

impl From<NodeError> for NapiError {
    fn from(err: NodeError) -> Self {
        match err {
            NodeError::InvalidArgument(msg) => NapiError::new(Status::InvalidArg, msg),
            NodeError::TimeoutError(msg) => NapiError::new(Status::GenericFailure, msg),
            _ => NapiError::new(Status::GenericFailure, err.to_string()),
        }
    }
}

/// Node.js configuration for Neo client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeClientConfig {
    pub server_address: String,
    pub timeout_seconds: f64,
    pub max_concurrent_requests: u32,
    pub enable_compression: bool,
    pub enable_auth: bool,
    pub auth_token: Option<String>,
}

impl Default for NodeClientConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:8080".to_string(),
            timeout_seconds: 30.0,
            max_concurrent_requests: 1000,
            enable_compression: false,
            enable_auth: false,
            auth_token: None,
        }
    }
}

impl From<NodeClientConfig> for NeoConfig {
    fn from(config: NodeClientConfig) -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            rpc_timeout: Duration::from_secs_f64(config.timeout_seconds),
            connect_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(30),
            max_concurrent_requests: config.max_concurrent_requests as usize,
            enable_compression: config.enable_compression,
            security: neo_protocol::SecurityConfig::default(),
        }
    }
}

/// Node.js configuration for Neo server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeServerConfig {
    pub address: String,
    pub max_concurrent_requests: u32,
    pub enable_compression: bool,
    pub enable_auth: bool,
    pub heartbeat_interval_seconds: f64,
}

impl Default for NodeServerConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:8080".to_string(),
            max_concurrent_requests: 1000,
            enable_compression: false,
            enable_auth: false,
            heartbeat_interval_seconds: 30.0,
        }
    }
}

impl From<NodeServerConfig> for NeoConfig {
    fn from(config: NodeServerConfig) -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            rpc_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs_f64(config.heartbeat_interval_seconds),
            max_concurrent_requests: config.max_concurrent_requests as usize,
            enable_compression: config.enable_compression,
            security: neo_protocol::SecurityConfig::default(),
        }
    }
}

/// Node.js service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeServiceDefinition {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub methods: HashMap<String, NodeMethodDefinition>,
    pub events: HashMap<String, NodeEventDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMethodDefinition {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub timeout_seconds: f64,
    pub queue: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEventDefinition {
    pub name: String,
    pub topic: String,
    pub partition_key: Option<String>,
    pub description: Option<String>,
}

/// Node.js RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRpcRequest {
    pub service_name: String,
    pub method_name: String,
    pub correlation_id: u64,
    pub payload: Vec<u8>,
    pub timeout_seconds: Option<f64>,
    pub timestamp: u64,
}

impl From<NodeRpcRequest> for RpcRequest {
    fn from(req: NodeRpcRequest) -> Self {
        Self {
            service_name: req.service_name,
            method_name: req.method_name,
            correlation_id: req.correlation_id,
            payload: bytes::Bytes::from(req.payload),
            timeout: req.timeout_seconds.map(Duration::from_secs_f64),
        }
    }
}

/// Node.js RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRpcResponse {
    pub correlation_id: u64,
    pub payload: Vec<u8>,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl From<RpcResponse> for NodeRpcResponse {
    fn from(resp: RpcResponse) -> Self {
        Self {
            correlation_id: resp.correlation_id,
            payload: resp.payload.to_vec(),
            success: resp.success,
            error: resp.error.map(|e| e.to_string()),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

/// Node.js event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEventMessage {
    pub topic: String,
    pub partition_key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Option<HashMap<String, String>>,
    pub timestamp: u64,
}

impl From<NodeEventMessage> for EventMessage {
    fn from(event: NodeEventMessage) -> Self {
        Self {
            topic: event.topic,
            partition_key: event.partition_key,
            payload: bytes::Bytes::from(event.payload),
            timestamp: SystemTime::now(),
        }
    }
}

/// Node.js metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetricsSnapshot {
    pub rpc_requests_total: u64,
    pub rpc_responses_total: u64,
    pub rpc_errors_total: u64,
    pub active_connections: u32,
    pub bytes_sent_total: u64,
    pub bytes_received_total: u64,
    pub service_registrations: u32,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: u64,
    pub uptime_seconds: u64,
    pub method_stats: HashMap<String, NodeMethodStats>,
    pub latency_stats: Option<NodeLatencyStats>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMethodStats {
    pub invocations: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub mean_ms: f64,
}

impl From<MetricsSnapshot> for NodeMetricsSnapshot {
    fn from(snapshot: MetricsSnapshot) -> Self {
        let method_stats = snapshot.method_stats.into_iter()
            .map(|(k, v)| (k, NodeMethodStats {
                invocations: v.invocations,
                errors: v.errors,
            }))
            .collect();

        let latency_stats = snapshot.latency_stats.map(|ls| NodeLatencyStats {
            min_ms: ls.min.as_secs_f64() * 1000.0,
            max_ms: ls.max.as_secs_f64() * 1000.0,
            p50_ms: ls.p50.as_secs_f64() * 1000.0,
            p95_ms: ls.p95.as_secs_f64() * 1000.0,
            p99_ms: ls.p99.as_secs_f64() * 1000.0,
            mean_ms: ls.mean.as_secs_f64() * 1000.0,
        });

        Self {
            rpc_requests_total: snapshot.rpc_requests_total,
            rpc_responses_total: snapshot.rpc_responses_total,
            rpc_errors_total: snapshot.rpc_errors_total,
            active_connections: snapshot.active_connections as u32,
            bytes_sent_total: snapshot.bytes_sent_total,
            bytes_received_total: snapshot.bytes_received_total,
            service_registrations: snapshot.service_registrations as u32,
            memory_usage_bytes: snapshot.memory_usage_bytes,
            cpu_usage_percent: snapshot.cpu_usage_percent,
            uptime_seconds: snapshot.uptime_seconds,
            method_stats,
            latency_stats,
            timestamp: snapshot.timestamp,
        }
    }
}

/// Node.js health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub checks: HashMap<String, NodeCheckResult>,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCheckResult {
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: u64,
}

impl From<HealthStatus> for NodeHealthStatus {
    fn from(status: HealthStatus) -> Self {
        let checks = status.checks.into_iter()
            .map(|(k, v)| (k, NodeCheckResult {
                status: match v.status {
                    neo_protocol::monitoring::HealthState::Healthy => "healthy".to_string(),
                    neo_protocol::monitoring::HealthState::Degraded => "degraded".to_string(),
                    neo_protocol::monitoring::HealthState::Unhealthy => "unhealthy".to_string(),
                },
                message: v.message,
                duration_ms: v.duration_ms,
                timestamp: v.timestamp,
            }))
            .collect();

        Self {
            status: match status.status {
                neo_protocol::monitoring::HealthState::Healthy => "healthy".to_string(),
                neo_protocol::monitoring::HealthState::Degraded => "degraded".to_string(),
                neo_protocol::monitoring::HealthState::Unhealthy => "unhealthy".to_string(),
            },
            timestamp: status.timestamp,
            checks,
            version: status.version,
            uptime_seconds: status.uptime_seconds,
        }
    }
}

/// Neo client for Node.js
#[napi]
pub struct NeoClient {
    config: NodeClientConfig,
    runtime: Arc<Runtime>,
    client: Arc<Mutex<Option<neo_protocol::NeoClient>>>,
    request_id: Arc<Mutex<u64>>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<RpcResponse, NeoError>>>>>,
}

#[napi]
impl NeoClient {
    #[napi(constructor)]
    pub fn new(config: Option<NodeClientConfig>) -> NapiResult<Self> {
        let config = config.unwrap_or_default();
        let runtime = Arc::new(Runtime::new().map_err(|e| NodeError::RuntimeError(e.to_string()))?);
        
        Ok(Self {
            config,
            runtime,
            client: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[napi]
    pub async fn connect(&self) -> NapiResult<()> {
        let runtime = self.runtime.clone();
        let config: NeoConfig = self.config.clone().into();
        
        let client = neo_protocol::NeoClient::new(config).await
            .map_err(NodeError::from)?;
        
        client.connect("127.0.0.1:8080".parse().unwrap()).await
            .map_err(NodeError::from)?;
        
        *self.client.lock().unwrap() = Some(client);
        Ok(())
    }

    #[napi]
    pub async fn call(&self, service_name: String, method_name: String, payload: Buffer) -> NapiResult<Buffer> {
        let runtime = self.runtime.clone();
        let client = self.client.clone();
        let request_id = self.request_id.clone();
        let pending_requests = self.pending_requests.clone();
        
        let mut client_guard = client.lock().unwrap();
        let client = client_guard.as_mut().ok_or_else(|| NodeError::RuntimeError("Client not connected".to_string()))?;
        
        let mut request_id_guard = request_id.lock().unwrap();
        *request_id_guard += 1;
        let correlation_id = *request_id_guard;
        drop(request_id_guard);
        
        let (tx, rx) = oneshot::channel();
        pending_requests.lock().unwrap().insert(correlation_id, tx);
        
        let response = client.call(
            "127.0.0.1:8080",
            &service_name,
            &method_name,
            bytes::Bytes::from(payload.to_vec()),
            Some(Duration::from_secs_f64(self.config.timeout_seconds)),
        ).await.map_err(NodeError::from)?;
        
        pending_requests.lock().unwrap().remove(&correlation_id);
        
        Ok(Buffer::from(response.to_vec()))
    }

    #[napi]
    pub async fn publish_event(&self, event: NodeEventMessage) -> NapiResult<()> {
        let client = self.client.clone();
        
        let mut client_guard = client.lock().unwrap();
        let client = client_guard.as_mut().ok_or_else(|| NodeError::RuntimeError("Client not connected".to_string()))?;
        
        let event: EventMessage = event.into();
        client.publish_event(&event).await.map_err(NodeError::from)?;
        
        Ok(())
    }

    #[napi]
    pub fn close(&self) -> NapiResult<()> {
        let mut client_guard = self.client.lock().unwrap();
        *client_guard = None;
        Ok(())
    }
}

/// Neo server for Node.js
#[napi]
pub struct NeoServer {
    config: NodeServerConfig,
    runtime: Arc<Runtime>,
    server: Arc<Mutex<Option<neo_protocol::NeoServer>>>,
    services: Arc<Mutex<HashMap<String, NodeServiceDefinition>>>,
    handlers: Arc<Mutex<HashMap<String, JsFunction>>>,
}

#[napi]
impl NeoServer {
    #[napi(constructor)]
    pub fn new(config: Option<NodeServerConfig>) -> NapiResult<Self> {
        let config = config.unwrap_or_default();
        let runtime = Arc::new(Runtime::new().map_err(|e| NodeError::RuntimeError(e.to_string()))?);
        
        Ok(Self {
            config,
            runtime,
            server: Arc::new(Mutex::new(None)),
            services: Arc::new(Mutex::new(HashMap::new())),
            handlers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[napi]
    pub fn register_service(&self, service: NodeServiceDefinition, handlers: JsFunction) -> NapiResult<()> {
        let services = self.services.clone();
        let handlers_map = self.handlers.clone();
        
        services.lock().unwrap().insert(service.name.clone(), service);
        handlers_map.lock().unwrap().insert(service.name.clone(), handlers);
        
        Ok(())
    }

    #[napi]
    pub async fn start(&self) -> NapiResult<()> {
        let config: NeoConfig = self.config.clone().into();
        let server = self.server.clone();
        
        let mut server_instance = neo_protocol::NeoServer::new(config).await
            .map_err(NodeError::from)?;
        
        server_instance.start("127.0.0.1:8080".parse().unwrap()).await
            .map_err(NodeError::from)?;
        
        *server.lock().unwrap() = Some(server_instance);
        Ok(())
    }

    #[napi]
    pub fn stop(&self) -> NapiResult<()> {
        let mut server_guard = self.server.lock().unwrap();
        *server_guard = None;
        Ok(())
    }
}

/// Metrics collector for Node.js
#[napi]
pub struct NeoMetricsCollector {
    collector: Arc<Mutex<MetricsCollector>>,
}

#[napi]
impl NeoMetricsCollector {
    #[napi(constructor)]
    pub fn new() -> NapiResult<Self> {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        Ok(Self {
            collector: Arc::new(Mutex::new(collector)),
        })
    }

    #[napi]
    pub async fn start(&self) -> NapiResult<()> {
        let collector = self.collector.clone();
        
        collector.lock().unwrap().start().await.map_err(NodeError::from)?;
        Ok(())
    }

    #[napi]
    pub fn get_snapshot(&self) -> NapiResult<NodeMetricsSnapshot> {
        let collector = self.collector.lock().unwrap();
        let snapshot = collector.get_metrics_snapshot();
        Ok(snapshot.into())
    }

    #[napi]
    pub async fn get_health_status(&self) -> NapiResult<NodeHealthStatus> {
        let collector = self.collector.clone();
        
        let status = collector.lock().unwrap().get_health_status().await;
        Ok(status.into())
    }

    #[napi]
    pub fn record_rpc_request(&self) {
        self.collector.lock().unwrap().record_rpc_request();
    }

    #[napi]
    pub fn record_rpc_response(&self, latency_ms: f64) {
        self.collector.lock().unwrap().record_rpc_response(Duration::from_secs_f64(latency_ms / 1000.0));
    }

    #[napi]
    pub fn record_rpc_error(&self, method_name: String) {
        self.collector.lock().unwrap().record_rpc_error(&method_name);
    }

    #[napi]
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.collector.lock().unwrap().record_bytes_sent(bytes);
    }

    #[napi]
    pub fn record_bytes_received(&self, bytes: u64) {
        self.collector.lock().unwrap().record_bytes_received(bytes);
    }

    #[napi]
    pub fn increment_connections(&self) {
        self.collector.lock().unwrap().increment_connections();
    }

    #[napi]
    pub fn decrement_connections(&self) {
        self.collector.lock().unwrap().decrement_connections();
    }

    #[napi]
    pub fn record_service_registration(&self) {
        self.collector.lock().unwrap().record_service_registration();
    }
}

/// Utility functions
#[napi]
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[napi]
pub fn get_protocol_version() -> u8 {
    neo_protocol::NEO_PROTOCOL_VERSION
}

#[napi]
pub fn get_max_message_size() -> u32 {
    neo_protocol::MAX_MESSAGE_SIZE as u32
}

/// Module initialization
#[napi]
pub fn init() -> NapiResult<()> {
    // Initialize any global state if needed
    Ok(())
}