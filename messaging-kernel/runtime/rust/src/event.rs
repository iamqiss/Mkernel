//! Event runtime implementation

use crate::Result;

/// Event handler trait
#[async_trait::async_trait]
pub trait EventHandler<T> {
    async fn handle(&self, event: T) -> Result<()>;
}