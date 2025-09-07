//! RPC runtime implementation

use crate::Result;

/// RPC handler trait
#[async_trait::async_trait]
pub trait RpcHandler<T, R> {
    async fn handle(&self, request: T) -> Result<R>;
}