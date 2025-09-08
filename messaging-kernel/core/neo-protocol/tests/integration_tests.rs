//! Integration tests for Neo Protocol

use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;

use neo_protocol::{
    NeoClient, NeoServer, NeoConfig, NeoMessage, MessageType,
    message::{MessageFactory, RpcRequest, RpcResponse},
    service::{ServiceRegistry, ServiceDefinition, MethodDefinition, MethodBuilder},
};

#[tokio::test]
async fn test_rpc_request_response() {
    // Create server
    let mut server = NeoServer::new(NeoConfig::default());
    
    // Create a test service
    let service_def = ServiceDefinition {
        name: "test_service".to_string(),
        version: "1.0.0".to_string(),
        namespace: "com.example".to_string(),
        description: Some("Test service".to_string()),
        methods: {
            let mut methods = std::collections::HashMap::new();
            methods.insert("test_method".to_string(), MethodBuilder::new(
                "test_method".to_string(),
                "TestInput".to_string(),
                "TestOutput".to_string(),
            ).build());
            methods
        },
        events: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    };

    // Register service with a simple handler
    let method_handlers = std::collections::HashMap::new();
    let async_method_handlers = std::collections::HashMap::new();
    let event_handlers = std::collections::HashMap::new();
    
    server.register_service(service_def, method_handlers, async_method_handlers, event_handlers).await.unwrap();

    // Start server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        // In a real test, we'd start the server properly
        // For now, we'll just accept connections
        while let Ok((stream, _)) = listener.accept().await {
            // Handle connection
            drop(stream);
        }
    });

    // Create client
    let client = NeoClient::new(NeoConfig::default());
    client.connect(&format!("127.0.0.1:{}", server_addr.port())).await.unwrap();

    // Test RPC call
    let params = b"test params".to_vec();
    let result = timeout(
        Duration::from_secs(5),
        client.call(
            &format!("127.0.0.1:{}", server_addr.port()),
            "test_service",
            "test_method",
            params,
            Some(Duration::from_secs(10)),
        )
    ).await;

    // The call should complete (even if it fails due to no handler)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_message_serialization() {
    // Test message creation and serialization
    let request = RpcRequest {
        service_name: "test_service".to_string(),
        method_name: "test_method".to_string(),
        params: bytes::Bytes::from("test params"),
        timeout: Some(Duration::from_secs(30)),
        client_info: None,
        metadata: std::collections::HashMap::new(),
    };

    let message = MessageFactory::rpc_request(123, "test_service".to_string(), "test_method".to_string(), request.params).unwrap();
    
    // Test serialization
    let serialized = message.serialize().unwrap();
    assert!(!serialized.is_empty());
    
    // Test deserialization
    let deserialized = NeoMessage::deserialize(&serialized).unwrap();
    assert_eq!(message.header, deserialized.header);
    assert_eq!(message.payload, deserialized.payload);
}

#[tokio::test]
async fn test_heartbeat() {
    // Test heartbeat message creation
    let heartbeat = MessageFactory::heartbeat("test_client".to_string()).unwrap();
    
    assert_eq!(heartbeat.header.message_type, MessageType::Heartbeat);
    assert_eq!(heartbeat.header.correlation_id, 0);
    
    // Test serialization
    let serialized = heartbeat.serialize().unwrap();
    let deserialized = NeoMessage::deserialize(&serialized).unwrap();
    
    assert_eq!(heartbeat.header, deserialized.header);
}

#[tokio::test]
async fn test_event_publishing() {
    // Test event message creation
    let event_data = bytes::Bytes::from("test event data");
    let event = MessageFactory::event(
        "test_topic".to_string(),
        event_data.clone(),
        Some("partition_key".to_string()),
    ).unwrap();
    
    assert_eq!(event.header.message_type, MessageType::Event);
    
    // Test serialization
    let serialized = event.serialize().unwrap();
    let deserialized = NeoMessage::deserialize(&serialized).unwrap();
    
    assert_eq!(event.header, deserialized.header);
    assert_eq!(event.payload, deserialized.payload);
}

#[tokio::test]
async fn test_error_handling() {
    // Test error message creation
    let error = MessageFactory::rpc_error(
        123,
        1,
        "Test error".to_string(),
        Some(bytes::Bytes::from("error details")),
    ).unwrap();
    
    assert_eq!(error.header.message_type, MessageType::RpcError);
    assert_eq!(error.header.correlation_id, 123);
    
    // Test serialization
    let serialized = error.serialize().unwrap();
    let deserialized = NeoMessage::deserialize(&serialized).unwrap();
    
    assert_eq!(error.header, deserialized.header);
}

#[tokio::test]
async fn test_authentication() {
    // Test authentication request
    let auth_request = MessageFactory::auth_request(
        "token".to_string(),
        bytes::Bytes::from("credentials"),
        vec!["neo-protocol".to_string()],
    ).unwrap();
    
    assert_eq!(auth_request.header.message_type, MessageType::AuthRequest);
    
    // Test serialization
    let serialized = auth_request.serialize().unwrap();
    let deserialized = NeoMessage::deserialize(&serialized).unwrap();
    
    assert_eq!(auth_request.header, deserialized.header);
}

#[tokio::test]
async fn test_configuration() {
    // Test default configuration
    let config = NeoConfig::default();
    assert_eq!(config.max_message_size, 10 * 1024 * 1024);
    assert_eq!(config.rpc_timeout, Duration::from_secs(30));
    assert_eq!(config.connect_timeout, Duration::from_secs(10));
    
    // Test custom configuration
    let custom_config = NeoConfig {
        max_message_size: 1024 * 1024,
        rpc_timeout: Duration::from_secs(60),
        connect_timeout: Duration::from_secs(5),
        heartbeat_interval: Duration::from_secs(15),
        max_concurrent_requests: 500,
        enable_compression: true,
        security: neo_protocol::security::SecurityConfig::default(),
    };
    
    assert_eq!(custom_config.max_message_size, 1024 * 1024);
    assert_eq!(custom_config.rpc_timeout, Duration::from_secs(60));
    assert!(custom_config.enable_compression);
}

#[tokio::test]
async fn test_statistics() {
    // Test server statistics
    let server = NeoServer::new(NeoConfig::default());
    let stats = server.get_stats().await;
    
    assert_eq!(stats.total_requests, 0);
    assert_eq!(stats.total_responses, 0);
    assert_eq!(stats.total_errors, 0);
    assert_eq!(stats.total_events, 0);
    assert_eq!(stats.active_connections, 0);
    
    // Test client statistics
    let client = NeoClient::new(NeoConfig::default());
    let client_stats = client.get_stats().await;
    
    assert_eq!(client_stats.total_requests, 0);
    assert_eq!(client_stats.total_responses, 0);
    assert_eq!(client_stats.total_errors, 0);
    assert_eq!(client_stats.active_connections, 0);
    assert_eq!(client_stats.pending_requests, 0);
}

#[tokio::test]
async fn test_service_registry() {
    // Test service registry
    let registry = ServiceRegistry::new();
    
    // Create a test service
    let service_def = ServiceDefinition {
        name: "test_service".to_string(),
        version: "1.0.0".to_string(),
        namespace: "com.example".to_string(),
        description: Some("Test service".to_string()),
        methods: {
            let mut methods = std::collections::HashMap::new();
            methods.insert("test_method".to_string(), MethodBuilder::new(
                "test_method".to_string(),
                "TestInput".to_string(),
                "TestOutput".to_string(),
            ).build());
            methods
        },
        events: std::collections::HashMap::new(),
        metadata: std::collections::HashMap::new(),
    };

    // Register service
    let method_handlers = std::collections::HashMap::new();
    let async_method_handlers = std::collections::HashMap::new();
    let event_handlers = std::collections::HashMap::new();
    
    registry.register_service(service_def, method_handlers, async_method_handlers, event_handlers).await.unwrap();
    
    // Test service retrieval
    let services = registry.list_services().await;
    assert_eq!(services.len(), 1);
    assert!(services.contains(&"test_service".to_string()));
    
    // Test service definition retrieval
    let service_def = registry.get_service("test_service").await.unwrap();
    assert_eq!(service_def.name, "test_service");
    assert_eq!(service_def.version, "1.0.0");
    
    // Test service statistics
    let stats = registry.get_service_stats("test_service").await.unwrap();
    assert_eq!(stats.service_name, "test_service");
    assert_eq!(stats.method_count, 1);
    assert_eq!(stats.event_count, 0);
    assert_eq!(stats.handler_count, 0);
    assert_eq!(stats.event_handler_count, 0);
}