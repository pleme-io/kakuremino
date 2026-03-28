pub mod direct;
#[cfg(feature = "i2p")]
pub mod i2p;
#[cfg(feature = "nym")]
pub mod nym;
#[cfg(feature = "shadowsocks")]
pub mod shadowsocks;
pub mod socks5;
#[cfg(feature = "tor")]
pub mod tor;

use std::fmt;
use std::net::IpAddr;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::stream::AnonStream;

/// Transport capability flags.
///
/// Describes what features a transport backend supports. Inspired by libp2p's
/// capability-based transport discovery, this allows callers to select transports
/// based on required properties rather than hardcoding transport names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportCapability {
    /// Can connect to .onion addresses.
    OnionRouting,
    /// Provides IP address anonymity.
    IpAnonymity,
    /// Supports stream isolation (multiple independent circuits).
    StreamIsolation,
    /// Can resolve DNS names.
    DnsResolution,
    /// Resistant to DPI (deep packet inspection).
    DpiResistant,
}

impl fmt::Display for TransportCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OnionRouting => write!(f, "onion_routing"),
            Self::IpAnonymity => write!(f, "ip_anonymity"),
            Self::StreamIsolation => write!(f, "stream_isolation"),
            Self::DnsResolution => write!(f, "dns_resolution"),
            Self::DpiResistant => write!(f, "dpi_resistant"),
        }
    }
}

/// Connection quality metrics.
///
/// Tracks health and throughput of a transport connection. Inspired by
/// Tailscale's disco protocol for connection quality monitoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionHealth {
    /// Round-trip latency in milliseconds, if measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    /// Total bytes sent through this connection.
    pub bytes_sent: u64,
    /// Total bytes received through this connection.
    pub bytes_received: u64,
    /// ISO 8601 timestamp when the connection was established, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established_at: Option<String>,
    /// Opaque circuit or session identifier, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_id: Option<String>,
}

impl Default for ConnectionHealth {
    fn default() -> Self {
        Self {
            latency_ms: None,
            bytes_sent: 0,
            bytes_received: 0,
            established_at: None,
            circuit_id: None,
        }
    }
}

/// Token for stream isolation.
///
/// Inspired by Arti's `IsolationToken`, each unique token value causes
/// the transport to use a separate circuit or connection path. Connections
/// sharing the same token MAY share a circuit; connections with different
/// tokens MUST NOT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IsolationToken(u64);

impl IsolationToken {
    /// Create a new isolation token with the given value.
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the inner token value.
    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }
}

/// Core trait for anonymous transport backends.
///
/// Implementations provide connectivity through different anonymity networks
/// (Tor, SOCKS5 proxy, or direct passthrough for testing).
#[async_trait]
pub trait AnonTransport: Send + Sync {
    /// Connect to a destination anonymously.
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream>;

    /// Connect to a destination with stream isolation.
    ///
    /// Connections with different [`IsolationToken`] values are guaranteed
    /// to use separate circuits or connection paths. The default
    /// implementation ignores the token and delegates to [`connect`](Self::connect).
    async fn connect_isolated(
        &self,
        target: &str,
        port: u16,
        _token: IsolationToken,
    ) -> Result<AnonStream> {
        self.connect(target, port).await
    }

    /// Connect to a .onion address.
    async fn connect_onion(&self, onion_addr: &str, port: u16) -> Result<AnonStream>;

    /// Resolve a hostname anonymously.
    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>>;

    /// Get a new identity (new circuit / new route).
    async fn new_identity(&self) -> Result<()>;

    /// Transport name for logging.
    fn name(&self) -> &str;

    /// Check if the transport is bootstrapped and ready.
    async fn is_ready(&self) -> bool;

    /// Query which capabilities this transport supports.
    fn capabilities(&self) -> Vec<TransportCapability>;
}

/// Mock transport for testing — returns errors for all operations.
///
/// Useful for testing code that depends on `AnonTransport` without requiring
/// network access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockTransport {
    /// Whether `is_ready` returns `true`.
    pub ready: bool,
    /// Name returned by `name()`.
    pub transport_name: String,
}

impl MockTransport {
    /// Create a new mock transport.
    #[must_use]
    pub fn new(ready: bool) -> Self {
        Self {
            ready,
            transport_name: "mock".to_owned(),
        }
    }
}

#[async_trait]
impl AnonTransport for MockTransport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        Err(crate::Error::Transport(format!(
            "mock transport: connect to {target}:{port}"
        )))
    }

    async fn connect_onion(&self, onion_addr: &str, port: u16) -> Result<AnonStream> {
        Err(crate::Error::Transport(format!(
            "mock transport: connect_onion to {onion_addr}:{port}"
        )))
    }

    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        Err(crate::Error::Transport(format!(
            "mock transport: resolve {hostname}"
        )))
    }

    async fn new_identity(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.transport_name
    }

    async fn is_ready(&self) -> bool {
        self.ready
    }

    fn capabilities(&self) -> Vec<TransportCapability> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_transport_name() {
        let t = MockTransport::new(true);
        assert_eq!(t.name(), "mock");
    }

    #[tokio::test]
    async fn mock_transport_ready() {
        let t = MockTransport::new(true);
        assert!(t.is_ready().await);

        let t = MockTransport::new(false);
        assert!(!t.is_ready().await);
    }

    #[tokio::test]
    async fn mock_transport_connect_fails() {
        let t = MockTransport::new(true);
        let result = t.connect("example.com", 80).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_transport_onion_fails() {
        let t = MockTransport::new(true);
        let result = t.connect_onion("test.onion", 80).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_transport_resolve_fails() {
        let t = MockTransport::new(true);
        let result = t.resolve("example.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_transport_new_identity_ok() {
        let t = MockTransport::new(true);
        t.new_identity().await.unwrap();
    }

    #[test]
    fn mock_transport_clone_eq() {
        let t1 = MockTransport::new(true);
        let t2 = t1.clone();
        assert_eq!(t1, t2);
    }

    #[test]
    fn mock_transport_capabilities_empty() {
        let t = MockTransport::new(true);
        assert!(t.capabilities().is_empty());
    }

    #[tokio::test]
    async fn mock_transport_connect_isolated_fails() {
        let t = MockTransport::new(true);
        let token = IsolationToken::new(42);
        let result = t.connect_isolated("example.com", 80, token).await;
        assert!(result.is_err());
    }

    // --- TransportCapability tests ---

    #[test]
    fn transport_capability_display() {
        assert_eq!(TransportCapability::OnionRouting.to_string(), "onion_routing");
        assert_eq!(TransportCapability::IpAnonymity.to_string(), "ip_anonymity");
        assert_eq!(
            TransportCapability::StreamIsolation.to_string(),
            "stream_isolation"
        );
        assert_eq!(TransportCapability::DnsResolution.to_string(), "dns_resolution");
        assert_eq!(TransportCapability::DpiResistant.to_string(), "dpi_resistant");
    }

    #[test]
    fn transport_capability_clone_eq_hash() {
        let a = TransportCapability::OnionRouting;
        let b = a;
        assert_eq!(a, b);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TransportCapability::OnionRouting);
        set.insert(TransportCapability::IpAnonymity);
        set.insert(TransportCapability::OnionRouting);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn transport_capability_serde_roundtrip() {
        for cap in [
            TransportCapability::OnionRouting,
            TransportCapability::IpAnonymity,
            TransportCapability::StreamIsolation,
            TransportCapability::DnsResolution,
            TransportCapability::DpiResistant,
        ] {
            let json = serde_json::to_string(&cap).unwrap();
            let parsed: TransportCapability = serde_json::from_str(&json).unwrap();
            assert_eq!(cap, parsed);
        }
    }

    // --- ConnectionHealth tests ---

    #[test]
    fn connection_health_default() {
        let h = ConnectionHealth::default();
        assert!(h.latency_ms.is_none());
        assert_eq!(h.bytes_sent, 0);
        assert_eq!(h.bytes_received, 0);
        assert!(h.established_at.is_none());
        assert!(h.circuit_id.is_none());
    }

    #[test]
    fn connection_health_serde_roundtrip() {
        let h = ConnectionHealth {
            latency_ms: Some(42.5),
            bytes_sent: 1024,
            bytes_received: 2048,
            established_at: Some("2026-03-28T12:00:00Z".to_owned()),
            circuit_id: Some("circuit-1".to_owned()),
        };
        let json = serde_json::to_string(&h).unwrap();
        let parsed: ConnectionHealth = serde_json::from_str(&json).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn connection_health_skip_serializing_none() {
        let h = ConnectionHealth::default();
        let json = serde_json::to_string(&h).unwrap();
        assert!(!json.contains("latency_ms"));
        assert!(!json.contains("established_at"));
        assert!(!json.contains("circuit_id"));
    }

    #[test]
    fn connection_health_clone() {
        let h = ConnectionHealth {
            latency_ms: Some(10.0),
            bytes_sent: 100,
            bytes_received: 200,
            established_at: None,
            circuit_id: None,
        };
        let h2 = h.clone();
        assert_eq!(h, h2);
    }

    // --- IsolationToken tests ---

    #[test]
    fn isolation_token_new_and_value() {
        let token = IsolationToken::new(42);
        assert_eq!(token.value(), 42);
    }

    #[test]
    fn isolation_token_eq_hash() {
        let a = IsolationToken::new(1);
        let b = IsolationToken::new(1);
        let c = IsolationToken::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        set.insert(c);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn isolation_token_clone_copy() {
        let a = IsolationToken::new(99);
        let b = a;
        assert_eq!(a, b);
    }
}
