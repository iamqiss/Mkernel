//! Integration tests for Precursor broker

use std::time::Duration;
use std::collections::HashMap;

use precursor_broker::{
    PrecursorBroker, BrokerConfig, Message, Offset,
    queue::{QueueConfig, Queue},
    topic::{TopicConfig, Topic},
    consumer::{Consumer, ConsumerConfig},
    producer::{Producer, ProducerConfig},
    storage::StorageEngine,
};

#[tokio::test]
async fn test_broker_lifecycle() {
    // Create broker with test configuration
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false, // Disable for testing
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    
    // Test start
    broker.start().await.unwrap();
    
    // Test statistics
    let stats = broker.get_stats().await;
    assert_eq!(stats.total_messages_produced, 0);
    assert_eq!(stats.total_messages_consumed, 0);
    assert_eq!(stats.active_producers, 0);
    assert_eq!(stats.active_consumers, 0);
    assert_eq!(stats.active_queues, 0);
    assert_eq!(stats.active_topics, 0);
    
    // Test stop
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_queue_operations() {
    // Create broker
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Create queue
    let queue_config = QueueConfig {
        name: "test_queue".to_string(),
        max_size: 1000,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    
    broker.create_queue("test_queue".to_string(), queue_config).await.unwrap();
    
    // Test produce message
    let payload = bytes::Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    let offset = broker.produce_to_queue("test_queue", message).await.unwrap();
    
    assert_eq!(offset.value(), 0);
    
    // Test consume message
    let consumed_message = broker.consume_from_queue("test_queue", "consumer1").await.unwrap();
    assert!(consumed_message.is_some());
    assert_eq!(consumed_message.unwrap().payload, bytes::Bytes::from("test message"));
    
    // Test statistics
    let stats = broker.get_stats().await;
    assert_eq!(stats.total_messages_produced, 1);
    assert_eq!(stats.total_messages_consumed, 1);
    assert_eq!(stats.active_queues, 1);
    
    // Test delete queue
    broker.delete_queue("test_queue").await.unwrap();
    
    let stats = broker.get_stats().await;
    assert_eq!(stats.active_queues, 0);
    
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_topic_operations() {
    // Create broker
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Create topic
    let topic_config = TopicConfig {
        name: "test_topic".to_string(),
        partition_count: 2,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    
    broker.create_topic("test_topic".to_string(), topic_config).await.unwrap();
    
    // Test produce message
    let payload = bytes::Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    let offset = broker.produce_to_topic("test_topic", message).await.unwrap();
    
    assert_eq!(offset.value(), 0);
    
    // Test consume message
    let consumed_message = broker.consume_from_topic("test_topic", 0, "test_group", "consumer1").await.unwrap();
    assert!(consumed_message.is_some());
    assert_eq!(consumed_message.unwrap().payload, bytes::Bytes::from("test message"));
    
    // Test statistics
    let stats = broker.get_stats().await;
    assert_eq!(stats.total_messages_produced, 1);
    assert_eq!(stats.total_messages_consumed, 1);
    assert_eq!(stats.active_topics, 1);
    
    // Test delete topic
    broker.delete_topic("test_topic").await.unwrap();
    
    let stats = broker.get_stats().await;
    assert_eq!(stats.active_topics, 0);
    
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_producer_consumer() {
    // Create broker
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Create topic
    let topic_config = TopicConfig {
        name: "test_topic".to_string(),
        partition_count: 1,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    
    broker.create_topic("test_topic".to_string(), topic_config).await.unwrap();
    
    // Create producer
    let producer_config = ProducerConfig {
        producer_id: "test_producer".to_string(),
        client_id: "test_client".to_string(),
        ..Default::default()
    };
    
    let producer = broker.create_producer(producer_config).await.unwrap();
    producer.start().await.unwrap();
    
    // Create consumer
    let consumer_config = ConsumerConfig {
        consumer_id: "test_consumer".to_string(),
        consumer_group: "test_group".to_string(),
        client_id: "test_client".to_string(),
        ..Default::default()
    };
    
    let consumer = broker.create_consumer(consumer_config).await.unwrap();
    consumer.start().await.unwrap();
    
    // Subscribe consumer to topic
    consumer.subscribe(vec!["test_topic".to_string()]).await.unwrap();
    
    // Send message
    let payload = bytes::Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    let offset = producer.send_to_topic("test_topic", message).await.unwrap();
    
    assert_eq!(offset.value(), 0);
    
    // Poll for messages
    let messages = consumer.poll(Some(Duration::from_secs(1))).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].payload, bytes::Bytes::from("test message"));
    
    // Test statistics
    let producer_stats = producer.get_stats().await;
    assert_eq!(producer_stats.total_messages, 1);
    assert_eq!(producer_stats.total_bytes, 12); // "test message" length
    
    let consumer_stats = consumer.get_stats().await;
    assert_eq!(consumer_stats.total_messages, 1);
    assert_eq!(consumer_stats.total_bytes, 12);
    
    // Test stop
    producer.stop().await.unwrap();
    consumer.stop().await.unwrap();
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_message_batching() {
    // Create broker
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Create topic
    let topic_config = TopicConfig {
        name: "test_topic".to_string(),
        partition_count: 1,
        retention: Duration::from_secs(3600),
        enable_persistence: false,
        ..Default::default()
    };
    
    broker.create_topic("test_topic".to_string(), topic_config).await.unwrap();
    
    // Create producer with batching
    let producer_config = ProducerConfig {
        producer_id: "test_producer".to_string(),
        client_id: "test_client".to_string(),
        batch_size: 3,
        linger: Duration::from_millis(100),
        ..Default::default()
    };
    
    let producer = broker.create_producer(producer_config).await.unwrap();
    producer.start().await.unwrap();
    
    // Send multiple messages
    let mut messages = Vec::new();
    for i in 0..5 {
        let payload = bytes::Bytes::from(format!("message {}", i));
        let headers = HashMap::new();
        let message = Message::new(payload, headers);
        messages.push(message);
    }
    
    let offsets = producer.send_batch(messages).await.unwrap();
    assert_eq!(offsets.len(), 5);
    
    // Test statistics
    let producer_stats = producer.get_stats().await;
    assert_eq!(producer_stats.total_messages, 5);
    
    producer.stop().await.unwrap();
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_storage_engine() {
    // Create storage engine
    let storage = StorageEngine::new("./test_data".to_string());
    storage.initialize().await.unwrap();
    
    // Test write message
    let payload = bytes::Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    let offset = Offset::new(1);
    
    storage.write_message("test_topic", 0, &message, offset).await.unwrap();
    
    // Test read message
    let read_message = storage.read_message("test_topic", 0, offset).await.unwrap();
    assert!(read_message.is_some());
    assert_eq!(read_message.unwrap().payload, bytes::Bytes::from("test message"));
    
    // Test statistics
    let stats = storage.get_stats().await;
    assert_eq!(stats.total_messages, 1);
    
    // Test flush
    storage.flush().await.unwrap();
}

#[tokio::test]
async fn test_replication() {
    // Create broker with replication enabled
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        enable_replication: true,
        replication_factor: 2,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Test replication statistics
    let stats = broker.get_stats().await;
    assert_eq!(stats.active_topics, 0);
    
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_cluster_management() {
    // Create broker with cluster configuration
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        cluster: precursor_broker::cluster::ClusterConfig {
            cluster_id: "test-cluster".to_string(),
            broker_id: "test-broker".to_string(),
            broker_address: "localhost:9092".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Test cluster statistics
    let stats = broker.get_stats().await;
    assert_eq!(stats.active_topics, 0);
    
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_error_handling() {
    // Create broker
    let config = BrokerConfig {
        broker_id: "test-broker".to_string(),
        data_dir: "./test_data".to_string(),
        enable_persistence: false,
        ..Default::default()
    };
    
    let mut broker = PrecursorBroker::new(config);
    broker.start().await.unwrap();
    
    // Test consuming from non-existent queue
    let result = broker.consume_from_queue("non_existent_queue", "consumer1").await;
    assert!(result.is_err());
    
    // Test producing to non-existent topic
    let payload = bytes::Bytes::from("test message");
    let headers = HashMap::new();
    let message = Message::new(payload, headers);
    let result = broker.produce_to_topic("non_existent_topic", message).await;
    assert!(result.is_err());
    
    broker.stop().await.unwrap();
}

#[tokio::test]
async fn test_message_metadata() {
    // Test message creation with metadata
    let payload = bytes::Bytes::from("test message");
    let mut headers = HashMap::new();
    headers.insert("key1".to_string(), "value1".to_string());
    headers.insert("key2".to_string(), "value2".to_string());
    
    let message = Message::new(payload, headers);
    
    assert_eq!(message.size(), 12); // "test message" length
    assert!(message.timestamp() <= std::time::SystemTime::now());
    assert_eq!(message.get_header("key1"), Some(&"value1".to_string()));
    assert_eq!(message.get_header("key2"), Some(&"value2".to_string()));
    assert_eq!(message.get_header("key3"), None);
    
    // Test message with partition key
    let message_with_key = Message::with_partition_key(
        bytes::Bytes::from("test message"),
        HashMap::new(),
        "partition_key".to_string(),
    );
    
    assert_eq!(message_with_key.partition_key(), Some(&"partition_key".to_string()));
}

#[tokio::test]
async fn test_offset_operations() {
    // Test offset creation and operations
    let offset = Offset::new(100);
    assert_eq!(offset.value(), 100);
    
    let incremented = offset.increment();
    assert_eq!(incremented.value(), 101);
    
    let decremented = offset.decrement();
    assert_eq!(decremented.value(), 99);
    
    // Test offset comparison
    let offset1 = Offset::new(100);
    let offset2 = Offset::new(200);
    assert!(offset1 < offset2);
    assert!(offset2 > offset1);
    assert_eq!(offset1, Offset::new(100));
}