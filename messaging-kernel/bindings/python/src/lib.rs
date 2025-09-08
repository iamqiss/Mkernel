//! Python bindings for Neo Messaging Kernel
//! 
//! This module provides high-performance Python bindings for the Neo Protocol,
//! enabling Python applications to use the Neo Messaging Kernel with minimal overhead.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};
use pyo3::exceptions::{PyValueError, PyRuntimeError, PyTimeoutError};
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

/// Python error types
#[derive(Debug, thiserror::Error)]
pub enum PythonError {
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

impl From<NeoError> for PythonError {
    fn from(err: NeoError) -> Self {
        match err {
            NeoError::InvalidMagic(_) => PythonError::InvalidArgument("Invalid magic number".to_string()),
            NeoError::UnsupportedVersion(_) => PythonError::InvalidArgument("Unsupported protocol version".to_string()),
            NeoError::MessageTooLarge(_, _) => PythonError::InvalidArgument("Message too large".to_string()),
            NeoError::InvalidMessageType(_) => PythonError::InvalidArgument("Invalid message type".to_string()),
            NeoError::SerializationError(msg) => PythonError::SerializationError(msg),
            NeoError::DeserializationError(msg) => PythonError::SerializationError(msg),
            NeoError::NetworkError(msg) => PythonError::NetworkError(msg),
            NeoError::TimeoutError => PythonError::TimeoutError("Operation timed out".to_string()),
            NeoError::AuthenticationError(msg) => PythonError::AuthenticationError(msg),
            NeoError::AuthorizationError(msg) => PythonError::AuthorizationError(msg),
            NeoError::ServiceNotFound(service) => PythonError::ServiceNotFound(service),
            NeoError::MethodNotFound(method) => PythonError::MethodNotFound(method),
            NeoError::InternalError(msg) => PythonError::InternalError(msg),
        }
    }
}

impl From<PythonError> for PyErr {
    fn from(err: PythonError) -> Self {
        match err {
            PythonError::InvalidArgument(msg) => PyValueError::new_err(msg),
            PythonError::TimeoutError(msg) => PyTimeoutError::new_err(msg),
            _ => PyRuntimeError::new_err(err.to_string()),
        }
    }
}

/// Python configuration for Neo client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonClientConfig {
    pub server_address: String,
    pub timeout_seconds: f64,
    pub max_concurrent_requests: usize,
    pub enable_compression: bool,
    pub enable_auth: bool,
    pub auth_token: Option<String>,
}

impl Default for PythonClientConfig {
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

impl From<PythonClientConfig> for NeoConfig {
    fn from(config: PythonClientConfig) -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            rpc_timeout: Duration::from_secs_f64(config.timeout_seconds),
            connect_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(30),
            max_concurrent_requests: config.max_concurrent_requests,
            enable_compression: config.enable_compression,
            security: neo_protocol::SecurityConfig::default(),
        }
    }
}

/// Python configuration for Neo server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonServerConfig {
    pub address: String,
    pub max_concurrent_requests: usize,
    pub enable_compression: bool,
    pub enable_auth: bool,
    pub heartbeat_interval_seconds: f64,
}

impl Default for PythonServerConfig {
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

impl From<PythonServerConfig> for NeoConfig {
    fn from(config: PythonServerConfig) -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            rpc_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs_f64(config.heartbeat_interval_seconds),
            max_concurrent_requests: config.max_concurrent_requests,
            enable_compression: config.enable_compression,
            security: neo_protocol::SecurityConfig::default(),
        }
    }
}

/// Python service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonServiceDefinition {
    pub name: String,
    pub version: String,
    pub namespace: String,
    pub methods: HashMap<String, PythonMethodDefinition>,
    pub events: HashMap<String, PythonEventDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonMethodDefinition {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub timeout_seconds: f64,
    pub queue: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEventDefinition {
    pub name: String,
    pub topic: String,
    pub partition_key: Option<String>,
    pub description: Option<String>,
}

/// Python RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRpcRequest {
    pub service_name: String,
    pub method_name: String,
    pub correlation_id: u64,
    pub payload: Vec<u8>,
    pub timeout_seconds: Option<f64>,
    pub timestamp: u64,
}

impl From<PythonRpcRequest> for RpcRequest {
    fn from(req: PythonRpcRequest) -> Self {
        Self {
            service_name: req.service_name,
            method_name: req.method_name,
            correlation_id: req.correlation_id,
            payload: bytes::Bytes::from(req.payload),
            timeout: req.timeout_seconds.map(Duration::from_secs_f64),
        }
    }
}

/// Python RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonRpcResponse {
    pub correlation_id: u64,
    pub payload: Vec<u8>,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl From<RpcResponse> for PythonRpcResponse {
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

/// Python event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonEventMessage {
    pub topic: String,
    pub partition_key: Option<String>,
    pub payload: Vec<u8>,
    pub headers: Option<HashMap<String, String>>,
    pub timestamp: u64,
}

impl From<PythonEventMessage> for EventMessage {
    fn from(event: PythonEventMessage) -> Self {
        Self {
            topic: event.topic,
            partition_key: event.partition_key,
            payload: bytes::Bytes::from(event.payload),
            timestamp: SystemTime::now(),
        }
    }
}

/// Python heartbeat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonHeartbeatMessage {
    pub client_id: String,
    pub timestamp: u64,
}

impl From<PythonHeartbeatMessage> for HeartbeatMessage {
    fn from(heartbeat: PythonHeartbeatMessage) -> Self {
        Self {
            client_id: heartbeat.client_id,
            timestamp: SystemTime::now(),
        }
    }
}

/// Python metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonMetricsSnapshot {
    pub rpc_requests_total: u64,
    pub rpc_responses_total: u64,
    pub rpc_errors_total: u64,
    pub active_connections: usize,
    pub bytes_sent_total: u64,
    pub bytes_received_total: u64,
    pub service_registrations: usize,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: u64,
    pub uptime_seconds: u64,
    pub method_stats: HashMap<String, PythonMethodStats>,
    pub latency_stats: Option<PythonLatencyStats>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonMethodStats {
    pub invocations: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonLatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub mean_ms: f64,
}

impl From<MetricsSnapshot> for PythonMetricsSnapshot {
    fn from(snapshot: MetricsSnapshot) -> Self {
        let method_stats = snapshot.method_stats.into_iter()
            .map(|(k, v)| (k, PythonMethodStats {
                invocations: v.invocations,
                errors: v.errors,
            }))
            .collect();

        let latency_stats = snapshot.latency_stats.map(|ls| PythonLatencyStats {
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
            active_connections: snapshot.active_connections,
            bytes_sent_total: snapshot.bytes_sent_total,
            bytes_received_total: snapshot.bytes_received_total,
            service_registrations: snapshot.service_registrations,
            memory_usage_bytes: snapshot.memory_usage_bytes,
            cpu_usage_percent: snapshot.cpu_usage_percent,
            uptime_seconds: snapshot.uptime_seconds,
            method_stats,
            latency_stats,
            timestamp: snapshot.timestamp,
        }
    }
}

/// Python health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonHealthStatus {
    pub status: String,
    pub timestamp: u64,
    pub checks: HashMap<String, PythonCheckResult>,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonCheckResult {
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: u64,
}

impl From<HealthStatus> for PythonHealthStatus {
    fn from(status: HealthStatus) -> Self {
        let checks = status.checks.into_iter()
            .map(|(k, v)| (k, PythonCheckResult {
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

/// Neo client for Python
#[pyclass]
pub struct NeoClient {
    config: PythonClientConfig,
    runtime: Arc<Runtime>,
    client: Arc<Mutex<Option<neo_protocol::NeoClient>>>,
    request_id: Arc<Mutex<u64>>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<RpcResponse, NeoError>>>>>,
}

#[pymethods]
impl NeoClient {
    #[new]
    fn new(config: Option<PythonClientConfig>) -> PyResult<Self> {
        let config = config.unwrap_or_default();
        let runtime = Arc::new(Runtime::new().map_err(|e| PythonError::RuntimeError(e.to_string()))?);
        
        Ok(Self {
            config,
            runtime,
            client: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn connect(&self) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let config: NeoConfig = self.config.clone().into();
        
        runtime.block_on(async {
            let client = neo_protocol::NeoClient::new(config).await
                .map_err(PythonError::from)?;
            
            client.connect("127.0.0.1:8080".parse().unwrap()).await
                .map_err(PythonError::from)?;
            
            *self.client.lock().unwrap() = Some(client);
            Ok(())
        })
    }

    fn call(&self, service_name: String, method_name: String, payload: Vec<u8>) -> PyResult<Vec<u8>> {
        let runtime = self.runtime.clone();
        let client = self.client.clone();
        let request_id = self.request_id.clone();
        let pending_requests = self.pending_requests.clone();
        
        runtime.block_on(async {
            let mut client_guard = client.lock().unwrap();
            let client = client_guard.as_mut().ok_or_else(|| PythonError::RuntimeError("Client not connected".to_string()))?;
            
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
                bytes::Bytes::from(payload),
                Some(Duration::from_secs_f64(self.config.timeout_seconds)),
            ).await.map_err(PythonError::from)?;
            
            pending_requests.lock().unwrap().remove(&correlation_id);
            
            Ok(response.to_vec())
        })
    }

    fn publish_event(&self, event: PythonEventMessage) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let client = self.client.clone();
        
        runtime.block_on(async {
            let mut client_guard = client.lock().unwrap();
            let client = client_guard.as_mut().ok_or_else(|| PythonError::RuntimeError("Client not connected".to_string()))?;
            
            let event: EventMessage = event.into();
            client.publish_event(&event).await.map_err(PythonError::from)?;
            
            Ok(())
        })
    }

    fn close(&self) -> PyResult<()> {
        let mut client_guard = self.client.lock().unwrap();
        *client_guard = None;
        Ok(())
    }

    fn __enter__(&self) -> PyResult<&Self> {
        self.connect()?;
        Ok(self)
    }

    fn __exit__(&self, _exc_type: PyObject, _exc_val: PyObject, _exc_tb: PyObject) -> PyResult<()> {
        self.close()
    }
}

/// Neo server for Python
#[pyclass]
pub struct NeoServer {
    config: PythonServerConfig,
    runtime: Arc<Runtime>,
    server: Arc<Mutex<Option<neo_protocol::NeoServer>>>,
    services: Arc<Mutex<HashMap<String, PythonServiceDefinition>>>,
    handlers: Arc<Mutex<HashMap<String, PyObject>>>,
}

#[pymethods]
impl NeoServer {
    #[new]
    fn new(config: Option<PythonServerConfig>) -> PyResult<Self> {
        let config = config.unwrap_or_default();
        let runtime = Arc::new(Runtime::new().map_err(|e| PythonError::RuntimeError(e.to_string()))?);
        
        Ok(Self {
            config,
            runtime,
            server: Arc::new(Mutex::new(None)),
            services: Arc::new(Mutex::new(HashMap::new())),
            handlers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn register_service(&self, service: PythonServiceDefinition, handlers: PyObject) -> PyResult<()> {
        let services = self.services.clone();
        let handlers_map = self.handlers.clone();
        
        services.lock().unwrap().insert(service.name.clone(), service);
        handlers_map.lock().unwrap().insert(service.name.clone(), handlers);
        
        Ok(())
    }

    fn start(&self) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let config: NeoConfig = self.config.clone().into();
        let server = self.server.clone();
        
        runtime.block_on(async {
            let mut server_instance = neo_protocol::NeoServer::new(config).await
                .map_err(PythonError::from)?;
            
            server_instance.start("127.0.0.1:8080".parse().unwrap()).await
                .map_err(PythonError::from)?;
            
            *server.lock().unwrap() = Some(server_instance);
            Ok(())
        })
    }

    fn stop(&self) -> PyResult<()> {
        let mut server_guard = self.server.lock().unwrap();
        *server_guard = None;
        Ok(())
    }

    fn __enter__(&self) -> PyResult<&Self> {
        self.start()?;
        Ok(self)
    }

    fn __exit__(&self, _exc_type: PyObject, _exc_val: PyObject, _exc_tb: PyObject) -> PyResult<()> {
        self.stop()
    }
}

/// Metrics collector for Python
#[pyclass]
pub struct NeoMetricsCollector {
    collector: Arc<Mutex<MetricsCollector>>,
}

#[pymethods]
impl NeoMetricsCollector {
    #[new]
    fn new() -> PyResult<Self> {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        Ok(Self {
            collector: Arc::new(Mutex::new(collector)),
        })
    }

    fn start(&self) -> PyResult<()> {
        let collector = self.collector.clone();
        let runtime = Runtime::new().map_err(|e| PythonError::RuntimeError(e.to_string()))?;
        
        runtime.block_on(async {
            collector.lock().unwrap().start().await.map_err(PythonError::from)?;
            Ok(())
        })
    }

    fn get_snapshot(&self) -> PyResult<PythonMetricsSnapshot> {
        let collector = self.collector.lock().unwrap();
        let snapshot = collector.get_metrics_snapshot();
        Ok(snapshot.into())
    }

    fn get_health_status(&self) -> PyResult<PythonHealthStatus> {
        let collector = self.collector.clone();
        let runtime = Runtime::new().map_err(|e| PythonError::RuntimeError(e.to_string()))?;
        
        runtime.block_on(async {
            let status = collector.lock().unwrap().get_health_status().await;
            Ok(status.into())
        })
    }

    fn record_rpc_request(&self) {
        self.collector.lock().unwrap().record_rpc_request();
    }

    fn record_rpc_response(&self, latency_ms: f64) {
        self.collector.lock().unwrap().record_rpc_response(Duration::from_secs_f64(latency_ms / 1000.0));
    }

    fn record_rpc_error(&self, method_name: String) {
        self.collector.lock().unwrap().record_rpc_error(&method_name);
    }

    fn record_bytes_sent(&self, bytes: u64) {
        self.collector.lock().unwrap().record_bytes_sent(bytes);
    }

    fn record_bytes_received(&self, bytes: u64) {
        self.collector.lock().unwrap().record_bytes_received(bytes);
    }

    fn increment_connections(&self) {
        self.collector.lock().unwrap().increment_connections();
    }

    fn decrement_connections(&self) {
        self.collector.lock().unwrap().decrement_connections();
    }

    fn record_service_registration(&self) {
        self.collector.lock().unwrap().record_service_registration();
    }
}

/// Utility functions
#[pyfunction]
fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyfunction]
fn get_protocol_version() -> u8 {
    neo_protocol::NEO_PROTOCOL_VERSION
}

#[pyfunction]
fn get_max_message_size() -> usize {
    neo_protocol::MAX_MESSAGE_SIZE
}

/// Python module definition
#[pymodule]
fn neo_protocol(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<NeoClient>()?;
    m.add_class::<NeoServer>()?;
    m.add_class::<NeoMetricsCollector>()?;
    
    m.add_function(wrap_pyfunction!(get_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_protocol_version, m)?)?;
    m.add_function(wrap_pyfunction!(get_max_message_size, m)?)?;
    
    m.add("VERSION", env!("CARGO_PKG_VERSION"))?;
    m.add("PROTOCOL_VERSION", neo_protocol::NEO_PROTOCOL_VERSION)?;
    m.add("MAX_MESSAGE_SIZE", neo_protocol::MAX_MESSAGE_SIZE)?;
    
    Ok(())
}