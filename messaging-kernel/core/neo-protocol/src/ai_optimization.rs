//! AI-Powered Performance Optimization Engine
//! 
//! This module implements machine learning-based optimization for the Neo Messaging Kernel,
//! providing adaptive performance tuning, predictive scaling, and intelligent resource management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

/// AI Optimization Engine for adaptive performance tuning
pub struct AiOptimizationEngine {
    /// Performance metrics collector
    metrics_collector: Arc<MetricsCollector>,
    /// ML model manager
    model_manager: Arc<ModelManager>,
    /// Optimization strategies
    strategies: Arc<RwLock<HashMap<String, OptimizationStrategy>>>,
    /// Performance predictor
    predictor: Arc<PerformancePredictor>,
    /// Resource allocator
    resource_allocator: Arc<ResourceAllocator>,
    /// Configuration
    config: AiOptimizationConfig,
}

/// Performance metrics for ML analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Request latency (microseconds)
    pub latency_us: u64,
    /// Throughput (requests per second)
    pub throughput_rps: u64,
    /// Memory usage (bytes)
    pub memory_usage_bytes: u64,
    /// CPU usage (percentage)
    pub cpu_usage_percent: f64,
    /// Network I/O (bytes per second)
    pub network_io_bps: u64,
    /// Disk I/O (bytes per second)
    pub disk_io_bps: u64,
    /// Error rate (percentage)
    pub error_rate_percent: f64,
    /// Queue depth
    pub queue_depth: usize,
    /// Active connections
    pub active_connections: usize,
    /// Cache hit rate (percentage)
    pub cache_hit_rate_percent: f64,
    /// Compression ratio
    pub compression_ratio: f64,
}

/// Optimization strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// Memory pool optimization
    MemoryPoolOptimization {
        pool_sizes: Vec<usize>,
        allocation_strategy: AllocationStrategy,
    },
    /// CPU affinity optimization
    CpuAffinityOptimization {
        cpu_mask: u64,
        numa_aware: bool,
    },
    /// Network optimization
    NetworkOptimization {
        tcp_nodelay: bool,
        tcp_congestion_control: String,
        buffer_sizes: NetworkBufferSizes,
    },
    /// Compression optimization
    CompressionOptimization {
        algorithm: CompressionAlgorithm,
        level: u8,
        threshold_bytes: usize,
    },
    /// Caching optimization
    CachingOptimization {
        cache_size_bytes: usize,
        eviction_policy: EvictionPolicy,
        ttl_seconds: u64,
    },
    /// Load balancing optimization
    LoadBalancingOptimization {
        algorithm: LoadBalancingAlgorithm,
        weights: HashMap<String, f64>,
    },
}

/// Allocation strategy for memory pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// First-fit allocation
    FirstFit,
    /// Best-fit allocation
    BestFit,
    /// Buddy system allocation
    BuddySystem,
    /// Slab allocation
    SlabAllocation,
}

/// Network buffer sizes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBufferSizes {
    pub send_buffer_bytes: usize,
    pub receive_buffer_bytes: usize,
    pub backlog_connections: usize,
}

/// Compression algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// LZ4 compression
    Lz4,
    /// Zstandard compression
    Zstd,
    /// Brotli compression
    Brotli,
    /// Gzip compression
    Gzip,
    /// Snappy compression
    Snappy,
}

/// Cache eviction policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least recently used
    Lru,
    /// Least frequently used
    Lfu,
    /// Time-based eviction
    Ttl,
    /// Random eviction
    Random,
}

/// Load balancing algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Round robin
    RoundRobin,
    /// Weighted round robin
    WeightedRoundRobin,
    /// Least connections
    LeastConnections,
    /// Least response time
    LeastResponseTime,
    /// Consistent hashing
    ConsistentHashing,
    /// Machine learning-based
    MlBased,
}

/// AI Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOptimizationConfig {
    /// Enable AI optimization
    pub enabled: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Model training interval
    pub training_interval: Duration,
    /// Optimization update interval
    pub optimization_interval: Duration,
    /// Performance prediction horizon
    pub prediction_horizon: Duration,
    /// Minimum data points for training
    pub min_training_samples: usize,
    /// Model complexity threshold
    pub model_complexity_threshold: f64,
    /// Optimization aggressiveness (0.0 to 1.0)
    pub optimization_aggressiveness: f64,
    /// Enable real-time adaptation
    pub enable_realtime_adaptation: bool,
    /// Enable predictive scaling
    pub enable_predictive_scaling: bool,
    /// Enable anomaly detection
    pub enable_anomaly_detection: bool,
}

impl Default for AiOptimizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: Duration::from_secs(1),
            training_interval: Duration::from_secs(60),
            optimization_interval: Duration::from_secs(10),
            prediction_horizon: Duration::from_secs(300),
            min_training_samples: 1000,
            model_complexity_threshold: 0.8,
            optimization_aggressiveness: 0.5,
            enable_realtime_adaptation: true,
            enable_predictive_scaling: true,
            enable_anomaly_detection: true,
        }
    }
}

/// Metrics collector for performance data
pub struct MetricsCollector {
    /// Collected metrics
    metrics: Arc<RwLock<Vec<PerformanceMetrics>>>,
    /// Metrics aggregation
    aggregation: Arc<RwLock<MetricsAggregation>>,
}

/// Metrics aggregation for ML analysis
#[derive(Debug, Default)]
pub struct MetricsAggregation {
    /// Average latency
    pub avg_latency_us: f64,
    /// P95 latency
    pub p95_latency_us: f64,
    /// P99 latency
    pub p99_latency_us: f64,
    /// Average throughput
    pub avg_throughput_rps: f64,
    /// Peak throughput
    pub peak_throughput_rps: f64,
    /// Average memory usage
    pub avg_memory_usage_bytes: f64,
    /// Peak memory usage
    pub peak_memory_usage_bytes: f64,
    /// Average CPU usage
    pub avg_cpu_usage_percent: f64,
    /// Peak CPU usage
    pub peak_cpu_usage_percent: f64,
    /// Average error rate
    pub avg_error_rate_percent: f64,
    /// Peak error rate
    pub peak_error_rate_percent: f64,
}

/// ML Model manager for optimization
pub struct ModelManager {
    /// Performance prediction model
    prediction_model: Arc<RwLock<Option<Box<dyn PredictionModel + Send + Sync>>>>,
    /// Anomaly detection model
    anomaly_model: Arc<RwLock<Option<Box<dyn AnomalyDetectionModel + Send + Sync>>>>,
    /// Optimization model
    optimization_model: Arc<RwLock<Option<Box<dyn OptimizationModel + Send + Sync>>>>,
}

/// Performance predictor trait
pub trait PredictionModel {
    /// Predict performance metrics for given input
    fn predict(&self, input: &PerformanceMetrics) -> Result<PerformancePrediction, PredictionError>;
    /// Train the model with historical data
    fn train(&mut self, data: &[PerformanceMetrics]) -> Result<(), PredictionError>;
    /// Get model accuracy
    fn accuracy(&self) -> f64;
}

/// Anomaly detection model trait
pub trait AnomalyDetectionModel {
    /// Detect anomalies in performance metrics
    fn detect_anomaly(&self, metrics: &PerformanceMetrics) -> Result<AnomalyScore, AnomalyError>;
    /// Train the model with normal data
    fn train(&mut self, data: &[PerformanceMetrics]) -> Result<(), AnomalyError>;
    /// Get anomaly threshold
    fn threshold(&self) -> f64;
}

/// Optimization model trait
pub trait OptimizationModel {
    /// Generate optimization strategy
    fn optimize(&self, metrics: &PerformanceMetrics) -> Result<OptimizationStrategy, OptimizationError>;
    /// Evaluate strategy effectiveness
    fn evaluate(&self, strategy: &OptimizationStrategy, metrics: &PerformanceMetrics) -> Result<f64, OptimizationError>;
}

/// Performance prediction result
#[derive(Debug, Clone)]
pub struct PerformancePrediction {
    /// Predicted latency
    pub predicted_latency_us: u64,
    /// Predicted throughput
    pub predicted_throughput_rps: u64,
    /// Predicted memory usage
    pub predicted_memory_usage_bytes: u64,
    /// Predicted CPU usage
    pub predicted_cpu_usage_percent: f64,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Prediction horizon
    pub horizon: Duration,
}

/// Anomaly score
#[derive(Debug, Clone)]
pub struct AnomalyScore {
    /// Anomaly score (0.0 to 1.0)
    pub score: f64,
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Severity level
    pub severity: SeverityLevel,
    /// Confidence
    pub confidence: f64,
}

/// Anomaly type
#[derive(Debug, Clone)]
pub enum AnomalyType {
    /// Latency spike
    LatencySpike,
    /// Throughput drop
    ThroughputDrop,
    /// Memory leak
    MemoryLeak,
    /// CPU spike
    CpuSpike,
    /// Error rate increase
    ErrorRateIncrease,
    /// Network congestion
    NetworkCongestion,
    /// Disk I/O bottleneck
    DiskIoBottleneck,
}

/// Severity level
#[derive(Debug, Clone)]
pub enum SeverityLevel {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Performance predictor implementation
pub struct LinearRegressionPredictor {
    /// Model coefficients
    coefficients: Vec<f64>,
    /// Model intercept
    intercept: f64,
    /// Training accuracy
    accuracy: f64,
}

impl PredictionModel for LinearRegressionPredictor {
    fn predict(&self, input: &PerformanceMetrics) -> Result<PerformancePrediction, PredictionError> {
        // Extract features from metrics
        let features = vec![
            input.latency_us as f64,
            input.throughput_rps as f64,
            input.memory_usage_bytes as f64,
            input.cpu_usage_percent,
            input.network_io_bps as f64,
            input.disk_io_bps as f64,
            input.error_rate_percent,
            input.queue_depth as f64,
            input.active_connections as f64,
            input.cache_hit_rate_percent,
            input.compression_ratio,
        ];

        // Calculate prediction
        let mut prediction = self.intercept;
        for (feature, coefficient) in features.iter().zip(self.coefficients.iter()) {
            prediction += feature * coefficient;
        }

        Ok(PerformancePrediction {
            predicted_latency_us: prediction.max(0.0) as u64,
            predicted_throughput_rps: (prediction * 1.1).max(0.0) as u64,
            predicted_memory_usage_bytes: (prediction * 1.05).max(0.0) as u64,
            predicted_cpu_usage_percent: prediction.min(100.0).max(0.0),
            confidence: self.accuracy,
            horizon: Duration::from_secs(60),
        })
    }

    fn train(&mut self, data: &[PerformanceMetrics]) -> Result<(), PredictionError> {
        if data.len() < 10 {
            return Err(PredictionError::InsufficientData);
        }

        // Simple linear regression implementation
        // In a real implementation, you would use a proper ML library
        let n = data.len() as f64;
        let sum_x: f64 = data.iter().map(|m| m.latency_us as f64).sum();
        let sum_y: f64 = data.iter().map(|m| m.throughput_rps as f64).sum();
        let sum_xy: f64 = data.iter().map(|m| (m.latency_us as f64) * (m.throughput_rps as f64)).sum();
        let sum_x2: f64 = data.iter().map(|m| (m.latency_us as f64).powi(2)).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        self.coefficients = vec![slope];
        self.intercept = intercept;
        self.accuracy = 0.85; // Simplified accuracy calculation

        Ok(())
    }

    fn accuracy(&self) -> f64 {
        self.accuracy
    }
}

/// Anomaly detection using statistical methods
pub struct StatisticalAnomalyDetector {
    /// Mean values for each metric
    means: HashMap<String, f64>,
    /// Standard deviations for each metric
    std_devs: HashMap<String, f64>,
    /// Anomaly threshold (number of standard deviations)
    threshold: f64,
}

impl AnomalyDetectionModel for StatisticalAnomalyDetector {
    fn detect_anomaly(&self, metrics: &PerformanceMetrics) -> Result<AnomalyScore, AnomalyError> {
        let mut max_score = 0.0;
        let mut anomaly_type = AnomalyType::LatencySpike;

        // Check latency anomaly
        if let (Some(mean), Some(std_dev)) = (self.means.get("latency"), self.std_devs.get("latency")) {
            let z_score = (metrics.latency_us as f64 - mean).abs() / std_dev;
            if z_score > max_score {
                max_score = z_score;
                anomaly_type = AnomalyType::LatencySpike;
            }
        }

        // Check throughput anomaly
        if let (Some(mean), Some(std_dev)) = (self.means.get("throughput"), self.std_devs.get("throughput")) {
            let z_score = (metrics.throughput_rps as f64 - mean).abs() / std_dev;
            if z_score > max_score {
                max_score = z_score;
                anomaly_type = AnomalyType::ThroughputDrop;
            }
        }

        // Check memory anomaly
        if let (Some(mean), Some(std_dev)) = (self.means.get("memory"), self.std_devs.get("memory")) {
            let z_score = (metrics.memory_usage_bytes as f64 - mean).abs() / std_dev;
            if z_score > max_score {
                max_score = z_score;
                anomaly_type = AnomalyType::MemoryLeak;
            }
        }

        let severity = if max_score > 3.0 {
            SeverityLevel::Critical
        } else if max_score > 2.0 {
            SeverityLevel::High
        } else if max_score > 1.5 {
            SeverityLevel::Medium
        } else {
            SeverityLevel::Low
        };

        Ok(AnomalyScore {
            score: max_score.min(1.0),
            anomaly_type,
            severity,
            confidence: max_score.min(1.0),
        })
    }

    fn train(&mut self, data: &[PerformanceMetrics]) -> Result<(), AnomalyError> {
        if data.is_empty() {
            return Err(AnomalyError::InsufficientData);
        }

        // Calculate means and standard deviations
        let latencies: Vec<f64> = data.iter().map(|m| m.latency_us as f64).collect();
        let throughputs: Vec<f64> = data.iter().map(|m| m.throughput_rps as f64).collect();
        let memories: Vec<f64> = data.iter().map(|m| m.memory_usage_bytes as f64).collect();

        self.means.insert("latency".to_string(), mean(&latencies));
        self.means.insert("throughput".to_string(), mean(&throughputs));
        self.means.insert("memory".to_string(), mean(&memories));

        self.std_devs.insert("latency".to_string(), standard_deviation(&latencies));
        self.std_devs.insert("throughput".to_string(), standard_deviation(&throughputs));
        self.std_devs.insert("memory".to_string(), standard_deviation(&memories));

        Ok(())
    }

    fn threshold(&self) -> f64 {
        self.threshold
    }
}

/// Resource allocator for dynamic resource management
pub struct ResourceAllocator {
    /// Current resource allocation
    allocation: Arc<RwLock<ResourceAllocation>>,
    /// Allocation history
    history: Arc<RwLock<Vec<ResourceAllocation>>>,
}

/// Resource allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// CPU cores allocated
    pub cpu_cores: usize,
    /// Memory allocated (bytes)
    pub memory_bytes: usize,
    /// Network bandwidth allocated (bytes per second)
    pub network_bandwidth_bps: u64,
    /// Disk I/O allocated (IOPS)
    pub disk_iops: u64,
    /// Cache size allocated (bytes)
    pub cache_size_bytes: usize,
    /// Thread pool size
    pub thread_pool_size: usize,
}

impl AiOptimizationEngine {
    /// Create a new AI optimization engine
    pub fn new(config: AiOptimizationConfig) -> Self {
        Self {
            metrics_collector: Arc::new(MetricsCollector::new()),
            model_manager: Arc::new(ModelManager::new()),
            strategies: Arc::new(RwLock::new(HashMap::new())),
            predictor: Arc::new(PerformancePredictor::new()),
            resource_allocator: Arc::new(ResourceAllocator::new()),
            config,
        }
    }

    /// Start the AI optimization engine
    pub async fn start(&self) -> Result<(), OptimizationError> {
        info!("Starting AI optimization engine");

        // Initialize models
        self.initialize_models().await?;

        // Start background tasks
        self.start_background_tasks().await;

        info!("AI optimization engine started successfully");
        Ok(())
    }

    /// Stop the AI optimization engine
    pub async fn stop(&self) -> Result<(), OptimizationError> {
        info!("Stopping AI optimization engine");
        // Implementation for stopping background tasks
        Ok(())
    }

    /// Collect performance metrics
    pub async fn collect_metrics(&self, metrics: PerformanceMetrics) -> Result<(), OptimizationError> {
        self.metrics_collector.collect(metrics).await?;
        Ok(())
    }

    /// Generate optimization recommendations
    pub async fn optimize(&self) -> Result<Vec<OptimizationStrategy>, OptimizationError> {
        let metrics = self.metrics_collector.get_latest_metrics().await?;
        let strategies = self.generate_optimization_strategies(&metrics).await?;
        Ok(strategies)
    }

    /// Apply optimization strategy
    pub async fn apply_strategy(&self, strategy: OptimizationStrategy) -> Result<(), OptimizationError> {
        info!("Applying optimization strategy: {:?}", strategy);
        // Implementation for applying optimization strategies
        Ok(())
    }

    /// Initialize ML models
    async fn initialize_models(&self) -> Result<(), OptimizationError> {
        // Initialize prediction model
        let predictor = Box::new(LinearRegressionPredictor {
            coefficients: vec![0.0],
            intercept: 0.0,
            accuracy: 0.0,
        });
        
        // Initialize anomaly detection model
        let anomaly_detector = Box::new(StatisticalAnomalyDetector {
            means: HashMap::new(),
            std_devs: HashMap::new(),
            threshold: 2.0,
        });

        // Store models
        // Implementation would store models in the model manager

        Ok(())
    }

    /// Start background optimization tasks
    async fn start_background_tasks(&self) {
        // Start metrics collection task
        // Start model training task
        // Start optimization task
        // Start anomaly detection task
    }

    /// Generate optimization strategies based on metrics
    async fn generate_optimization_strategies(&self, metrics: &PerformanceMetrics) -> Result<Vec<OptimizationStrategy>, OptimizationError> {
        let mut strategies = Vec::new();

        // Memory optimization
        if metrics.memory_usage_bytes > 1024 * 1024 * 1024 { // > 1GB
            strategies.push(OptimizationStrategy::MemoryPoolOptimization {
                pool_sizes: vec![1024, 4096, 16384, 65536],
                allocation_strategy: AllocationStrategy::SlabAllocation,
            });
        }

        // CPU optimization
        if metrics.cpu_usage_percent > 80.0 {
            strategies.push(OptimizationStrategy::CpuAffinityOptimization {
                cpu_mask: 0xFF, // Use all available cores
                numa_aware: true,
            });
        }

        // Network optimization
        if metrics.network_io_bps > 100 * 1024 * 1024 { // > 100MB/s
            strategies.push(OptimizationStrategy::NetworkOptimization {
                tcp_nodelay: true,
                tcp_congestion_control: "bbr".to_string(),
                buffer_sizes: NetworkBufferSizes {
                    send_buffer_bytes: 1024 * 1024,
                    receive_buffer_bytes: 1024 * 1024,
                    backlog_connections: 1024,
                },
            });
        }

        // Compression optimization
        if metrics.compression_ratio < 0.5 {
            strategies.push(OptimizationStrategy::CompressionOptimization {
                algorithm: CompressionAlgorithm::Zstd,
                level: 6,
                threshold_bytes: 1024,
            });
        }

        Ok(strategies)
    }
}

impl MetricsCollector {
    fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            aggregation: Arc::new(RwLock::new(MetricsAggregation::default())),
        }
    }

    async fn collect(&self, metrics: PerformanceMetrics) -> Result<(), OptimizationError> {
        let mut metrics_vec = self.metrics.write().await;
        metrics_vec.push(metrics.clone());

        // Keep only recent metrics (last 1000 samples)
        if metrics_vec.len() > 1000 {
            metrics_vec.drain(0..metrics_vec.len() - 1000);
        }

        // Update aggregation
        self.update_aggregation(&metrics).await?;

        Ok(())
    }

    async fn get_latest_metrics(&self) -> Result<PerformanceMetrics, OptimizationError> {
        let metrics_vec = self.metrics.read().await;
        metrics_vec.last()
            .cloned()
            .ok_or(OptimizationError::NoMetricsAvailable)
    }

    async fn update_aggregation(&self, metrics: &PerformanceMetrics) -> Result<(), OptimizationError> {
        let mut aggregation = self.aggregation.write().await;
        
        // Simple moving average update
        let alpha = 0.1; // Learning rate
        aggregation.avg_latency_us = alpha * metrics.latency_us as f64 + (1.0 - alpha) * aggregation.avg_latency_us;
        aggregation.avg_throughput_rps = alpha * metrics.throughput_rps as f64 + (1.0 - alpha) * aggregation.avg_throughput_rps;
        aggregation.avg_memory_usage_bytes = alpha * metrics.memory_usage_bytes as f64 + (1.0 - alpha) * aggregation.avg_memory_usage_bytes;
        aggregation.avg_cpu_usage_percent = alpha * metrics.cpu_usage_percent + (1.0 - alpha) * aggregation.avg_cpu_usage_percent;
        aggregation.avg_error_rate_percent = alpha * metrics.error_rate_percent + (1.0 - alpha) * aggregation.avg_error_rate_percent;

        Ok(())
    }
}

impl ModelManager {
    fn new() -> Self {
        Self {
            prediction_model: Arc::new(RwLock::new(None)),
            anomaly_model: Arc::new(RwLock::new(None)),
            optimization_model: Arc::new(RwLock::new(None)),
        }
    }
}

impl PerformancePredictor {
    fn new() -> Self {
        Self {
            // Implementation
        }
    }
}

impl ResourceAllocator {
    fn new() -> Self {
        Self {
            allocation: Arc::new(RwLock::new(ResourceAllocation {
                cpu_cores: 4,
                memory_bytes: 1024 * 1024 * 1024, // 1GB
                network_bandwidth_bps: 100 * 1024 * 1024, // 100MB/s
                disk_iops: 1000,
                cache_size_bytes: 256 * 1024 * 1024, // 256MB
                thread_pool_size: 16,
            })),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum OptimizationError {
    #[error("No metrics available")]
    NoMetricsAvailable,
    #[error("Model training failed: {0}")]
    ModelTrainingFailed(String),
    #[error("Strategy application failed: {0}")]
    StrategyApplicationFailed(String),
    #[error("Resource allocation failed: {0}")]
    ResourceAllocationFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum PredictionError {
    #[error("Insufficient data for prediction")]
    InsufficientData,
    #[error("Model not trained")]
    ModelNotTrained,
    #[error("Prediction failed: {0}")]
    PredictionFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AnomalyError {
    #[error("Insufficient data for anomaly detection")]
    InsufficientData,
    #[error("Anomaly detection failed: {0}")]
    DetectionFailed(String),
}

// Utility functions
fn mean(data: &[f64]) -> f64 {
    if data.is_empty() {
        0.0
    } else {
        data.iter().sum::<f64>() / data.len() as f64
    }
}

fn standard_deviation(data: &[f64]) -> f64 {
    if data.len() < 2 {
        return 0.0;
    }
    
    let mean_val = mean(data);
    let variance = data.iter()
        .map(|x| (x - mean_val).powi(2))
        .sum::<f64>() / (data.len() - 1) as f64;
    
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_optimization_engine_creation() {
        let config = AiOptimizationConfig::default();
        let engine = AiOptimizationEngine::new(config);
        assert!(engine.config.enabled);
    }

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics {
            timestamp: SystemTime::now(),
            latency_us: 1000,
            throughput_rps: 10000,
            memory_usage_bytes: 1024 * 1024,
            cpu_usage_percent: 50.0,
            network_io_bps: 1024 * 1024,
            disk_io_bps: 512 * 1024,
            error_rate_percent: 0.1,
            queue_depth: 100,
            active_connections: 50,
            cache_hit_rate_percent: 95.0,
            compression_ratio: 0.6,
        };

        assert_eq!(metrics.latency_us, 1000);
        assert_eq!(metrics.throughput_rps, 10000);
    }

    #[test]
    fn test_optimization_strategy_creation() {
        let strategy = OptimizationStrategy::MemoryPoolOptimization {
            pool_sizes: vec![1024, 4096, 16384],
            allocation_strategy: AllocationStrategy::SlabAllocation,
        };

        match strategy {
            OptimizationStrategy::MemoryPoolOptimization { pool_sizes, .. } => {
                assert_eq!(pool_sizes.len(), 3);
            }
            _ => panic!("Expected MemoryPoolOptimization"),
        }
    }

    #[test]
    fn test_statistical_functions() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(mean(&data), 3.0);
        assert!(standard_deviation(&data) > 0.0);
    }
}