//! Advanced monitoring and metrics collection system
//! 
//! This module provides comprehensive monitoring capabilities with real-time metrics,
//! performance analytics, and intelligent alerting.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, broadcast};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::{Result, Error};

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enable_monitoring: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Metrics retention period
    pub retention_period: Duration,
    /// Enable real-time metrics
    pub enable_realtime: bool,
    /// Enable performance analytics
    pub enable_analytics: bool,
    /// Enable alerting
    pub enable_alerting: bool,
    /// Alert thresholds
    pub alert_thresholds: AlertThresholds,
    /// Metrics export configuration
    pub export_config: MetricsExportConfig,
    /// Dashboard configuration
    pub dashboard_config: DashboardConfig,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            collection_interval: Duration::from_secs(1),
            retention_period: Duration::from_secs(3600), // 1 hour
            enable_realtime: true,
            enable_analytics: true,
            enable_alerting: true,
            alert_thresholds: AlertThresholds::default(),
            export_config: MetricsExportConfig::default(),
            dashboard_config: DashboardConfig::default(),
        }
    }
}

/// Alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU usage threshold (%)
    pub cpu_threshold: f64,
    /// Memory usage threshold (%)
    pub memory_threshold: f64,
    /// Disk usage threshold (%)
    pub disk_threshold: f64,
    /// Network latency threshold (ms)
    pub latency_threshold: f64,
    /// Error rate threshold (%)
    pub error_rate_threshold: f64,
    /// Throughput threshold (messages/sec)
    pub throughput_threshold: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            disk_threshold: 90.0,
            latency_threshold: 100.0,
            error_rate_threshold: 5.0,
            throughput_threshold: 1000.0,
        }
    }
}

/// Metrics export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExportConfig {
    /// Enable Prometheus export
    pub enable_prometheus: bool,
    /// Prometheus endpoint
    pub prometheus_endpoint: String,
    /// Enable InfluxDB export
    pub enable_influxdb: bool,
    /// InfluxDB configuration
    pub influxdb_config: Option<InfluxDBConfig>,
    /// Enable Graphite export
    pub enable_graphite: bool,
    /// Graphite configuration
    pub graphite_config: Option<GraphiteConfig>,
}

impl Default for MetricsExportConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            prometheus_endpoint: "0.0.0.0:9090".to_string(),
            enable_influxdb: false,
            influxdb_config: None,
            enable_graphite: false,
            graphite_config: None,
        }
    }
}

/// InfluxDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluxDBConfig {
    /// InfluxDB URL
    pub url: String,
    /// Database name
    pub database: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Retention policy
    pub retention_policy: String,
}

/// Graphite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphiteConfig {
    /// Graphite host
    pub host: String,
    /// Graphite port
    pub port: u16,
    /// Metric prefix
    pub prefix: String,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Enable web dashboard
    pub enable_web_dashboard: bool,
    /// Dashboard port
    pub dashboard_port: u16,
    /// Dashboard host
    pub dashboard_host: String,
    /// Enable real-time updates
    pub enable_realtime_updates: bool,
    /// Update interval
    pub update_interval: Duration,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enable_web_dashboard: true,
            dashboard_port: 3000,
            dashboard_host: "0.0.0.0".to_string(),
            enable_realtime_updates: true,
            update_interval: Duration::from_millis(100),
        }
    }
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    /// Counter metric
    Counter,
    /// Gauge metric
    Gauge,
    /// Histogram metric
    Histogram,
    /// Summary metric
    Summary,
}

/// Metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    /// Counter value
    Counter(u64),
    /// Gauge value
    Gauge(f64),
    /// Histogram values
    Histogram(Vec<f64>),
    /// Summary values
    Summary { count: u64, sum: f64 },
}

/// Metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Metric value
    pub value: MetricValue,
    /// Labels
    pub labels: HashMap<String, String>,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Disk usage percentage
    pub disk_usage: f64,
    /// Network bytes received
    pub network_bytes_received: u64,
    /// Network bytes sent
    pub network_bytes_sent: u64,
    /// Load average
    pub load_average: [f64; 3],
    /// Uptime
    pub uptime: Duration,
}

/// Broker metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerMetrics {
    /// Total messages produced
    pub total_messages_produced: u64,
    /// Total messages consumed
    pub total_messages_consumed: u64,
    /// Total bytes produced
    pub total_bytes_produced: u64,
    /// Total bytes consumed
    pub total_bytes_consumed: u64,
    /// Active producers
    pub active_producers: usize,
    /// Active consumers
    pub active_consumers: usize,
    /// Active topics
    pub active_topics: usize,
    /// Active queues
    pub active_queues: usize,
    /// Consumer groups
    pub consumer_groups: usize,
    /// Average message latency (microseconds)
    pub avg_message_latency_us: u64,
    /// Average throughput (messages/second)
    pub avg_throughput: f64,
    /// Error rate (percentage)
    pub error_rate: f64,
}

/// Performance analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalytics {
    /// Latency percentiles
    pub latency_percentiles: LatencyPercentiles,
    /// Throughput trends
    pub throughput_trends: Vec<ThroughputTrend>,
    /// Resource utilization
    pub resource_utilization: ResourceUtilization,
    /// Performance predictions
    pub performance_predictions: PerformancePredictions,
}

/// Latency percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    /// P50 latency (microseconds)
    pub p50: u64,
    /// P90 latency (microseconds)
    pub p90: u64,
    /// P95 latency (microseconds)
    pub p95: u64,
    /// P99 latency (microseconds)
    pub p99: u64,
    /// P99.9 latency (microseconds)
    pub p999: u64,
}

/// Throughput trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputTrend {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Throughput (messages/second)
    pub throughput: f64,
    /// Trend direction
    pub trend: TrendDirection,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Increasing
    Increasing,
    /// Decreasing
    Decreasing,
    /// Stable
    Stable,
    /// Volatile
    Volatile,
}

/// Resource utilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    /// CPU utilization over time
    pub cpu_utilization: Vec<f64>,
    /// Memory utilization over time
    pub memory_utilization: Vec<f64>,
    /// Network utilization over time
    pub network_utilization: Vec<f64>,
    /// Disk utilization over time
    pub disk_utilization: Vec<f64>,
}

/// Performance predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformancePredictions {
    /// Predicted CPU usage
    pub predicted_cpu_usage: f64,
    /// Predicted memory usage
    pub predicted_memory_usage: f64,
    /// Predicted throughput
    pub predicted_throughput: f64,
    /// Predicted latency
    pub predicted_latency: f64,
    /// Confidence score
    pub confidence_score: f64,
}

/// Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert ID
    pub id: String,
    /// Alert name
    pub name: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Alert source
    pub source: String,
    /// Alert labels
    pub labels: HashMap<String, String>,
    /// Alert status
    pub status: AlertStatus,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Info
    Info,
    /// Warning
    Warning,
    /// Critical
    Critical,
    /// Emergency
    Emergency,
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Firing
    Firing,
    /// Resolved
    Resolved,
    /// Acknowledged
    Acknowledged,
}

/// High-performance monitoring system
pub struct MonitoringSystem {
    /// Monitoring configuration
    config: MonitoringConfig,
    /// Metrics storage
    metrics: Arc<RwLock<VecDeque<Metric>>>,
    /// System metrics
    system_metrics: Arc<RwLock<SystemMetrics>>,
    /// Broker metrics
    broker_metrics: Arc<RwLock<BrokerMetrics>>,
    /// Performance analytics
    performance_analytics: Arc<RwLock<PerformanceAnalytics>>,
    /// Active alerts
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Metrics collection task
    collection_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Analytics task
    analytics_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Alerting task
    alerting_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Metrics event sender
    metrics_sender: broadcast::Sender<Metric>,
    /// Alert event sender
    alert_sender: broadcast::Sender<Alert>,
}

impl MonitoringSystem {
    /// Create a new monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        let (metrics_sender, _) = broadcast::channel(1000);
        let (alert_sender, _) = broadcast::channel(100);
        
        Self {
            config,
            metrics: Arc::new(RwLock::new(VecDeque::new())),
            system_metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            broker_metrics: Arc::new(RwLock::new(BrokerMetrics::default())),
            performance_analytics: Arc::new(RwLock::new(PerformanceAnalytics::default())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            collection_task: Arc::new(Mutex::new(None)),
            analytics_task: Arc::new(Mutex::new(None)),
            alerting_task: Arc::new(Mutex::new(None)),
            metrics_sender,
            alert_sender,
        }
    }
    
    /// Start the monitoring system
    pub async fn start(&mut self) -> Result<()> {
        if !self.config.enable_monitoring {
            return Ok(());
        }
        
        info!("Starting monitoring system");
        
        // Start metrics collection task
        if self.config.enable_realtime {
            self.start_metrics_collection().await;
        }
        
        // Start analytics task
        if self.config.enable_analytics {
            self.start_analytics_task().await;
        }
        
        // Start alerting task
        if self.config.enable_alerting {
            self.start_alerting_task().await;
        }
        
        info!("Monitoring system started");
        Ok(())
    }
    
    /// Stop the monitoring system
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping monitoring system");
        
        // Stop all tasks
        if let Some(task) = self.collection_task.lock().unwrap().take() {
            task.abort();
        }
        
        if let Some(task) = self.analytics_task.lock().unwrap().take() {
            task.abort();
        }
        
        if let Some(task) = self.alerting_task.lock().unwrap().take() {
            task.abort();
        }
        
        info!("Monitoring system stopped");
        Ok(())
    }
    
    /// Record a metric
    pub async fn record_metric(&self, metric: Metric) -> Result<()> {
        // Add to metrics storage
        {
            let mut metrics = self.metrics.write().await;
            metrics.push_back(metric.clone());
            
            // Remove old metrics
            let cutoff_time = SystemTime::now() - self.config.retention_period;
            while let Some(front) = metrics.front() {
                if front.timestamp < cutoff_time {
                    metrics.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Broadcast metric event
        let _ = self.metrics_sender.send(metric);
        
        Ok(())
    }
    
    /// Update system metrics
    pub async fn update_system_metrics(&self, metrics: SystemMetrics) -> Result<()> {
        let mut system_metrics = self.system_metrics.write().await;
        *system_metrics = metrics;
        Ok(())
    }
    
    /// Update broker metrics
    pub async fn update_broker_metrics(&self, metrics: BrokerMetrics) -> Result<()> {
        let mut broker_metrics = self.broker_metrics.write().await;
        *broker_metrics = metrics;
        Ok(())
    }
    
    /// Get system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }
    
    /// Get broker metrics
    pub async fn get_broker_metrics(&self) -> BrokerMetrics {
        self.broker_metrics.read().await.clone()
    }
    
    /// Get performance analytics
    pub async fn get_performance_analytics(&self) -> PerformanceAnalytics {
        self.performance_analytics.read().await.clone()
    }
    
    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.values().cloned().collect()
    }
    
    /// Subscribe to metrics events
    pub fn subscribe_to_metrics(&self) -> broadcast::Receiver<Metric> {
        self.metrics_sender.subscribe()
    }
    
    /// Subscribe to alert events
    pub fn subscribe_to_alerts(&self) -> broadcast::Receiver<Alert> {
        self.alert_sender.subscribe()
    }
    
    /// Start metrics collection task
    async fn start_metrics_collection(&mut self) {
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let system_metrics = self.system_metrics.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(config.collection_interval);
            
            loop {
                interval.tick().await;
                
                // Collect system metrics
                let system_metrics_data = Self::collect_system_metrics().await;
                {
                    let mut metrics_guard = system_metrics.write().await;
                    *metrics_guard = system_metrics_data;
                }
                
                // Record system metrics
                let system_metrics_guard = system_metrics.read().await;
                let cpu_metric = Metric {
                    name: "system_cpu_usage".to_string(),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(system_metrics_guard.cpu_usage),
                    labels: HashMap::new(),
                    timestamp: SystemTime::now(),
                };
                
                let memory_metric = Metric {
                    name: "system_memory_usage".to_string(),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Gauge(system_metrics_guard.memory_usage),
                    labels: HashMap::new(),
                    timestamp: SystemTime::now(),
                };
                
                {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.push_back(cpu_metric);
                    metrics_guard.push_back(memory_metric);
                }
            }
        });
        
        let mut collection_task = self.collection_task.lock().unwrap();
        *collection_task = Some(handle);
    }
    
    /// Start analytics task
    async fn start_analytics_task(&mut self) {
        let config = self.config.clone();
        let metrics = self.metrics.clone();
        let performance_analytics = self.performance_analytics.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // Run every 10 seconds
            
            loop {
                interval.tick().await;
                
                // Perform analytics
                let analytics = Self::perform_analytics(&metrics).await;
                {
                    let mut analytics_guard = performance_analytics.write().await;
                    *analytics_guard = analytics;
                }
            }
        });
        
        let mut analytics_task = self.analytics_task.lock().unwrap();
        *analytics_task = Some(handle);
    }
    
    /// Start alerting task
    async fn start_alerting_task(&mut self) {
        let config = self.config.clone();
        let system_metrics = self.system_metrics.clone();
        let broker_metrics = self.broker_metrics.clone();
        let alerts = self.alerts.clone();
        let alert_sender = self.alert_sender.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Check every 5 seconds
            
            loop {
                interval.tick().await;
                
                // Check for alerts
                Self::check_alerts(
                    &config,
                    &system_metrics,
                    &broker_metrics,
                    &alerts,
                    &alert_sender,
                ).await;
            }
        });
        
        let mut alerting_task = self.alerting_task.lock().unwrap();
        *alerting_task = Some(handle);
    }
    
    /// Collect system metrics
    async fn collect_system_metrics() -> SystemMetrics {
        // In a real implementation, this would collect actual system metrics
        // For now, we'll return mock data
        SystemMetrics {
            cpu_usage: 45.0,
            memory_usage: 60.0,
            disk_usage: 30.0,
            network_bytes_received: 1024 * 1024,
            network_bytes_sent: 512 * 1024,
            load_average: [1.5, 1.2, 1.0],
            uptime: Duration::from_secs(3600),
        }
    }
    
    /// Perform performance analytics
    async fn perform_analytics(metrics: &Arc<RwLock<VecDeque<Metric>>>) -> PerformanceAnalytics {
        let metrics_guard = metrics.read().await;
        
        // Calculate latency percentiles
        let mut latencies: Vec<u64> = metrics_guard
            .iter()
            .filter(|m| m.name.contains("latency"))
            .filter_map(|m| match &m.value {
                MetricValue::Gauge(v) => Some(*v as u64),
                _ => None,
            })
            .collect();
        
        latencies.sort();
        
        let latency_percentiles = if !latencies.is_empty() {
            LatencyPercentiles {
                p50: latencies[latencies.len() * 50 / 100],
                p90: latencies[latencies.len() * 90 / 100],
                p95: latencies[latencies.len() * 95 / 100],
                p99: latencies[latencies.len() * 99 / 100],
                p999: latencies[latencies.len() * 999 / 1000],
            }
        } else {
            LatencyPercentiles {
                p50: 0,
                p90: 0,
                p95: 0,
                p99: 0,
                p999: 0,
            }
        };
        
        // Calculate throughput trends
        let throughput_trends = Vec::new(); // Would be calculated from historical data
        
        // Calculate resource utilization
        let resource_utilization = ResourceUtilization {
            cpu_utilization: vec![45.0, 50.0, 55.0],
            memory_utilization: vec![60.0, 65.0, 70.0],
            network_utilization: vec![30.0, 35.0, 40.0],
            disk_utilization: vec![20.0, 25.0, 30.0],
        };
        
        // Calculate performance predictions
        let performance_predictions = PerformancePredictions {
            predicted_cpu_usage: 55.0,
            predicted_memory_usage: 75.0,
            predicted_throughput: 1500.0,
            predicted_latency: 2.5,
            confidence_score: 0.85,
        };
        
        PerformanceAnalytics {
            latency_percentiles,
            throughput_trends,
            resource_utilization,
            performance_predictions,
        }
    }
    
    /// Check for alerts
    async fn check_alerts(
        config: &MonitoringConfig,
        system_metrics: &Arc<RwLock<SystemMetrics>>,
        broker_metrics: &Arc<RwLock<BrokerMetrics>>,
        alerts: &Arc<RwLock<HashMap<String, Alert>>>,
        alert_sender: &broadcast::Sender<Alert>,
    ) {
        let system_metrics_guard = system_metrics.read().await;
        let broker_metrics_guard = broker_metrics.read().await;
        
        // Check CPU threshold
        if system_metrics_guard.cpu_usage > config.alert_thresholds.cpu_threshold {
            let alert = Alert {
                id: "high_cpu_usage".to_string(),
                name: "High CPU Usage".to_string(),
                severity: AlertSeverity::Warning,
                message: format!("CPU usage is {}%", system_metrics_guard.cpu_usage),
                timestamp: SystemTime::now(),
                source: "monitoring".to_string(),
                labels: HashMap::new(),
                status: AlertStatus::Firing,
            };
            
            let _ = alert_sender.send(alert.clone());
            
            let mut alerts_guard = alerts.write().await;
            alerts_guard.insert(alert.id.clone(), alert);
        }
        
        // Check memory threshold
        if system_metrics_guard.memory_usage > config.alert_thresholds.memory_threshold {
            let alert = Alert {
                id: "high_memory_usage".to_string(),
                name: "High Memory Usage".to_string(),
                severity: AlertSeverity::Warning,
                message: format!("Memory usage is {}%", system_metrics_guard.memory_usage),
                timestamp: SystemTime::now(),
                source: "monitoring".to_string(),
                labels: HashMap::new(),
                status: AlertStatus::Firing,
            };
            
            let _ = alert_sender.send(alert.clone());
            
            let mut alerts_guard = alerts.write().await;
            alerts_guard.insert(alert.id.clone(), alert);
        }
        
        // Check error rate threshold
        if broker_metrics_guard.error_rate > config.alert_thresholds.error_rate_threshold {
            let alert = Alert {
                id: "high_error_rate".to_string(),
                name: "High Error Rate".to_string(),
                severity: AlertSeverity::Critical,
                message: format!("Error rate is {}%", broker_metrics_guard.error_rate),
                timestamp: SystemTime::now(),
                source: "monitoring".to_string(),
                labels: HashMap::new(),
                status: AlertStatus::Firing,
            };
            
            let _ = alert_sender.send(alert.clone());
            
            let mut alerts_guard = alerts.write().await;
            alerts_guard.insert(alert.id.clone(), alert);
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_bytes_received: 0,
            network_bytes_sent: 0,
            load_average: [0.0, 0.0, 0.0],
            uptime: Duration::from_secs(0),
        }
    }
}

impl Default for BrokerMetrics {
    fn default() -> Self {
        Self {
            total_messages_produced: 0,
            total_messages_consumed: 0,
            total_bytes_produced: 0,
            total_bytes_consumed: 0,
            active_producers: 0,
            active_consumers: 0,
            active_topics: 0,
            active_queues: 0,
            consumer_groups: 0,
            avg_message_latency_us: 0,
            avg_throughput: 0.0,
            error_rate: 0.0,
        }
    }
}

impl Default for PerformanceAnalytics {
    fn default() -> Self {
        Self {
            latency_percentiles: LatencyPercentiles {
                p50: 0,
                p90: 0,
                p95: 0,
                p99: 0,
                p999: 0,
            },
            throughput_trends: Vec::new(),
            resource_utilization: ResourceUtilization {
                cpu_utilization: Vec::new(),
                memory_utilization: Vec::new(),
                network_utilization: Vec::new(),
                disk_utilization: Vec::new(),
            },
            performance_predictions: PerformancePredictions {
                predicted_cpu_usage: 0.0,
                predicted_memory_usage: 0.0,
                predicted_throughput: 0.0,
                predicted_latency: 0.0,
                confidence_score: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_monitoring);
        assert!(config.enable_realtime);
        assert!(config.enable_analytics);
        assert!(config.enable_alerting);
        assert_eq!(config.collection_interval, Duration::from_secs(1));
    }
    
    #[test]
    fn test_alert_thresholds_default() {
        let thresholds = AlertThresholds::default();
        assert_eq!(thresholds.cpu_threshold, 80.0);
        assert_eq!(thresholds.memory_threshold, 85.0);
        assert_eq!(thresholds.disk_threshold, 90.0);
        assert_eq!(thresholds.latency_threshold, 100.0);
        assert_eq!(thresholds.error_rate_threshold, 5.0);
        assert_eq!(thresholds.throughput_threshold, 1000.0);
    }
    
    #[test]
    fn test_metric_creation() {
        let metric = Metric {
            name: "test_metric".to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Gauge(42.0),
            labels: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        
        assert_eq!(metric.name, "test_metric");
        assert_eq!(metric.metric_type, MetricType::Gauge);
    }
    
    #[test]
    fn test_alert_severity() {
        assert_eq!(AlertSeverity::Info, AlertSeverity::Info);
        assert_ne!(AlertSeverity::Info, AlertSeverity::Warning);
        assert_ne!(AlertSeverity::Warning, AlertSeverity::Critical);
        assert_ne!(AlertSeverity::Critical, AlertSeverity::Emergency);
    }
    
    #[test]
    fn test_alert_status() {
        assert_eq!(AlertStatus::Firing, AlertStatus::Firing);
        assert_ne!(AlertStatus::Firing, AlertStatus::Resolved);
        assert_ne!(AlertStatus::Resolved, AlertStatus::Acknowledged);
    }
}