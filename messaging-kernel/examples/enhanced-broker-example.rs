//! Enhanced Precursor Broker Example
//! 
//! This example demonstrates the advanced Kafka-level features of the enhanced
//! Precursor broker including zero-copy memory management, high-performance
//! storage, compression, networking, consumer groups, monitoring, and security.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use tokio::time::sleep;
use tracing::{info, warn};

use precursor_broker::{
    EnhancedPrecursorBroker, EnhancedBrokerConfig, PerformanceLevel,
    SecurityConfig, AuthenticationMethod, AuthorizationModel, TlsVersion,
    EncryptionAlgorithm, MonitoringConfig, AlertThresholds,
    NetworkConfig, CompressionConfig, CompressionAlgorithm,
    MemoryPoolConfig, SegmentStorageConfig,
    Message, MessageMetadata, CompressionType,
    Credentials, AuthenticationResult, Action,
};

/// Example message handler
struct ExampleMessageHandler;

#[async_trait::async_trait]
impl precursor_broker::MessageHandler for ExampleMessageHandler {
    async fn handle_message(
        &self,
        connection: Arc<precursor_broker::NetworkConnection>,
        message: Message,
    ) -> precursor_broker::Result<()> {
        info!(
            "Received message from connection {}: {} bytes",
            connection.id,
            message.size()
        );
        Ok(())
    }
    
    async fn handle_connection_established(
        &self,
        connection: Arc<precursor_broker::NetworkConnection>,
    ) -> precursor_broker::Result<()> {
        info!("New connection established: {}", connection.id);
        Ok(())
    }
    
    async fn handle_connection_closed(
        &self,
        connection: Arc<precursor_broker::NetworkConnection>,
    ) -> precursor_broker::Result<()> {
        info!("Connection closed: {}", connection.id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting Enhanced Precursor Broker Example");
    
    // Create enhanced broker configuration
    let config = create_enhanced_config();
    
    // Create and start the enhanced broker
    let mut broker = EnhancedPrecursorBroker::new(config);
    broker.start().await?;
    
    info!("Enhanced broker started successfully");
    
    // Demonstrate message production with compression
    demonstrate_message_production(&broker).await?;
    
    // Demonstrate message consumption
    demonstrate_message_consumption(&broker).await?;
    
    // Demonstrate security features
    demonstrate_security_features(&broker).await?;
    
    // Demonstrate monitoring capabilities
    demonstrate_monitoring(&broker).await?;
    
    // Demonstrate performance metrics
    demonstrate_performance_metrics(&broker).await?;
    
    // Run for a while to show continuous operation
    info!("Running broker for 30 seconds to demonstrate continuous operation...");
    sleep(Duration::from_secs(30)).await;
    
    // Stop the broker
    broker.stop().await?;
    
    info!("Enhanced broker example completed successfully");
    Ok(())
}

/// Create enhanced broker configuration
fn create_enhanced_config() -> EnhancedBrokerConfig {
    // Memory pool configuration for high performance
    let memory_pool_config = MemoryPoolConfig {
        initial_size: 128 * 1024 * 1024, // 128MB
        max_size: 2048 * 1024 * 1024,    // 2GB
        chunk_size: 4096,                // 4KB
        numa_aware: true,
        simd_enabled: true,
        alignment: 64,                   // Cache line alignment
        prewarm: true,
        gc_threshold: 0.8,               // 80% utilization threshold
    };
    
    // Segment storage configuration
    let segment_storage_config = SegmentStorageConfig {
        data_dir: std::path::PathBuf::from("./enhanced_data"),
        segment_size: 1024 * 1024 * 1024, // 1GB
        index_interval: 4096,              // 4KB
        enable_mmap: true,
        enable_compression: true,
        compression_algorithm: precursor_broker::CompressionAlgorithm::Lz4,
        flush_interval: Duration::from_millis(100),
        sync_on_write: false,
        enable_zero_copy: true,
        memory_pool_config: memory_pool_config.clone(),
    };
    
    // Compression configuration
    let compression_config = CompressionConfig {
        default_algorithm: CompressionAlgorithm::Lz4,
        enable_hardware_acceleration: true,
        compression_level: 3,
        enable_adaptive: true,
        compression_threshold: 1024, // 1KB
        enable_dictionary: true,
        dictionary_size: 64 * 1024, // 64KB
        enable_parallel: true,
        compression_threads: num_cpus::get(),
    };
    
    // Network configuration
    let network_config = NetworkConfig {
        listen_addr: "0.0.0.0:9092".parse().unwrap(),
        enable_kernel_bypass: false, // Would be true in production
        enable_rdma: false,          // Would be true in production
        enable_dpdk: false,          // Would be true in production
        enable_sriov: false,         // Would be true in production
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
    };
    
    // Security configuration
    let security_config = SecurityConfig {
        enable_tls: true,
        tls_version: TlsVersion::Tls13,
        cert_file: Some("./certs/server.crt".to_string()),
        key_file: Some("./certs/server.key".to_string()),
        ca_cert_file: Some("./certs/ca.crt".to_string()),
        enable_mtls: true,
        verify_client_cert: true,
        enable_authentication: true,
        auth_method: AuthenticationMethod::Jwt,
        enable_authorization: true,
        authz_model: AuthorizationModel::Rbac,
        enable_audit_logging: true,
        audit_log_level: precursor_broker::AuditLogLevel::Info,
        enable_encryption_at_rest: true,
        encryption_algorithm: EncryptionAlgorithm::Aes256Gcm,
        key_rotation_interval: Duration::from_secs(86400), // 24 hours
        session_timeout: Duration::from_secs(3600),        // 1 hour
        max_failed_attempts: 5,
        lockout_duration: Duration::from_secs(900),        // 15 minutes
    };
    
    // Monitoring configuration
    let monitoring_config = MonitoringConfig {
        enable_monitoring: true,
        collection_interval: Duration::from_secs(1),
        retention_period: Duration::from_secs(3600), // 1 hour
        enable_realtime: true,
        enable_analytics: true,
        enable_alerting: true,
        alert_thresholds: AlertThresholds {
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            disk_threshold: 90.0,
            latency_threshold: 100.0,
            error_rate_threshold: 5.0,
            throughput_threshold: 1000.0,
        },
        export_config: precursor_broker::MetricsExportConfig {
            enable_prometheus: true,
            prometheus_endpoint: "0.0.0.0:9090".to_string(),
            enable_influxdb: false,
            influxdb_config: None,
            enable_graphite: false,
            graphite_config: None,
        },
        dashboard_config: precursor_broker::DashboardConfig {
            enable_web_dashboard: true,
            dashboard_port: 3000,
            dashboard_host: "0.0.0.0".to_string(),
            enable_realtime_updates: true,
            update_interval: Duration::from_millis(100),
        },
    };
    
    EnhancedBrokerConfig {
        broker: precursor_broker::BrokerConfig {
            broker_id: "enhanced-broker-1".to_string(),
            data_dir: "./enhanced_data".to_string(),
            max_message_size: 10 * 1024 * 1024, // 10MB
            default_retention: Duration::from_secs(7 * 24 * 3600), // 7 days
            enable_persistence: true,
            enable_replication: true,
            replication_factor: 3,
            sync_replication: true,
            enable_compression: true,
            compression_level: 6,
            flush_interval: Duration::from_millis(100),
            segment_size: 1024 * 1024, // 1MB
            index_interval: 4096,      // 4KB
            cleanup_interval: Duration::from_secs(3600), // 1 hour
            cluster: precursor_broker::ClusterConfig::default(),
        },
        memory_pool: memory_pool_config,
        segment_storage: segment_storage_config,
        compression: compression_config,
        network: network_config,
        consumer_group: precursor_broker::ConsumerGroupConfig::default(),
        monitoring: monitoring_config,
        enable_optimizations: true,
        performance_level: PerformanceLevel::High,
    }
}

/// Demonstrate message production with advanced features
async fn demonstrate_message_production(
    broker: &EnhancedPrecursorBroker,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating message production with compression...");
    
    // Create test messages of different sizes
    let small_message = create_test_message("small", 100);
    let medium_message = create_test_message("medium", 5000);
    let large_message = create_test_message("large", 50000);
    
    // Produce messages to different topics
    let topics = ["test-topic-1", "test-topic-2", "test-topic-3"];
    let messages = [small_message, medium_message, large_message];
    
    for (i, (topic, message)) in topics.iter().zip(messages.iter()).enumerate() {
        let offset = broker.produce_message(topic, i as i32, message.clone()).await?;
        info!(
            "Produced {} message to topic {} at offset {}",
            if message.size() < 1000 { "small" } else if message.size() < 10000 { "medium" } else { "large" },
            topic,
            offset.value()
        );
    }
    
    // Demonstrate batch production
    info!("Demonstrating batch message production...");
    for i in 0..100 {
        let message = create_test_message(&format!("batch-{}", i), 1000);
        let offset = broker.produce_message("batch-topic", 0, message).await?;
        
        if i % 10 == 0 {
            info!("Produced batch message {} at offset {}", i, offset.value());
        }
    }
    
    Ok(())
}

/// Demonstrate message consumption
async fn demonstrate_message_consumption(
    broker: &EnhancedPrecursorBroker,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating message consumption...");
    
    // Consume messages from different topics
    let topics = ["test-topic-1", "test-topic-2", "test-topic-3"];
    
    for (i, topic) in topics.iter().enumerate() {
        if let Some(message) = broker.consume_message(topic, i as i32, "test-group", "consumer-1").await? {
            info!(
                "Consumed message from topic {}: {} bytes",
                topic,
                message.size()
            );
        } else {
            warn!("No message available from topic {}", topic);
        }
    }
    
    // Demonstrate batch consumption
    info!("Demonstrating batch message consumption...");
    let mut consumed_count = 0;
    for i in 0..100 {
        if let Some(message) = broker.consume_message("batch-topic", 0, "batch-group", "batch-consumer").await? {
            consumed_count += 1;
            if i % 10 == 0 {
                info!("Consumed batch message {}: {} bytes", i, message.size());
            }
        }
    }
    
    info!("Total batch messages consumed: {}", consumed_count);
    Ok(())
}

/// Demonstrate security features
async fn demonstrate_security_features(
    broker: &EnhancedPrecursorBroker,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating security features...");
    
    // Note: In a real implementation, the security manager would be accessible
    // through the broker. For this example, we'll show the concepts.
    
    info!("Security features demonstrated:");
    info!("- TLS 1.3 encryption in transit");
    info!("- mTLS mutual authentication");
    info!("- JWT-based authentication");
    info!("- RBAC authorization");
    info!("- AES-256-GCM encryption at rest");
    info!("- Comprehensive audit logging");
    info!("- Session management");
    info!("- Account lockout protection");
    
    Ok(())
}

/// Demonstrate monitoring capabilities
async fn demonstrate_monitoring(
    broker: &EnhancedPrecursorBroker,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating monitoring capabilities...");
    
    // Get enhanced broker statistics
    let stats = broker.get_enhanced_stats().await;
    
    info!("Broker Statistics:");
    info!("- Total messages produced: {}", stats.broker.total_messages_produced);
    info!("- Total messages consumed: {}", stats.broker.total_messages_consumed);
    info!("- Total bytes produced: {}", stats.broker.total_bytes_produced);
    info!("- Total bytes consumed: {}", stats.broker.total_bytes_consumed);
    info!("- Active producers: {}", stats.broker.active_producers);
    info!("- Active consumers: {}", stats.broker.active_consumers);
    info!("- Active topics: {}", stats.broker.active_topics);
    info!("- Active queues: {}", stats.broker.active_queues);
    info!("- Consumer groups: {}", stats.broker.consumer_groups);
    
    info!("Performance Metrics:");
    info!("- Messages per second: {:.2}", stats.performance.messages_per_second);
    info!("- Bytes per second: {:.2}", stats.performance.bytes_per_second);
    info!("- Average latency: {}μs", stats.performance.avg_latency_us);
    info!("- P99 latency: {}μs", stats.performance.p99_latency_us);
    info!("- Memory usage: {}MB", stats.performance.memory_usage / 1024 / 1024);
    info!("- CPU usage: {:.1}%", stats.performance.cpu_usage);
    info!("- Network throughput: {:.2}MB/s", stats.performance.network_throughput / 1024.0 / 1024.0);
    info!("- Compression ratio: {:.2}x", stats.performance.compression_ratio);
    info!("- Cache hit ratio: {:.1}%", stats.performance.cache_hit_ratio * 100.0);
    
    info!("Memory Statistics:");
    info!("- Total allocations: {}", stats.memory.total_allocations);
    info!("- Total deallocations: {}", stats.memory.total_deallocations);
    info!("- Current allocated: {}MB", stats.memory.current_allocated / 1024 / 1024);
    info!("- Peak allocated: {}MB", stats.memory.peak_allocated / 1024 / 1024);
    info!("- Allocation failures: {}", stats.memory.allocation_failures);
    info!("- Cache hits: {}", stats.memory.cache_hits);
    info!("- Cache misses: {}", stats.memory.cache_misses);
    info!("- GC runs: {}", stats.memory.gc_runs);
    info!("- Fragmentation ratio: {:.2}", stats.memory.fragmentation_ratio);
    
    info!("Storage Statistics:");
    info!("- Total segments: {}", stats.storage.total_segments);
    info!("- Total size: {}MB", stats.storage.total_size / 1024 / 1024);
    info!("- Total messages: {}", stats.storage.total_messages);
    info!("- Data directory: {}", stats.storage.data_dir);
    
    info!("Compression Statistics:");
    info!("- Total bytes compressed: {}MB", stats.compression.total_bytes_compressed / 1024 / 1024);
    info!("- Total bytes decompressed: {}MB", stats.compression.total_bytes_decompressed / 1024 / 1024);
    info!("- Total compression time: {}μs", stats.compression.total_compression_time_us);
    info!("- Total decompression time: {}μs", stats.compression.total_decompression_time_us);
    info!("- Average compression ratio: {:.2}", stats.compression.avg_compression_ratio);
    
    info!("Network Statistics:");
    info!("- Total connections: {}", stats.network.total_connections);
    info!("- Active connections: {}", stats.network.active_connections);
    info!("- Total bytes received: {}MB", stats.network.total_bytes_received / 1024 / 1024);
    info!("- Total bytes sent: {}MB", stats.network.total_bytes_sent / 1024 / 1024);
    info!("- Total messages received: {}", stats.network.total_messages_received);
    info!("- Total messages sent: {}", stats.network.total_messages_sent);
    info!("- Server uptime: {:?}", stats.network.uptime);
    info!("- Average connection duration: {:?}", stats.network.avg_connection_duration);
    info!("- Peak connections: {}", stats.network.peak_connections);
    
    info!("System Metrics:");
    info!("- CPU usage: {:.1}%", stats.system.cpu_usage);
    info!("- Memory usage: {:.1}%", stats.system.memory_usage);
    info!("- Disk usage: {:.1}%", stats.system.disk_usage);
    info!("- Network bytes received: {}MB", stats.system.network_bytes_received / 1024 / 1024);
    info!("- Network bytes sent: {}MB", stats.system.network_bytes_sent / 1024 / 1024);
    info!("- Load average: [{:.2}, {:.2}, {:.2}]", 
          stats.system.load_average[0], 
          stats.system.load_average[1], 
          stats.system.load_average[2]);
    info!("- Uptime: {:?}", stats.system.uptime);
    
    Ok(())
}

/// Demonstrate performance metrics
async fn demonstrate_performance_metrics(
    broker: &EnhancedPrecursorBroker,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demonstrating performance metrics...");
    
    // Simulate some load to generate metrics
    info!("Generating load to demonstrate performance metrics...");
    
    let start_time = std::time::Instant::now();
    let mut message_count = 0;
    
    // Produce messages for 5 seconds
    while start_time.elapsed() < Duration::from_secs(5) {
        let message = create_test_message(&format!("perf-{}", message_count), 1000);
        let _ = broker.produce_message("performance-topic", 0, message).await?;
        message_count += 1;
        
        if message_count % 100 == 0 {
            let elapsed = start_time.elapsed();
            let throughput = message_count as f64 / elapsed.as_secs_f64();
            info!("Produced {} messages in {:?} ({:.2} msg/s)", message_count, elapsed, throughput);
        }
    }
    
    let total_time = start_time.elapsed();
    let final_throughput = message_count as f64 / total_time.as_secs_f64();
    
    info!("Performance test completed:");
    info!("- Total messages: {}", message_count);
    info!("- Total time: {:?}", total_time);
    info!("- Average throughput: {:.2} messages/second", final_throughput);
    
    // Get final statistics
    let stats = broker.get_enhanced_stats().await;
    info!("Final performance metrics:");
    info!("- Messages per second: {:.2}", stats.performance.messages_per_second);
    info!("- Average latency: {}μs", stats.performance.avg_latency_us);
    info!("- P99 latency: {}μs", stats.performance.p99_latency_us);
    info!("- Memory usage: {}MB", stats.performance.memory_usage / 1024 / 1024);
    info!("- CPU usage: {:.1}%", stats.performance.cpu_usage);
    
    Ok(())
}

/// Create a test message
fn create_test_message(content: &str, size: usize) -> Message {
    let payload = format!("{}-{}", content, "x".repeat(size.saturating_sub(content.len() + 1)));
    let payload_bytes = Bytes::from(payload);
    let headers = HashMap::new();
    
    Message {
        metadata: MessageMetadata {
            message_id: 0, // Will be set by broker
            timestamp: SystemTime::now(),
            size: payload_bytes.len(),
            compression: Some(CompressionType::None),
            checksum: 0, // Will be calculated by broker
            headers,
            partition_key: None,
        },
        payload: payload_bytes,
    }
}