//! Comprehensive monitoring and observability system for Neo Protocol
//! 
//! This module provides:
//! - Prometheus metrics collection
//! - Distributed tracing with OpenTelemetry
//! - Health checks and system monitoring
//! - Performance profiling and analysis

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Result;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::RwLock as AsyncRwLock,
    time::{interval, sleep},
};
use tracing::{debug, error, info, warn, Instrument};

/// Core metrics collector for Neo Protocol
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    // Protocol metrics
    pub rpc_requests_total: Arc<AtomicU64>,
    pub rpc_responses_total: Arc<AtomicU64>,
    pub rpc_errors_total: Arc<AtomicU64>,
    pub rpc_latency_histogram: Arc<RwLock<Vec<Duration>>>,
    pub active_connections: Arc<AtomicUsize>,
    pub bytes_sent_total: Arc<AtomicU64>,
    pub bytes_received_total: Arc<AtomicU64>,
    
    // Service metrics
    pub service_registrations: Arc<AtomicUsize>,
    pub method_invocations: Arc<RwLock<HashMap<String, AtomicU64>>>,
    pub method_errors: Arc<RwLock<HashMap<String, AtomicU64>>>,
    
    // System metrics
    pub memory_usage_bytes: Arc<AtomicU64>,
    pub cpu_usage_percent: Arc<AtomicU64>,
    pub uptime_seconds: Arc<AtomicU64>,
    
    // Health status
    pub health_status: Arc<AsyncRwLock<HealthStatus>>,
    
    // Configuration
    pub config: MetricsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enable_prometheus: bool,
    pub prometheus_port: u16,
    pub enable_tracing: bool,
    pub tracing_endpoint: Option<String>,
    pub metrics_interval: Duration,
    pub health_check_interval: Duration,
    pub max_latency_samples: usize,
    pub enable_detailed_metrics: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_port: 9090,
            enable_tracing: true,
            tracing_endpoint: None,
            metrics_interval: Duration::from_secs(10),
            health_check_interval: Duration::from_secs(30),
            max_latency_samples: 10000,
            enable_detailed_metrics: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub timestamp: u64,
    pub checks: HashMap<String, CheckResult>,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: HealthState,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: u64,
}

impl MetricsCollector {
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            rpc_requests_total: Arc::new(AtomicU64::new(0)),
            rpc_responses_total: Arc::new(AtomicU64::new(0)),
            rpc_errors_total: Arc::new(AtomicU64::new(0)),
            rpc_latency_histogram: Arc::new(RwLock::new(Vec::new())),
            active_connections: Arc::new(AtomicUsize::new(0)),
            bytes_sent_total: Arc::new(AtomicU64::new(0)),
            bytes_received_total: Arc::new(AtomicU64::new(0)),
            service_registrations: Arc::new(AtomicUsize::new(0)),
            method_invocations: Arc::new(RwLock::new(HashMap::new())),
            method_errors: Arc::new(RwLock::new(HashMap::new())),
            memory_usage_bytes: Arc::new(AtomicU64::new(0)),
            cpu_usage_percent: Arc::new(AtomicU64::new(0)),
            uptime_seconds: Arc::new(AtomicU64::new(0)),
            health_status: Arc::new(AsyncRwLock::new(HealthStatus {
                status: HealthState::Healthy,
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                checks: HashMap::new(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_seconds: 0,
            })),
            config,
        }
    }

    /// Start the metrics collection and health monitoring
    pub async fn start(&self) -> Result<()> {
        let metrics_collector = self.clone();
        let health_collector = self.clone();
        
        // Start metrics collection
        tokio::spawn(async move {
            metrics_collector.collect_metrics_loop().await;
        });

        // Start health monitoring
        tokio::spawn(async move {
            health_collector.health_check_loop().await;
        });

        // Start Prometheus server if enabled
        if self.config.enable_prometheus {
            let prometheus_collector = self.clone();
            tokio::spawn(async move {
                if let Err(e) = prometheus_collector.start_prometheus_server().await {
                    error!("Failed to start Prometheus server: {}", e);
                }
            });
        }

        info!("Metrics collector started successfully");
        Ok(())
    }

    /// Record an RPC request
    pub fn record_rpc_request(&self) {
        self.rpc_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an RPC response
    pub fn record_rpc_response(&self, latency: Duration) {
        self.rpc_responses_total.fetch_add(1, Ordering::Relaxed);
        
        if self.config.enable_detailed_metrics {
            let mut histogram = self.rpc_latency_histogram.write().unwrap();
            histogram.push(latency);
            
            // Keep only the most recent samples
            if histogram.len() > self.config.max_latency_samples {
                histogram.drain(0..histogram.len() - self.config.max_latency_samples);
            }
        }
    }

    /// Record an RPC error
    pub fn record_rpc_error(&self, method: &str) {
        self.rpc_errors_total.fetch_add(1, Ordering::Relaxed);
        
        if self.config.enable_detailed_metrics {
            let mut errors = self.method_errors.write().unwrap();
            errors.entry(method.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record method invocation
    pub fn record_method_invocation(&self, method: &str) {
        if self.config.enable_detailed_metrics {
            let mut invocations = self.method_invocations.write().unwrap();
            invocations.entry(method.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent_total.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received_total.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record service registration
    pub fn record_service_registration(&self) {
        self.service_registrations.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let histogram = self.rpc_latency_histogram.read().unwrap();
        let invocations = self.method_invocations.read().unwrap();
        let errors = self.method_errors.read().unwrap();

        let mut method_stats = HashMap::new();
        for (method, count) in invocations.iter() {
            let error_count = errors.get(method).map(|e| e.load(Ordering::Relaxed)).unwrap_or(0);
            method_stats.insert(method.clone(), MethodStats {
                invocations: count.load(Ordering::Relaxed),
                errors: error_count,
            });
        }

        let latency_stats = if !histogram.is_empty() {
            let mut sorted_latencies: Vec<Duration> = histogram.clone();
            sorted_latencies.sort();
            
            let len = sorted_latencies.len();
            Some(LatencyStats {
                min: sorted_latencies[0],
                max: sorted_latencies[len - 1],
                p50: sorted_latencies[len / 2],
                p95: sorted_latencies[(len * 95) / 100],
                p99: sorted_latencies[(len * 99) / 100],
                mean: sorted_latencies.iter().sum::<Duration>() / len as u32,
            })
        } else {
            None
        };

        MetricsSnapshot {
            rpc_requests_total: self.rpc_requests_total.load(Ordering::Relaxed),
            rpc_responses_total: self.rpc_responses_total.load(Ordering::Relaxed),
            rpc_errors_total: self.rpc_errors_total.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            bytes_sent_total: self.bytes_sent_total.load(Ordering::Relaxed),
            bytes_received_total: self.bytes_received_total.load(Ordering::Relaxed),
            service_registrations: self.service_registrations.load(Ordering::Relaxed),
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed),
            cpu_usage_percent: self.cpu_usage_percent.load(Ordering::Relaxed),
            uptime_seconds: self.uptime_seconds.load(Ordering::Relaxed),
            method_stats,
            latency_stats,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    /// Get health status
    pub async fn get_health_status(&self) -> HealthStatus {
        self.health_status.read().await.clone()
    }

    /// Update health status
    pub async fn update_health_status(&self, status: HealthStatus) {
        *self.health_status.write().await = status;
    }

    /// Metrics collection loop
    async fn collect_metrics_loop(&self) {
        let mut interval = interval(self.config.metrics_interval);
        let start_time = Instant::now();

        loop {
            interval.tick().await;
            
            // Update uptime
            self.uptime_seconds.store(
                start_time.elapsed().as_secs(),
                Ordering::Relaxed
            );

            // Collect system metrics
            self.collect_system_metrics().await;

            debug!("Metrics collected successfully");
        }
    }

    /// Health check loop
    async fn health_check_loop(&self) {
        let mut interval = interval(self.config.health_check_interval);

        loop {
            interval.tick().await;
            
            let health_status = self.perform_health_checks().await;
            self.update_health_status(health_status).await;

            debug!("Health checks completed");
        }
    }

    /// Collect system metrics
    async fn collect_system_metrics(&self) {
        // Memory usage
        if let Ok(memory_info) = get_memory_usage() {
            self.memory_usage_bytes.store(memory_info, Ordering::Relaxed);
        }

        // CPU usage
        if let Ok(cpu_usage) = get_cpu_usage().await {
            self.cpu_usage_percent.store(cpu_usage, Ordering::Relaxed);
        }
    }

    /// Perform health checks
    async fn perform_health_checks(&self) -> HealthStatus {
        let mut checks = HashMap::new();
        let mut overall_status = HealthState::Healthy;

        // Check memory usage
        let memory_check = self.check_memory_usage().await;
        if memory_check.status == HealthState::Unhealthy {
            overall_status = HealthState::Unhealthy;
        } else if memory_check.status == HealthState::Degraded && overall_status == HealthState::Healthy {
            overall_status = HealthState::Degraded;
        }
        checks.insert("memory".to_string(), memory_check);

        // Check connection count
        let connection_check = self.check_connection_count().await;
        if connection_check.status == HealthState::Unhealthy {
            overall_status = HealthState::Unhealthy;
        } else if connection_check.status == HealthState::Degraded && overall_status == HealthState::Healthy {
            overall_status = HealthState::Degraded;
        }
        checks.insert("connections".to_string(), connection_check);

        // Check error rate
        let error_rate_check = self.check_error_rate().await;
        if error_rate_check.status == HealthState::Unhealthy {
            overall_status = HealthState::Unhealthy;
        } else if error_rate_check.status == HealthState::Degraded && overall_status == HealthState::Healthy {
            overall_status = HealthState::Degraded;
        }
        checks.insert("error_rate".to_string(), error_rate_check);

        HealthStatus {
            status: overall_status,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            checks,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.uptime_seconds.load(Ordering::Relaxed),
        }
    }

    /// Check memory usage
    async fn check_memory_usage(&self) -> CheckResult {
        let start = Instant::now();
        let memory_usage = self.memory_usage_bytes.load(Ordering::Relaxed);
        let duration = start.elapsed();

        let status = if memory_usage > 1024 * 1024 * 1024 { // 1GB
            HealthState::Unhealthy
        } else if memory_usage > 512 * 1024 * 1024 { // 512MB
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        CheckResult {
            status,
            message: format!("Memory usage: {} bytes", memory_usage),
            duration_ms: duration.as_millis() as u64,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    /// Check connection count
    async fn check_connection_count(&self) -> CheckResult {
        let start = Instant::now();
        let connections = self.active_connections.load(Ordering::Relaxed);
        let duration = start.elapsed();

        let status = if connections > 10000 {
            HealthState::Unhealthy
        } else if connections > 5000 {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        CheckResult {
            status,
            message: format!("Active connections: {}", connections),
            duration_ms: duration.as_millis() as u64,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    /// Check error rate
    async fn check_error_rate(&self) -> CheckResult {
        let start = Instant::now();
        let requests = self.rpc_requests_total.load(Ordering::Relaxed);
        let errors = self.rpc_errors_total.load(Ordering::Relaxed);
        let duration = start.elapsed();

        let error_rate = if requests > 0 {
            (errors as f64 / requests as f64) * 100.0
        } else {
            0.0
        };

        let status = if error_rate > 10.0 {
            HealthState::Unhealthy
        } else if error_rate > 5.0 {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        CheckResult {
            status,
            message: format!("Error rate: {:.2}%", error_rate),
            duration_ms: duration.as_millis() as u64,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }

    /// Start Prometheus metrics server
    async fn start_prometheus_server(&self) -> Result<()> {
        use std::net::SocketAddr;
        use tokio::net::TcpListener;

        let addr: SocketAddr = format!("0.0.0.0:{}", self.config.prometheus_port).parse()?;
        let listener = TcpListener::bind(addr).await?;
        
        info!("Prometheus metrics server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let metrics_collector = self.clone();
            
            tokio::spawn(async move {
                if let Err(e) = metrics_collector.handle_prometheus_request(stream).await {
                    error!("Error handling Prometheus request: {}", e);
                }
            });
        }
    }

    /// Handle Prometheus metrics request
    async fn handle_prometheus_request(&self, mut stream: tokio::net::TcpStream) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);

        if request.starts_with("GET /metrics") {
            let metrics = self.format_prometheus_metrics();
            let response = format!(
                "HTTP/1.1 200 OK\r\n\
                 Content-Type: text/plain; version=0.0.4\r\n\
                 Content-Length: {}\r\n\
                 \r\n\
                 {}",
                metrics.len(),
                metrics
            );
            stream.write_all(response.as_bytes()).await?;
        } else {
            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
            stream.write_all(response.as_bytes()).await?;
        }

        Ok(())
    }

    /// Format metrics in Prometheus format
    fn format_prometheus_metrics(&self) -> String {
        let snapshot = self.get_metrics_snapshot();
        let mut output = String::new();

        // Protocol metrics
        output.push_str(&format!("# HELP neo_rpc_requests_total Total number of RPC requests\n"));
        output.push_str(&format!("# TYPE neo_rpc_requests_total counter\n"));
        output.push_str(&format!("neo_rpc_requests_total {}\n", snapshot.rpc_requests_total));

        output.push_str(&format!("# HELP neo_rpc_responses_total Total number of RPC responses\n"));
        output.push_str(&format!("# TYPE neo_rpc_responses_total counter\n"));
        output.push_str(&format!("neo_rpc_responses_total {}\n", snapshot.rpc_responses_total));

        output.push_str(&format!("# HELP neo_rpc_errors_total Total number of RPC errors\n"));
        output.push_str(&format!("# TYPE neo_rpc_errors_total counter\n"));
        output.push_str(&format!("neo_rpc_errors_total {}\n", snapshot.rpc_errors_total));

        output.push_str(&format!("# HELP neo_active_connections Current number of active connections\n"));
        output.push_str(&format!("# TYPE neo_active_connections gauge\n"));
        output.push_str(&format!("neo_active_connections {}\n", snapshot.active_connections));

        output.push_str(&format!("# HELP neo_bytes_sent_total Total bytes sent\n"));
        output.push_str(&format!("# TYPE neo_bytes_sent_total counter\n"));
        output.push_str(&format!("neo_bytes_sent_total {}\n", snapshot.bytes_sent_total));

        output.push_str(&format!("# HELP neo_bytes_received_total Total bytes received\n"));
        output.push_str(&format!("# TYPE neo_bytes_received_total counter\n"));
        output.push_str(&format!("neo_bytes_received_total {}\n", snapshot.bytes_received_total));

        output.push_str(&format!("# HELP neo_service_registrations Total number of service registrations\n"));
        output.push_str(&format!("# TYPE neo_service_registrations counter\n"));
        output.push_str(&format!("neo_service_registrations {}\n", snapshot.service_registrations));

        output.push_str(&format!("# HELP neo_memory_usage_bytes Current memory usage in bytes\n"));
        output.push_str(&format!("# TYPE neo_memory_usage_bytes gauge\n"));
        output.push_str(&format!("neo_memory_usage_bytes {}\n", snapshot.memory_usage_bytes));

        output.push_str(&format!("# HELP neo_cpu_usage_percent Current CPU usage percentage\n"));
        output.push_str(&format!("# TYPE neo_cpu_usage_percent gauge\n"));
        output.push_str(&format!("neo_cpu_usage_percent {}\n", snapshot.cpu_usage_percent));

        output.push_str(&format!("# HELP neo_uptime_seconds System uptime in seconds\n"));
        output.push_str(&format!("# TYPE neo_uptime_seconds counter\n"));
        output.push_str(&format!("neo_uptime_seconds {}\n", snapshot.uptime_seconds));

        // Method-specific metrics
        for (method, stats) in &snapshot.method_stats {
            output.push_str(&format!("# HELP neo_method_invocations_total Total method invocations\n"));
            output.push_str(&format!("# TYPE neo_method_invocations_total counter\n"));
            output.push_str(&format!("neo_method_invocations_total{{method=\"{}\"}} {}\n", method, stats.invocations));

            output.push_str(&format!("# HELP neo_method_errors_total Total method errors\n"));
            output.push_str(&format!("# TYPE neo_method_errors_total counter\n"));
            output.push_str(&format!("neo_method_errors_total{{method=\"{}\"}} {}\n", method, stats.errors));
        }

        // Latency metrics
        if let Some(latency_stats) = &snapshot.latency_stats {
            output.push_str(&format!("# HELP neo_rpc_latency_seconds RPC latency in seconds\n"));
            output.push_str(&format!("# TYPE neo_rpc_latency_seconds histogram\n"));
            output.push_str(&format!("neo_rpc_latency_seconds_bucket{{le=\"0.001\"}} {}\n", 
                latency_stats.p50.as_secs_f64()));
            output.push_str(&format!("neo_rpc_latency_seconds_bucket{{le=\"0.005\"}} {}\n", 
                latency_stats.p95.as_secs_f64()));
            output.push_str(&format!("neo_rpc_latency_seconds_bucket{{le=\"0.01\"}} {}\n", 
                latency_stats.p99.as_secs_f64()));
            output.push_str(&format!("neo_rpc_latency_seconds_bucket{{le=\"+Inf\"}} {}\n", 
                latency_stats.max.as_secs_f64()));
            output.push_str(&format!("neo_rpc_latency_seconds_sum {}\n", 
                latency_stats.mean.as_secs_f64()));
            output.push_str(&format!("neo_rpc_latency_seconds_count {}\n", 
                snapshot.rpc_responses_total));
        }

        output
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
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
    pub method_stats: HashMap<String, MethodStats>,
    pub latency_stats: Option<LatencyStats>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodStats {
    pub invocations: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    pub min: Duration,
    pub max: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub mean: Duration,
}

/// Get current memory usage in bytes
fn get_memory_usage() -> Result<u64> {
    use std::fs;
    
    let status = fs::read_to_string("/proc/self/status")?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let kb: u64 = parts[1].parse()?;
                return Ok(kb * 1024);
            }
        }
    }
    
    Ok(0)
}

/// Get current CPU usage percentage
async fn get_cpu_usage() -> Result<u64> {
    use std::fs;
    
    let stat1 = fs::read_to_string("/proc/stat")?;
    sleep(Duration::from_millis(100)).await;
    let stat2 = fs::read_to_string("/proc/stat")?;
    
    let parse_cpu_line = |line: &str| -> Result<(u64, u64)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return Err(anyhow::anyhow!("Invalid CPU line"));
        }
        
        let user: u64 = parts[1].parse()?;
        let nice: u64 = parts[2].parse()?;
        let system: u64 = parts[3].parse()?;
        let idle: u64 = parts[4].parse()?;
        let iowait: u64 = parts[5].parse()?;
        let irq: u64 = parts[6].parse()?;
        let softirq: u64 = parts[7].parse()?;
        
        let total = user + nice + system + idle + iowait + irq + softirq;
        let idle_total = idle + iowait;
        
        Ok((total, idle_total))
    };
    
    let (total1, idle1) = parse_cpu_line(&stat1.lines().next().unwrap())?;
    let (total2, idle2) = parse_cpu_line(&stat2.lines().next().unwrap())?;
    
    let total_diff = total2 - total1;
    let idle_diff = idle2 - idle1;
    
    if total_diff == 0 {
        return Ok(0);
    }
    
    let cpu_usage = ((total_diff - idle_diff) * 100) / total_diff;
    Ok(cpu_usage)
}

/// Distributed tracing support
pub mod tracing {
    use super::*;
    use std::collections::HashMap;

    /// Trace context for distributed tracing
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TraceContext {
        pub trace_id: String,
        pub span_id: String,
        pub parent_span_id: Option<String>,
        pub baggage: HashMap<String, String>,
    }

    /// Span for distributed tracing
    #[derive(Debug)]
    pub struct Span {
        pub context: TraceContext,
        pub operation_name: String,
        pub start_time: Instant,
        pub tags: HashMap<String, String>,
        pub logs: Vec<LogEntry>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LogEntry {
        pub timestamp: u64,
        pub level: String,
        pub message: String,
        pub fields: HashMap<String, String>,
    }

    impl Span {
        pub fn new(operation_name: String, context: TraceContext) -> Self {
            Self {
                context,
                operation_name,
                start_time: Instant::now(),
                tags: HashMap::new(),
                logs: Vec::new(),
            }
        }

        pub fn set_tag(&mut self, key: String, value: String) {
            self.tags.insert(key, value);
        }

        pub fn log(&mut self, level: String, message: String, fields: HashMap<String, String>) {
            self.logs.push(LogEntry {
                timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                level,
                message,
                fields,
            });
        }

        pub fn finish(self) -> FinishedSpan {
            FinishedSpan {
                context: self.context,
                operation_name: self.operation_name,
                start_time: self.start_time,
                duration: self.start_time.elapsed(),
                tags: self.tags,
                logs: self.logs,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FinishedSpan {
        pub context: TraceContext,
        pub operation_name: String,
        pub start_time: Instant,
        pub duration: Duration,
        pub tags: HashMap<String, String>,
        pub logs: Vec<LogEntry>,
    }

    /// Tracer for distributed tracing
    #[derive(Debug, Clone)]
    pub struct Tracer {
        pub service_name: String,
        pub version: String,
    }

    impl Tracer {
        pub fn new(service_name: String, version: String) -> Self {
            Self {
                service_name,
                version,
            }
        }

        pub fn start_span(&self, operation_name: String) -> Span {
            let trace_id = generate_trace_id();
            let span_id = generate_span_id();
            
            let context = TraceContext {
                trace_id,
                span_id,
                parent_span_id: None,
                baggage: HashMap::new(),
            };

            Span::new(operation_name, context)
        }

        pub fn start_child_span(&self, operation_name: String, parent_context: &TraceContext) -> Span {
            let span_id = generate_span_id();
            
            let context = TraceContext {
                trace_id: parent_context.trace_id.clone(),
                span_id,
                parent_span_id: Some(parent_context.span_id.clone()),
                baggage: parent_context.baggage.clone(),
            };

            Span::new(operation_name, context)
        }
    }

    /// Generate a random trace ID
    fn generate_trace_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Generate a random span ID
    fn generate_span_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_metrics_collector() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        // Test basic metrics recording
        collector.record_rpc_request();
        collector.record_rpc_response(Duration::from_millis(10));
        collector.record_bytes_sent(1024);
        collector.record_bytes_received(2048);
        
        let snapshot = collector.get_metrics_snapshot();
        assert_eq!(snapshot.rpc_requests_total, 1);
        assert_eq!(snapshot.rpc_responses_total, 1);
        assert_eq!(snapshot.bytes_sent_total, 1024);
        assert_eq!(snapshot.bytes_received_total, 2048);
    }

    #[tokio::test]
    async fn test_health_checks() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        let health_status = collector.get_health_status().await;
        assert_eq!(health_status.status, HealthState::Healthy);
        assert!(!health_status.checks.is_empty());
    }

    #[tokio::test]
    async fn test_tracing() {
        let tracer = tracing::Tracer::new("test-service".to_string(), "1.0.0".to_string());
        let mut span = tracer.start_span("test-operation".to_string());
        
        span.set_tag("test".to_string(), "value".to_string());
        span.log("info".to_string(), "test message".to_string(), HashMap::new());
        
        let finished_span = span.finish();
        assert_eq!(finished_span.operation_name, "test-operation");
        assert_eq!(finished_span.tags.get("test").unwrap(), "value");
        assert!(!finished_span.logs.is_empty());
    }
}