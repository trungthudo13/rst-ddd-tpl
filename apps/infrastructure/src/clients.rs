//! Outbound service client adapters.
//!
//! Clients own transport concerns such as endpoints, authentication, timeouts,
//! retries, serialization, and protocol-specific error mapping. Business-facing
//! client interfaces are declared in `shared::ports`; use cases own their policy.

pub mod internal_service;
pub mod third_party;
