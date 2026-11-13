//! Outbound message broker and queue adapters.
//!
//! Messaging ports describe application-level publish operations. This module
//! contains broker connections, topic or queue configuration, serialization,
//! delivery guarantees, and protocol-specific error handling.

pub mod broker;
pub mod producer;
pub mod protocol;
