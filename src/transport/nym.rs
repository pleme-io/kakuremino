use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::{AnonTransport, TransportCapability};

/// Default Nym SOCKS5 network requester address.
const DEFAULT_NYM_SOCKS5_ADDR: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    1080,
);

/// Nym mixnet transport backend.
///
/// Connects through the Nym network requester's local SOCKS5 proxy
/// (`nym-socks5-client`). Traffic is routed through the Nym mixnet,
/// providing network-level anonymity via mix nodes and Sphinx packets.
///
/// # Prerequisites
///
/// A running `nym-socks5-client` instance connected to a Nym network
/// requester. By default, it listens on `127.0.0.1:1080`.
///
/// # Limitations
///
/// - Does not support `.onion` addresses (Tor-specific).
/// - DNS resolution is handled by the network requester, not exposed locally.
/// - Higher latency than Tor due to mixnet store-and-forward design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NymTransport {
    /// Address where the Nym SOCKS5 network requester listens.
    socks5_addr: SocketAddr,
}

impl NymTransport {
    /// Create a new Nym transport connecting through the SOCKS5 proxy at the given address.
    #[must_use]
    pub fn new(socks5_addr: SocketAddr) -> Self {
        Self { socks5_addr }
    }

    /// Create a transport using the default Nym SOCKS5 address (`127.0.0.1:1080`).
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(DEFAULT_NYM_SOCKS5_ADDR)
    }

    /// Return the SOCKS5 address this transport connects through.
    #[must_use]
    pub fn socks5_addr(&self) -> SocketAddr {
        self.socks5_addr
    }
}

#[async_trait]
impl AnonTransport for NymTransport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        debug!(
            "nym connect via {} to {target}:{port}",
            self.socks5_addr
        );
        let proxy = self.socks5_addr.to_string();
        let stream = Socks5Stream::connect(&*proxy, (target, port))
            .await
            .map_err(|e| {
                Error::Socks5(format!(
                    "nym connect to {target}:{port} via {}: {e}",
                    self.socks5_addr
                ))
            })?;
        Ok(AnonStream::from_tcp(stream.into_inner()))
    }

    async fn connect_onion(&self, _onion_addr: &str, _port: u16) -> Result<AnonStream> {
        Err(Error::Transport(
            "Nym does not support .onion addresses".to_string(),
        ))
    }

    async fn resolve(&self, _hostname: &str) -> Result<Vec<IpAddr>> {
        Err(Error::Transport(
            "Nym does not support DNS resolution — the network requester resolves hostnames on connect".to_string(),
        ))
    }

    async fn new_identity(&self) -> Result<()> {
        info!("nym new identity (no-op — mixnet provides per-packet anonymity)");
        Ok(())
    }

    fn name(&self) -> &str {
        "nym"
    }

    async fn is_ready(&self) -> bool {
        // Check if the Nym SOCKS5 network requester port is reachable
        TcpStream::connect(self.socks5_addr).await.is_ok()
    }

    fn capabilities(&self) -> Vec<TransportCapability> {
        // Nym provides IP anonymity through its mixnet and DPI resistance
        // via Sphinx packet encryption.
        vec![
            TransportCapability::IpAnonymity,
            TransportCapability::DpiResistant,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_name() {
        let t = NymTransport::default_config();
        assert_eq!(t.name(), "nym");
    }

    #[test]
    fn default_socks5_addr() {
        let t = NymTransport::default_config();
        assert_eq!(
            t.socks5_addr(),
            SocketAddr::from(([127, 0, 0, 1], 1080))
        );
    }

    #[tokio::test]
    async fn onion_not_supported() {
        let t = NymTransport::default_config();
        let result = t.connect_onion("example.onion", 80).await;
        assert!(matches!(result, Err(Error::Transport(_))));
    }

    #[tokio::test]
    async fn resolve_not_supported() {
        let t = NymTransport::default_config();
        let result = t.resolve("example.com").await;
        assert!(matches!(result, Err(Error::Transport(_))));
    }

    #[tokio::test]
    async fn not_ready_when_no_proxy() {
        let t = NymTransport::new(SocketAddr::from(([127, 0, 0, 1], 19996)));
        assert!(!t.is_ready().await);
    }

    #[tokio::test]
    async fn new_identity_is_noop() {
        let t = NymTransport::default_config();
        t.new_identity().await.unwrap();
    }

    #[tokio::test]
    async fn custom_socks5_addr() {
        let addr = SocketAddr::from(([10, 0, 0, 5], 9999));
        let t = NymTransport::new(addr);
        assert_eq!(t.socks5_addr(), addr);
    }

    #[test]
    fn capabilities_has_anonymity_and_dpi() {
        let t = NymTransport::default_config();
        let caps = t.capabilities();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&TransportCapability::IpAnonymity));
        assert!(caps.contains(&TransportCapability::DpiResistant));
    }
}
