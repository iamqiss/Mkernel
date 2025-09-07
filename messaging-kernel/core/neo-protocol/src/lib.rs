//! High-performance RPC protocol implementation
//! 
//! This crate is part of the Neo Messaging Kernel, providing high-performance rpc protocol implementation.

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod error;

pub use error::Error;

/// Result type for this crate
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn basic_functionality() {
        // TODO: Add basic functionality tests
        assert!(true);
    }
}
