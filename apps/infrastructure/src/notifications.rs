//! Outbound notification transports.
//!
//! Notification ports belong to `shared::ports`. This module implements
//! the delivery mechanisms and provider-specific behavior for email and SMS.

pub mod email;
pub mod sms;
