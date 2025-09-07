//! RPC runtime implementation

use qiss_format::Result;

/// RPC handler trait
#[async_trait::async_trait]
pub trait RpcHandler<T, R> {
    async fn handle(&self, request: T) -> Result<R>;
}