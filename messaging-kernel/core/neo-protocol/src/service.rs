//! Service definition and registration for Neo Protocol

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Result, Error, NeoContext, SecurityContext};

/// Service method handler
pub type MethodHandler = Arc<dyn Fn(NeoContext, Vec<u8>) -> Result<Vec<u8>> + Send + Sync>;

/// Async service method handler
pub type AsyncMethodHandler = Arc<dyn Fn(NeoContext, Vec<u8>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + Send>> + Send + Sync>;

/// Service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service namespace
    pub namespace: String,
    /// Service description
    pub description: Option<String>,
    /// Service methods
    pub methods: HashMap<String, MethodDefinition>,
    /// Service events
    pub events: HashMap<String, EventDefinition>,
    /// Service metadata
    pub metadata: HashMap<String, String>,
}

/// Method definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodDefinition {
    /// Method name
    pub name: String,
    /// Method description
    pub description: Option<String>,
    /// Input type
    pub input_type: String,
    /// Output type
    pub output_type: String,
    /// Method timeout
    pub timeout: Option<Duration>,
    /// Method metadata
    pub metadata: HashMap<String, String>,
    /// Whether the method is async
    pub is_async: bool,
}

/// Event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Event name
    pub name: String,
    /// Event description
    pub description: Option<String>,
    /// Event topic
    pub topic: String,
    /// Event data type
    pub data_type: String,
    /// Partition key field
    pub partition_key: Option<String>,
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

/// Service registry
pub struct ServiceRegistry {
    /// Registered services
    services: Arc<RwLock<HashMap<String, RegisteredService>>>,
}

/// Registered service
pub struct RegisteredService {
    /// Service definition
    pub definition: ServiceDefinition,
    /// Method handlers
    pub method_handlers: HashMap<String, MethodHandler>,
    /// Async method handlers
    pub async_method_handlers: HashMap<String, AsyncMethodHandler>,
    /// Event handlers
    pub event_handlers: HashMap<String, EventHandler>,
}

/// Event handler
pub type EventHandler = Arc<dyn Fn(NeoContext, Vec<u8>) -> Result<()> + Send + Sync>;

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a service
    pub async fn register_service(
        &self,
        definition: ServiceDefinition,
        method_handlers: HashMap<String, MethodHandler>,
        async_method_handlers: HashMap<String, AsyncMethodHandler>,
        event_handlers: HashMap<String, EventHandler>,
    ) -> Result<()> {
        let service_name = definition.name.clone();
        
        let registered_service = RegisteredService {
            definition,
            method_handlers,
            async_method_handlers,
            event_handlers,
        };

        let mut services = self.services.write().await;
        services.insert(service_name.clone(), registered_service);
        
        info!("Registered service: {}", service_name);
        Ok(())
    }

    /// Unregister a service
    pub async fn unregister_service(&self, service_name: &str) -> Result<()> {
        let mut services = self.services.write().await;
        if services.remove(service_name).is_some() {
            info!("Unregistered service: {}", service_name);
            Ok(())
        } else {
            Err(Error::ServiceNotFound(service_name.to_string()))
        }
    }

    /// Get a service definition
    pub async fn get_service(&self, service_name: &str) -> Result<ServiceDefinition> {
        let services = self.services.read().await;
        if let Some(service) = services.get(service_name) {
            Ok(service.definition.clone())
        } else {
            Err(Error::ServiceNotFound(service_name.to_string()))
        }
    }

    /// Get all registered services
    pub async fn list_services(&self) -> Vec<String> {
        let services = self.services.read().await;
        services.keys().cloned().collect()
    }

    /// Call a service method
    pub async fn call_method(
        &self,
        service_name: &str,
        method_name: &str,
        context: NeoContext,
        params: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let services = self.services.read().await;
        let service = services.get(service_name)
            .ok_or_else(|| Error::ServiceNotFound(service_name.to_string()))?;

        // Check if method exists
        if !service.definition.methods.contains_key(method_name) {
            return Err(Error::MethodNotFound(service_name.to_string(), method_name.to_string()));
        }

        let method_def = &service.definition.methods[method_name];

        // Try async handler first
        if let Some(handler) = service.async_method_handlers.get(method_name) {
            debug!("Calling async method: {}.{}", service_name, method_name);
            handler(context, params).await
        } else if let Some(handler) = service.method_handlers.get(method_name) {
            debug!("Calling sync method: {}.{}", service_name, method_name);
            handler(context, params)
        } else {
            Err(Error::MethodNotFound(service_name.to_string(), method_name.to_string()))
        }
    }

    /// Publish an event
    pub async fn publish_event(
        &self,
        service_name: &str,
        event_name: &str,
        context: NeoContext,
        data: Vec<u8>,
    ) -> Result<()> {
        let services = self.services.read().await;
        let service = services.get(service_name)
            .ok_or_else(|| Error::ServiceNotFound(service_name.to_string()))?;

        // Check if event exists
        if !service.definition.events.contains_key(event_name) {
            return Err(Error::Protocol(format!("Event not found: {}.{}", service_name, event_name)));
        }

        // Call event handler if registered
        if let Some(handler) = service.event_handlers.get(event_name) {
            debug!("Publishing event: {}.{}", service_name, event_name);
            handler(context, data)
        } else {
            // Event has no handler, which is fine for publishing
            debug!("Event published (no handler): {}.{}", service_name, event_name);
            Ok(())
        }
    }

    /// Get service statistics
    pub async fn get_service_stats(&self, service_name: &str) -> Result<ServiceStats> {
        let services = self.services.read().await;
        if let Some(service) = services.get(service_name) {
            Ok(ServiceStats {
                service_name: service_name.to_string(),
                method_count: service.definition.methods.len(),
                event_count: service.definition.events.len(),
                handler_count: service.method_handlers.len() + service.async_method_handlers.len(),
                event_handler_count: service.event_handlers.len(),
            })
        } else {
            Err(Error::ServiceNotFound(service_name.to_string()))
        }
    }
}

/// Service statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStats {
    /// Service name
    pub service_name: String,
    /// Number of methods
    pub method_count: usize,
    /// Number of events
    pub event_count: usize,
    /// Number of method handlers
    pub handler_count: usize,
    /// Number of event handlers
    pub event_handler_count: usize,
}

/// Service builder for creating service definitions
pub struct ServiceBuilder {
    name: String,
    version: String,
    namespace: String,
    description: Option<String>,
    methods: HashMap<String, MethodDefinition>,
    events: HashMap<String, EventDefinition>,
    metadata: HashMap<String, String>,
}

impl ServiceBuilder {
    /// Create a new service builder
    pub fn new(name: String, version: String, namespace: String) -> Self {
        Self {
            name,
            version,
            namespace,
            description: None,
            methods: HashMap::new(),
            events: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set service description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a method to the service
    pub fn add_method(mut self, method: MethodDefinition) -> Self {
        self.methods.insert(method.name.clone(), method);
        self
    }

    /// Add an event to the service
    pub fn add_event(mut self, event: EventDefinition) -> Self {
        self.events.insert(event.name.clone(), event);
        self
    }

    /// Add metadata to the service
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the service definition
    pub fn build(self) -> ServiceDefinition {
        ServiceDefinition {
            name: self.name,
            version: self.version,
            namespace: self.namespace,
            description: self.description,
            methods: self.methods,
            events: self.events,
            metadata: self.metadata,
        }
    }
}

/// Method builder for creating method definitions
pub struct MethodBuilder {
    name: String,
    description: Option<String>,
    input_type: String,
    output_type: String,
    timeout: Option<Duration>,
    metadata: HashMap<String, String>,
    is_async: bool,
}

impl MethodBuilder {
    /// Create a new method builder
    pub fn new(name: String, input_type: String, output_type: String) -> Self {
        Self {
            name,
            description: None,
            input_type,
            output_type,
            timeout: None,
            metadata: HashMap::new(),
            is_async: false,
        }
    }

    /// Set method description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set method timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Mark method as async
    pub fn async_method(mut self) -> Self {
        self.is_async = true;
        self
    }

    /// Add metadata to the method
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the method definition
    pub fn build(self) -> MethodDefinition {
        MethodDefinition {
            name: self.name,
            description: self.description,
            input_type: self.input_type,
            output_type: self.output_type,
            timeout: self.timeout,
            metadata: self.metadata,
            is_async: self.is_async,
        }
    }
}

/// Event builder for creating event definitions
pub struct EventBuilder {
    name: String,
    description: Option<String>,
    topic: String,
    data_type: String,
    partition_key: Option<String>,
    metadata: HashMap<String, String>,
}

impl EventBuilder {
    /// Create a new event builder
    pub fn new(name: String, topic: String, data_type: String) -> Self {
        Self {
            name,
            description: None,
            topic,
            data_type,
            partition_key: None,
            metadata: HashMap::new(),
        }
    }

    /// Set event description
    pub fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set partition key field
    pub fn partition_key(mut self, partition_key: String) -> Self {
        self.partition_key = Some(partition_key);
        self
    }

    /// Add metadata to the event
    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Build the event definition
    pub fn build(self) -> EventDefinition {
        EventDefinition {
            name: self.name,
            description: self.description,
            topic: self.topic,
            data_type: self.data_type,
            partition_key: self.partition_key,
            metadata: self.metadata,
        }
    }
}

/// Service discovery
pub struct ServiceDiscovery {
    /// Service registry
    registry: Arc<ServiceRegistry>,
    /// Service endpoints
    endpoints: Arc<RwLock<HashMap<String, ServiceEndpoint>>>,
}

/// Service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub service_name: String,
    /// Endpoint address
    pub address: String,
    /// Endpoint port
    pub port: u16,
    /// Endpoint metadata
    pub metadata: HashMap<String, String>,
    /// Last heartbeat
    pub last_heartbeat: std::time::SystemTime,
}

impl ServiceDiscovery {
    /// Create a new service discovery
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self {
            registry,
            endpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a service endpoint
    pub async fn register_endpoint(&self, endpoint: ServiceEndpoint) -> Result<()> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(endpoint.service_name.clone(), endpoint);
        Ok(())
    }

    /// Unregister a service endpoint
    pub async fn unregister_endpoint(&self, service_name: &str) -> Result<()> {
        let mut endpoints = self.endpoints.write().await;
        endpoints.remove(service_name);
        Ok(())
    }

    /// Discover service endpoints
    pub async fn discover_endpoints(&self, service_name: &str) -> Result<Vec<ServiceEndpoint>> {
        let endpoints = self.endpoints.read().await;
        let service_endpoints: Vec<ServiceEndpoint> = endpoints
            .values()
            .filter(|endpoint| endpoint.service_name == service_name)
            .cloned()
            .collect();
        
        if service_endpoints.is_empty() {
            Err(Error::ServiceNotFound(service_name.to_string()))
        } else {
            Ok(service_endpoints)
        }
    }

    /// Get all registered endpoints
    pub async fn list_endpoints(&self) -> Vec<ServiceEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_service_builder() {
        let service = ServiceBuilder::new(
            "test_service".to_string(),
            "1.0.0".to_string(),
            "com.example".to_string(),
        )
        .description("Test service".to_string())
        .add_method(
            MethodBuilder::new(
                "test_method".to_string(),
                "TestInput".to_string(),
                "TestOutput".to_string(),
            )
            .description("Test method".to_string())
            .timeout(Duration::from_secs(30))
            .build(),
        )
        .build();

        assert_eq!(service.name, "test_service");
        assert_eq!(service.version, "1.0.0");
        assert_eq!(service.namespace, "com.example");
        assert_eq!(service.methods.len(), 1);
        assert!(service.methods.contains_key("test_method"));
    }

    #[test]
    fn test_method_builder() {
        let method = MethodBuilder::new(
            "test_method".to_string(),
            "TestInput".to_string(),
            "TestOutput".to_string(),
        )
        .description("Test method".to_string())
        .timeout(Duration::from_secs(30))
        .async_method()
        .build();

        assert_eq!(method.name, "test_method");
        assert_eq!(method.input_type, "TestInput");
        assert_eq!(method.output_type, "TestOutput");
        assert!(method.is_async);
        assert_eq!(method.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_event_builder() {
        let event = EventBuilder::new(
            "test_event".to_string(),
            "test_topic".to_string(),
            "TestData".to_string(),
        )
        .description("Test event".to_string())
        .partition_key("id".to_string())
        .build();

        assert_eq!(event.name, "test_event");
        assert_eq!(event.topic, "test_topic");
        assert_eq!(event.data_type, "TestData");
        assert_eq!(event.partition_key, Some("id".to_string()));
    }

    #[tokio::test]
    async fn test_service_registry() {
        let registry = ServiceRegistry::new();
        
        let service = ServiceBuilder::new(
            "test_service".to_string(),
            "1.0.0".to_string(),
            "com.example".to_string(),
        )
        .add_method(
            MethodBuilder::new(
                "test_method".to_string(),
                "TestInput".to_string(),
                "TestOutput".to_string(),
            )
            .build(),
        )
        .build();

        let method_handlers: HashMap<String, MethodHandler> = HashMap::new();
        let async_method_handlers: HashMap<String, AsyncMethodHandler> = HashMap::new();
        let event_handlers: HashMap<String, EventHandler> = HashMap::new();

        registry.register_service(service, method_handlers, async_method_handlers, event_handlers).await.unwrap();

        let services = registry.list_services().await;
        assert_eq!(services.len(), 1);
        assert!(services.contains(&"test_service".to_string()));

        let service_def = registry.get_service("test_service").await.unwrap();
        assert_eq!(service_def.name, "test_service");
    }
}