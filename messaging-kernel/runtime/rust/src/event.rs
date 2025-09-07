//! Event runtime implementation

use qiss_format::Result;

/// Event handler trait
#[async_trait::async_trait]
pub trait EventHandler<T> {
    async fn handle(&self, event: T) -> Result<()>;
}