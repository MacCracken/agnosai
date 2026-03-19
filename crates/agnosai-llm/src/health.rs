//! Provider health tracking with ring buffer (from SecureYeoman).
//!
//! 5-point ring buffer per provider. After 3 consecutive failures →
//! mark unhealthy → failover to next provider. One success resets.

// TODO: Port from SY health scoring
