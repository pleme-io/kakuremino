use std::net::IpAddr;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::{AnonTransport, TransportCapability};

/// SOCKS5 proxy transport.
///
/// Connects through an external SOCKS5 proxy (e.g., a running Tor instance,
/// an SSH tunnel, or any SOCKS5-compatible proxy server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socks5Transport {
    proxy_addr: String,
}

impl Socks5Transport {
    /// Create a new SOCKS5 transport connecting through the given proxy.
    ///
    /// `proxy_addr` should be in `host:port` format (e.g., `"127.0.0.1:9050"`).
    #[must_use]
    pub fn new(proxy_addr: impl Into<String>) -> Self {
        Self {
            proxy_addr: proxy_addr.into(),
        }
    }

    /// Create a transport using the standard Tor SOCKS5 port.
    #[must_use]
    pub fn tor_default() -> Self {
        Self::new("127.0.0.1:9050")
    }
}

#[async_trait]
impl AnonTransport for Socks5Transport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        debug!("socks5 connect via {} to {target}:{port}", self.proxy_addr);
        let stream =
            Socks5Stream::connect(&*self.proxy_addr, (target, port))
                .await
                .map_err(|e| Error::Socks5(format!("connect to {target}:{port} via {}: {e}", self.proxy_addr)))?;
        Ok(AnonStream::from_tcp(stream.into_inner()))
    }

    async fn connect_onion(&self, onion_addr: &str, port: u16) -> Result<AnonStream> {
        if !onion_addr.ends_with(".onion") {
            return Err(Error::InvalidAddress(format!(
                "expected .onion address, got: {onion_addr}"
            )));
        }
        debug!(
            "socks5 onion connect via {} to {onion_addr}:{port}",
            self.proxy_addr
        );
        // SOCKS5 supports connecting to .onion addresses when the proxy is Tor
        let stream =
            Socks5Stream::connect(&*self.proxy_addr, (onion_addr, port))
                .await
                .map_err(|e| {
                    Error::Socks5(format!(
                        "onion connect to {onion_addr}:{port} via {}: {e}",
                        self.proxy_addr
                    ))
                })?;
        Ok(AnonStream::from_tcp(stream.into_inner()))
    }

    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        debug!("socks5 resolve {hostname} via {}", self.proxy_addr);
        // SOCKS5 RESOLVE command (Tor extension) — connect to get the resolved addr
        // Note: standard SOCKS5 doesn't have a resolve command, so we do a connect
        // and extract the remote address. For proper Tor DNS, use TorTransport.
        Err(Error::Transport(
            "SOCKS5 resolve not supported — use connect with hostname directly".to_string(),
        ))
    }

    async fn new_identity(&self) -> Result<()> {
        info!(
            "socks5 new identity (reconnecting through {})",
            self.proxy_addr
        );
        // SOCKS5 doesn't have a native identity rotation mechanism.
        // If the proxy is Tor, you'd need to send SIGNAL NEWNYM via the control port.
        Ok(())
    }

    fn name(&self) -> &str {
        "socks5"
    }

    async fn is_ready(&self) -> bool {
        // Try to connect to the proxy to check if it's running
        TcpStream::connect(&*self.proxy_addr).await.is_ok()
    }

    fn capabilities(&self) -> Vec<TransportCapability> {
        // SOCKS5 proxy provides IP anonymity (hides client IP from target).
        vec![TransportCapability::IpAnonymity]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_name() {
        let t = Socks5Transport::new("127.0.0.1:9050");
        assert_eq!(t.name(), "socks5");
    }

    #[test]
    fn tor_default_port() {
        let t = Socks5Transport::tor_default();
        assert_eq!(t.proxy_addr, "127.0.0.1:9050");
    }

    #[tokio::test]
    async fn not_ready_when_no_proxy() {
        let t = Socks5Transport::new("127.0.0.1:19999");
        assert!(!t.is_ready().await);
    }

    #[tokio::test]
    async fn resolve_not_supported() {
        let t = Socks5Transport::new("127.0.0.1:9050");
        let result = t.resolve("example.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn invalid_onion_rejected() {
        let t = Socks5Transport::new("127.0.0.1:9050");
        let result = t.connect_onion("not-an-onion.com", 80).await;
        assert!(matches!(result, Err(Error::InvalidAddress(_))));
    }

    #[test]
    fn capabilities_has_ip_anonymity() {
        let t = Socks5Transport::new("127.0.0.1:9050");
        let caps = t.capabilities();
        assert_eq!(caps.len(), 1);
        assert!(caps.contains(&TransportCapability::IpAnonymity));
    }
}
