//! Prelude module for Neo runtime

// Re-export specific types to avoid ambiguity
pub use qiss_format::{QissSerializable, QissDeserializable, QissWriter, QissReader, QissSerializer, QissDeserializer, QissCodec, Timestamp};
pub use neo_protocol::Error as ProtocolError;
pub use precursor_broker::Error as BrokerError;

// Use qiss_format's Result type as the default
pub use qiss_format::Result;

pub use crate::service::*;
pub use crate::rpc::*;
pub use crate::event::*;