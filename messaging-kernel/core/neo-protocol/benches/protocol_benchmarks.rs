//! Performance benchmarks for Neo Protocol

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;
use bytes::Bytes;

use neo_protocol::{
    NeoMessage, MessageType, MessageFactory,
    message::{RpcRequest, RpcResponse, RpcError},
    codec::{NeoCodec, AsyncNeoCodec, FramedNeoCodec},
};

fn benchmark_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");
    
    group.bench_function("rpc_request", |b| {
        b.iter(|| {
            let request = MessageFactory::rpc_request(
                black_box(123),
                black_box("test_service".to_string()),
                black_box("test_method".to_string()),
                black_box(Bytes::from("test params")),
            ).unwrap();
            black_box(request)
        })
    });
    
    group.bench_function("rpc_response", |b| {
        b.iter(|| {
            let response = MessageFactory::rpc_response(
                black_box(123),
                black_box(Bytes::from("test response")),
                black_box(Duration::from_millis(100)),
            ).unwrap();
            black_box(response)
        })
    });
    
    group.bench_function("rpc_error", |b| {
        b.iter(|| {
            let error = MessageFactory::rpc_error(
                black_box(123),
                black_box(1),
                black_box("Test error".to_string()),
                black_box(Some(Bytes::from("error details"))),
            ).unwrap();
            black_box(error)
        })
    });
    
    group.bench_function("event", |b| {
        b.iter(|| {
            let event = MessageFactory::event(
                black_box("test_topic".to_string()),
                black_box(Bytes::from("test event data")),
                black_box(Some("partition_key".to_string())),
            ).unwrap();
            black_box(event)
        })
    });
    
    group.bench_function("heartbeat", |b| {
        b.iter(|| {
            let heartbeat = MessageFactory::heartbeat(
                black_box("test_client".to_string()),
            ).unwrap();
            black_box(heartbeat)
        })
    });
    
    group.finish();
}

fn benchmark_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");
    
    // Create test messages
    let rpc_request = MessageFactory::rpc_request(
        123,
        "test_service".to_string(),
        "test_method".to_string(),
        Bytes::from("test params"),
    ).unwrap();
    
    let rpc_response = MessageFactory::rpc_response(
        123,
        Bytes::from("test response"),
        Duration::from_millis(100),
    ).unwrap();
    
    let event = MessageFactory::event(
        "test_topic".to_string(),
        Bytes::from("test event data"),
        Some("partition_key".to_string()),
    ).unwrap();
    
    group.bench_function("rpc_request_serialize", |b| {
        b.iter(|| {
            let serialized = rpc_request.serialize().unwrap();
            black_box(serialized)
        })
    });
    
    group.bench_function("rpc_response_serialize", |b| {
        b.iter(|| {
            let serialized = rpc_response.serialize().unwrap();
            black_box(serialized)
        })
    });
    
    group.bench_function("event_serialize", |b| {
        b.iter(|| {
            let serialized = event.serialize().unwrap();
            black_box(serialized)
        })
    });
    
    group.finish();
}

fn benchmark_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_deserialization");
    
    // Create and serialize test messages
    let rpc_request = MessageFactory::rpc_request(
        123,
        "test_service".to_string(),
        "test_method".to_string(),
        Bytes::from("test params"),
    ).unwrap();
    let rpc_request_data = rpc_request.serialize().unwrap();
    
    let rpc_response = MessageFactory::rpc_response(
        123,
        Bytes::from("test response"),
        Duration::from_millis(100),
    ).unwrap();
    let rpc_response_data = rpc_response.serialize().unwrap();
    
    let event = MessageFactory::event(
        "test_topic".to_string(),
        Bytes::from("test event data"),
        Some("partition_key".to_string()),
    ).unwrap();
    let event_data = event.serialize().unwrap();
    
    group.bench_function("rpc_request_deserialize", |b| {
        b.iter(|| {
            let deserialized = NeoMessage::deserialize(&rpc_request_data).unwrap();
            black_box(deserialized)
        })
    });
    
    group.bench_function("rpc_response_deserialize", |b| {
        b.iter(|| {
            let deserialized = NeoMessage::deserialize(&rpc_response_data).unwrap();
            black_box(deserialized)
        })
    });
    
    group.bench_function("event_deserialize", |b| {
        b.iter(|| {
            let deserialized = NeoMessage::deserialize(&event_data).unwrap();
            black_box(deserialized)
        })
    });
    
    group.finish();
}

fn benchmark_codec_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_performance");
    
    // Create test message
    let message = MessageFactory::rpc_request(
        123,
        "test_service".to_string(),
        "test_method".to_string(),
        Bytes::from("test params"),
    ).unwrap();
    
    group.bench_function("neo_codec_encode", |b| {
        b.iter(|| {
            let encoded = NeoCodec::encode(&message).unwrap();
            black_box(encoded)
        })
    });
    
    group.bench_function("neo_codec_decode", |b| {
        let encoded = NeoCodec::encode(&message).unwrap();
        b.iter(|| {
            let decoded = NeoCodec::decode(&encoded).unwrap();
            black_box(decoded)
        })
    });
    
    group.bench_function("framed_codec_encode", |b| {
        b.iter(|| {
            let encoded = FramedNeoCodec::encode_framed(&message).unwrap();
            black_box(encoded)
        })
    });
    
    group.bench_function("framed_codec_decode", |b| {
        let encoded = FramedNeoCodec::encode_framed(&message).unwrap();
        b.iter(|| {
            let decoded = FramedNeoCodec::decode_framed(&encoded).unwrap();
            black_box(decoded)
        })
    });
    
    group.finish();
}

fn benchmark_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_sizes");
    
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536];
    
    for size in sizes {
        let payload = Bytes::from(vec![0u8; size]);
        
        group.bench_with_input(BenchmarkId::new("rpc_request", size), &size, |b, &size| {
            b.iter(|| {
                let request = MessageFactory::rpc_request(
                    123,
                    "test_service".to_string(),
                    "test_method".to_string(),
                    payload.clone(),
                ).unwrap();
                let serialized = request.serialize().unwrap();
                black_box(serialized)
            })
        });
        
        group.bench_with_input(BenchmarkId::new("event", size), &size, |b, &size| {
            b.iter(|| {
                let event = MessageFactory::event(
                    "test_topic".to_string(),
                    payload.clone(),
                    Some("partition_key".to_string()),
                ).unwrap();
                let serialized = event.serialize().unwrap();
                black_box(serialized)
            })
        });
    }
    
    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    
    group.bench_function("concurrent_serialization", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..100).map(|i| {
                std::thread::spawn(move || {
                    let request = MessageFactory::rpc_request(
                        i,
                        format!("service_{}", i),
                        format!("method_{}", i),
                        Bytes::from(format!("params_{}", i)),
                    ).unwrap();
                    request.serialize().unwrap()
                })
            }).collect();
            
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            black_box(results)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_message_creation,
    benchmark_message_serialization,
    benchmark_message_deserialization,
    benchmark_codec_performance,
    benchmark_message_sizes,
    benchmark_concurrent_operations
);

criterion_main!(benches);