//! Rust runtime for Neo messaging kernel

#![warn(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod prelude;
pub mod service;
pub mod rpc;
pub mod event;

pub use prelude::*;