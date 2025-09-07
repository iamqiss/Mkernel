//! Service runtime implementation

use qiss_format::Result;

/// A Neo service runtime
pub struct Service {
    name: String,
    version: String,
    namespace: String,
}

impl Service {
    /// Create a new service
    pub fn new(name: String, version: String, namespace: String) -> Self {
        Self {
            name,
            version,
            namespace,
        }
    }

    /// Start the service
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting service: {} v{}", self.name, self.version);
        Ok(())
    }

    /// Stop the service
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping service: {}", self.name);
        Ok(())
    }
}