//! Comprehensive benchmarking suite for Neo Protocol
//! 
//! This module provides detailed performance benchmarks for all aspects of the Neo Protocol,
//! including message creation, serialization, RPC calls, and system performance.

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
    measurement::WallTime, BatchSize, SamplingMode,
};
use neo_protocol::{
    NeoHeader, MessageType, NeoConfig, NeoContext, NeoStats,
    message::{RpcRequest, RpcResponse, EventMessage, HeartbeatMessage},
    monitoring::{MetricsCollector, MetricsConfig},
};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::runtime::Runtime;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};

/// Benchmark message creation performance
fn bench_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);
    group.sampling_mode(SamplingMode::Linear);

    // RPC Request creation
    group.bench_function("rpc_request", |b| {
        b.iter(|| {
            let request = RpcRequest {
                service_name: black_box("user_service".to_string()),
                method_name: black_box("get_user".to_string()),
                correlation_id: black_box(12345),
                payload: black_box(Bytes::from("test payload")),
                timeout: black_box(Some(Duration::from_secs(30))),
            };
            black_box(request)
        })
    });

    // RPC Response creation
    group.bench_function("rpc_response", |b| {
        b.iter(|| {
            let response = RpcResponse {
                correlation_id: black_box(12345),
                payload: black_box(Bytes::from("response payload")),
                success: black_box(true),
            };
            black_box(response)
        })
    });

    // Event message creation
    group.bench_function("event_message", |b| {
        b.iter(|| {
            let event = EventMessage {
                topic: black_box("user_events".to_string()),
                partition_key: black_box(Some("user_123".to_string())),
                payload: black_box(Bytes::from("event payload")),
                timestamp: black_box(Instant::now()),
            };
            black_box(event)
        })
    });

    // Heartbeat message creation
    group.bench_function("heartbeat", |b| {
        b.iter(|| {
            let heartbeat = HeartbeatMessage {
                timestamp: black_box(Instant::now()),
                client_id: black_box("client_123".to_string()),
            };
            black_box(heartbeat)
        })
    });

    group.finish();
}

/// Benchmark header creation and validation
fn bench_header_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Header creation
    group.bench_function("header_creation", |b| {
        b.iter(|| {
            let header = NeoHeader::new(
                black_box(MessageType::RpcRequest),
                black_box(12345),
                black_box(1024),
            );
            black_box(header)
        })
    });

    // Header validation
    group.bench_function("header_validation", |b| {
        let header = NeoHeader::new(MessageType::RpcRequest, 12345, 1024);
        b.iter(|| {
            let result = black_box(&header).validate();
            black_box(result)
        })
    });

    // Header serialization
    group.bench_function("header_serialization", |b| {
        let header = NeoHeader::new(MessageType::RpcRequest, 12345, 1024);
        b.iter(|| {
            let serialized = bincode::serialize(black_box(&header)).unwrap();
            black_box(serialized)
        })
    });

    // Header deserialization
    group.bench_function("header_deserialization", |b| {
        let header = NeoHeader::new(MessageType::RpcRequest, 12345, 1024);
        let serialized = bincode::serialize(&header).unwrap();
        b.iter(|| {
            let deserialized: NeoHeader = bincode::deserialize(black_box(&serialized)).unwrap();
            black_box(deserialized)
        })
    });

    group.finish();
}

/// Benchmark context operations
fn bench_context_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Context creation
    group.bench_function("context_creation", |b| {
        b.iter(|| {
            let context = NeoContext::new(
                black_box(12345),
                black_box("user_service".to_string()),
                black_box("get_user".to_string()),
            );
            black_box(context)
        })
    });

    // Context cloning
    group.bench_function("context_cloning", |b| {
        let context = NeoContext::new(12345, "user_service".to_string(), "get_user".to_string());
        b.iter(|| {
            let cloned = black_box(&context).clone();
            black_box(cloned)
        })
    });

    group.finish();
}

/// Benchmark statistics operations
fn bench_statistics_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistics_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Statistics recording
    group.bench_function("stats_recording", |b| {
        let mut stats = NeoStats::default();
        b.iter(|| {
            stats.record_request(black_box(1000));
            stats.record_response();
            stats.record_error();
            stats.record_event();
            black_box(&stats)
        })
    });

    // Statistics access
    group.bench_function("stats_access", |b| {
        let mut stats = NeoStats::default();
        stats.record_request(1000);
        stats.record_response();
        stats.record_error();
        stats.record_event();
        
        b.iter(|| {
            let total_requests = black_box(&stats).total_requests;
            let total_responses = black_box(&stats).total_responses;
            let total_errors = black_box(&stats).total_errors;
            let total_events = black_box(&stats).total_events;
            black_box((total_requests, total_responses, total_errors, total_events))
        })
    });

    group.finish();
}

/// Benchmark metrics collection
fn bench_metrics_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_collection");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Metrics collector creation
    group.bench_function("metrics_collector_creation", |b| {
        b.iter(|| {
            let config = MetricsConfig::default();
            let collector = MetricsCollector::new(black_box(config));
            black_box(collector)
        })
    });

    // Metrics recording
    group.bench_function("metrics_recording", |b| {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        b.iter(|| {
            collector.record_rpc_request();
            collector.record_rpc_response(black_box(Duration::from_millis(10)));
            collector.record_bytes_sent(black_box(1024));
            collector.record_bytes_received(black_box(2048));
            collector.record_method_invocation(black_box("get_user"));
        })
    });

    // Metrics snapshot
    group.bench_function("metrics_snapshot", |b| {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        // Pre-populate with some data
        for _ in 0..100 {
            collector.record_rpc_request();
            collector.record_rpc_response(Duration::from_millis(10));
            collector.record_bytes_sent(1024);
            collector.record_bytes_received(2048);
            collector.record_method_invocation("get_user");
        }
        
        b.iter(|| {
            let snapshot = collector.get_metrics_snapshot();
            black_box(snapshot)
        })
    });

    group.finish();
}

/// Benchmark async operations
fn bench_async_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("async_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    // Async context creation
    group.bench_function("async_context_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let context = NeoContext::new(
                black_box(12345),
                black_box("user_service".to_string()),
                black_box("get_user".to_string()),
            );
            black_box(context)
        })
    });

    // Async metrics collection
    group.bench_function("async_metrics_collection", |b| {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);
        
        b.to_async(&rt).iter(|| async {
            collector.record_rpc_request();
            collector.record_rpc_response(black_box(Duration::from_millis(10)));
            collector.record_bytes_sent(black_box(1024));
            collector.record_bytes_received(black_box(2048));
        })
    });

    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // String allocation
    group.bench_function("string_allocation", |b| {
        b.iter(|| {
            let s = black_box("user_service".to_string());
            black_box(s)
        })
    });

    // Bytes allocation
    group.bench_function("bytes_allocation", |b| {
        b.iter(|| {
            let bytes = black_box(Bytes::from("test payload"));
            black_box(bytes)
        })
    });

    // BytesMut allocation
    group.bench_function("bytes_mut_allocation", |b| {
        b.iter(|| {
            let mut bytes = black_box(BytesMut::with_capacity(1024));
            bytes.extend_from_slice(black_box(b"test payload"));
            black_box(bytes)
        })
    });

    // Vec allocation
    group.bench_function("vec_allocation", |b| {
        b.iter(|| {
            let mut vec = black_box(Vec::with_capacity(1024));
            vec.extend_from_slice(black_box(b"test payload"));
            black_box(vec)
        })
    });

    group.finish();
}

/// Benchmark throughput for different message sizes
fn bench_throughput_by_size(c: &mut Criterion) {
    let sizes = [64, 256, 1024, 4096, 16384, 65536, 262144, 1048576]; // 64B to 1MB
    
    let mut group = c.benchmark_group("throughput_by_size");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    for size in sizes.iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(BenchmarkId::new("message_creation", size), size, |b, &size| {
            let payload = vec![0u8; size];
            b.iter(|| {
                let request = RpcRequest {
                    service_name: "user_service".to_string(),
                    method_name: "get_user".to_string(),
                    correlation_id: 12345,
                    payload: Bytes::from(payload.clone()),
                    timeout: Some(Duration::from_secs(30)),
                };
                black_box(request)
            })
        });

        group.bench_with_input(BenchmarkId::new("header_creation", size), size, |b, &size| {
            b.iter(|| {
                let header = NeoHeader::new(
                    MessageType::RpcRequest,
                    12345,
                    size as u32,
                );
                black_box(header)
            })
        });
    }

    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Concurrent message creation
    group.bench_function("concurrent_message_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let handles: Vec<_> = (0..100).map(|i| {
                tokio::spawn(async move {
                    let request = RpcRequest {
                        service_name: format!("service_{}", i),
                        method_name: format!("method_{}", i),
                        correlation_id: i as u64,
                        payload: Bytes::from(format!("payload_{}", i)),
                        timeout: Some(Duration::from_secs(30)),
                    };
                    black_box(request)
                })
            }).collect();

            let results = futures::future::join_all(handles).await;
            black_box(results)
        })
    });

    // Concurrent metrics collection
    group.bench_function("concurrent_metrics_collection", |b| {
        let config = MetricsConfig::default();
        let collector = Arc::new(MetricsCollector::new(config));
        
        b.to_async(&rt).iter(|| {
            let collector = collector.clone();
            async move {
                let handles: Vec<_> = (0..100).map(|i| {
                    let collector = collector.clone();
                    tokio::spawn(async move {
                        collector.record_rpc_request();
                        collector.record_rpc_response(Duration::from_millis(i % 100));
                        collector.record_bytes_sent(1024 + i as u64);
                        collector.record_bytes_received(2048 + i as u64);
                        collector.record_method_invocation(&format!("method_{}", i));
                    })
                }).collect();

                futures::future::join_all(handles).await;
            }
        })
    });

    group.finish();
}

/// Benchmark error handling
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Error creation
    group.bench_function("error_creation", |b| {
        b.iter(|| {
            let error = neo_protocol::Error::InvalidMagic(black_box(0x12345678));
            black_box(error)
        })
    });

    // Error conversion
    group.bench_function("error_conversion", |b| {
        b.iter(|| {
            let error: neo_protocol::Error = black_box(anyhow::anyhow!("test error")).into();
            black_box(error)
        })
    });

    // Error display
    group.bench_function("error_display", |b| {
        let error = neo_protocol::Error::InvalidMagic(0x12345678);
        b.iter(|| {
            let display = format!("{}", black_box(&error));
            black_box(display)
        })
    });

    group.finish();
}

/// Benchmark configuration operations
fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);

    // Config creation
    group.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = NeoConfig::default();
            black_box(config)
        })
    });

    // Config cloning
    group.bench_function("config_cloning", |b| {
        let config = NeoConfig::default();
        b.iter(|| {
            let cloned = black_box(&config).clone();
            black_box(cloned)
        })
    });

    // Config serialization
    group.bench_function("config_serialization", |b| {
        let config = NeoConfig::default();
        b.iter(|| {
            let serialized = bincode::serialize(black_box(&config)).unwrap();
            black_box(serialized)
        })
    });

    // Config deserialization
    group.bench_function("config_deserialization", |b| {
        let config = NeoConfig::default();
        let serialized = bincode::serialize(&config).unwrap();
        b.iter(|| {
            let deserialized: NeoConfig = bincode::deserialize(black_box(&serialized)).unwrap();
            black_box(deserialized)
        })
    });

    group.finish();
}

/// Benchmark system performance
fn bench_system_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("system_performance");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    // System time access
    group.bench_function("system_time_access", |b| {
        b.iter(|| {
            let now = black_box(std::time::SystemTime::now());
            black_box(now)
        })
    });

    // Instant access
    group.bench_function("instant_access", |b| {
        b.iter(|| {
            let now = black_box(Instant::now());
            black_box(now)
        })
    });

    // Duration calculation
    group.bench_function("duration_calculation", |b| {
        let start = Instant::now();
        b.iter(|| {
            let elapsed = black_box(start.elapsed());
            black_box(elapsed)
        })
    });

    group.finish();
}

/// Custom benchmark group for comprehensive testing
criterion_group!(
    name = comprehensive_benchmarks;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(30))
        .sample_size(1000)
        .warm_up_time(Duration::from_secs(5))
        .noise_threshold(0.05);
    targets = 
        bench_message_creation,
        bench_header_operations,
        bench_context_operations,
        bench_statistics_operations,
        bench_metrics_collection,
        bench_async_operations,
        bench_memory_allocation,
        bench_throughput_by_size,
        bench_concurrent_operations,
        bench_error_handling,
        bench_config_operations,
        bench_system_performance
);

criterion_main!(comprehensive_benchmarks);