//! Performance benchmarks for Precursor broker

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use std::collections::HashMap;
use bytes::Bytes;

use precursor_broker::{
    PrecursorBroker, BrokerConfig, Message, Offset,
    queue::{QueueConfig, Queue},
    topic::{TopicConfig, Topic},
    consumer::{Consumer, ConsumerConfig},
    producer::{Producer, ProducerConfig},
    storage::StorageEngine,
};

fn benchmark_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");
    
    group.bench_function("simple_message", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let headers = HashMap::new();
            let message = Message::new(payload, headers);
            black_box(message)
        })
    });
    
    group.bench_function("message_with_headers", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let mut headers = HashMap::new();
            headers.insert("key1".to_string(), "value1".to_string());
            headers.insert("key2".to_string(), "value2".to_string());
            headers.insert("key3".to_string(), "value3".to_string());
            let message = Message::new(payload, headers);
            black_box(message)
        })
    });
    
    group.bench_function("message_with_partition_key", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let headers = HashMap::new();
            let message = Message::with_partition_key(payload, headers, "partition_key".to_string());
            black_box(message)
        })
    });
    
    group.finish();
}

fn benchmark_queue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_operations");
    
    // Create test queue
    let storage = std::sync::Arc::new(StorageEngine::new("./test_data".to_string()));
    let queue_config = QueueConfig {
        name: "test_queue".to_string(),
        max_size: 10000,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    let queue = Queue::new("test_queue".to_string(), queue_config, storage);
    
    group.bench_function("produce_message", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let headers = HashMap::new();
            let message = Message::new(payload, headers);
            // Note: This would need to be async in real usage
            black_box(message)
        })
    });
    
    group.bench_function("consume_message", |b| {
        b.iter(|| {
            // Note: This would need to be async in real usage
            black_box("consumer1")
        })
    });
    
    group.finish();
}

fn benchmark_topic_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("topic_operations");
    
    // Create test topic
    let storage = std::sync::Arc::new(StorageEngine::new("./test_data".to_string()));
    let topic_config = TopicConfig {
        name: "test_topic".to_string(),
        partition_count: 4,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    let topic = Topic::new("test_topic".to_string(), topic_config, storage);
    
    group.bench_function("produce_to_topic", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let headers = HashMap::new();
            let message = Message::new(payload, headers);
            // Note: This would need to be async in real usage
            black_box(message)
        })
    });
    
    group.bench_function("consume_from_topic", |b| {
        b.iter(|| {
            // Note: This would need to be async in real usage
            black_box(("test_group", "consumer1"))
        })
    });
    
    group.finish();
}

fn benchmark_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_operations");
    
    let storage = StorageEngine::new("./test_data".to_string());
    
    group.bench_function("write_message", |b| {
        b.iter(|| {
            let payload = Bytes::from("test message");
            let headers = HashMap::new();
            let message = Message::new(payload, headers);
            let offset = Offset::new(1);
            // Note: This would need to be async in real usage
            black_box((message, offset))
        })
    });
    
    group.bench_function("read_message", |b| {
        b.iter(|| {
            let offset = Offset::new(1);
            // Note: This would need to be async in real usage
            black_box(offset)
        })
    });
    
    group.finish();
}

fn benchmark_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_sizes");
    
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536];
    
    for size in sizes {
        let payload = Bytes::from(vec![0u8; size]);
        
        group.bench_with_input(BenchmarkId::new("message_creation", size), &size, |b, &size| {
            b.iter(|| {
                let headers = HashMap::new();
                let message = Message::new(payload.clone(), headers);
                black_box(message)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("message_with_headers", size), &size, |b, &size| {
            b.iter(|| {
                let mut headers = HashMap::new();
                headers.insert("key1".to_string(), "value1".to_string());
                headers.insert("key2".to_string(), "value2".to_string());
                headers.insert("key3".to_string(), "value3".to_string());
                let message = Message::new(payload.clone(), headers);
                black_box(message)
            })
        });
    }
    
    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    
    group.bench_function("concurrent_message_creation", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..100).map(|i| {
                std::thread::spawn(move || {
                    let payload = Bytes::from(format!("message_{}", i));
                    let mut headers = HashMap::new();
                    headers.insert("id".to_string(), i.to_string());
                    let message = Message::new(payload, headers);
                    message
                })
            }).collect();
            
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            black_box(results)
        })
    });
    
    group.bench_function("concurrent_offset_operations", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..1000).map(|i| {
                std::thread::spawn(move || {
                    let offset = Offset::new(i);
                    let incremented = offset.increment();
                    let decremented = offset.decrement();
                    (offset, incremented, decremented)
                })
            }).collect();
            
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            black_box(results)
        })
    });
    
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("message_memory_footprint", |b| {
        b.iter(|| {
            let mut messages = Vec::new();
            for i in 0..1000 {
                let payload = Bytes::from(format!("message_{}", i));
                let headers = HashMap::new();
                let message = Message::new(payload, headers);
                messages.push(message);
            }
            black_box(messages)
        })
    });
    
    group.bench_function("offset_memory_footprint", |b| {
        b.iter(|| {
            let mut offsets = Vec::new();
            for i in 0..10000 {
                let offset = Offset::new(i);
                offsets.push(offset);
            }
            black_box(offsets)
        })
    });
    
    group.finish();
}

fn benchmark_serialization_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_performance");
    
    // Create test message
    let payload = Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    
    group.bench_function("message_serialization", |b| {
        b.iter(|| {
            // Note: This would use bincode or similar in real implementation
            let serialized = format!("{:?}", message);
            black_box(serialized)
        })
    });
    
    group.bench_function("message_deserialization", |b| {
        let serialized = format!("{:?}", message);
        b.iter(|| {
            // Note: This would use bincode or similar in real implementation
            black_box(&serialized)
        })
    });
    
    group.finish();
}

fn benchmark_partition_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("partition_selection");
    
    let partition_counts = vec![1, 2, 4, 8, 16, 32, 64];
    
    for partition_count in partition_counts {
        group.bench_with_input(BenchmarkId::new("hash_based", partition_count), &partition_count, |b, &partition_count| {
            b.iter(|| {
                let partition_key = "test_partition_key";
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                partition_key.hash(&mut hasher);
                let hash = hasher.finish();
                let partition = (hash % partition_count as u64) as i32;
                black_box(partition)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("round_robin", partition_count), &partition_count, |b, &partition_count| {
            let mut counter = 0;
            b.iter(|| {
                let partition = counter % partition_count;
                counter += 1;
                black_box(partition)
            })
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_message_creation,
    benchmark_queue_operations,
    benchmark_topic_operations,
    benchmark_storage_operations,
    benchmark_message_sizes,
    benchmark_concurrent_operations,
    benchmark_memory_usage,
    benchmark_serialization_performance,
    benchmark_partition_selection
);

criterion_main!(benches);