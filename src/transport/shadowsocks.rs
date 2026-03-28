use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::AnonTransport;

/// Shadowsocks transport backend.
///
/// Connects through a local Shadowsocks client (`ss-local`) which exposes a
/// SOCKS5 interface. The Shadowsocks client handles encryption and tunneling
/// to the remote Shadowsocks server transparently.
///
/// # Prerequisites
///
/// A running `ss-local` instance configured with the appropriate server,
/// password, and cipher method. By default, `ss-local` listens on
/// `127.0.0.1:1080`.
pub struct ShadowsocksTransport {
    /// Address where the local `ss-local` SOCKS5 proxy listens.
    local_socks_addr: SocketAddr,
}

impl ShadowsocksTransport {
    /// Create a new Shadowsocks transport connecting through a local `ss-local` proxy.
    ///
    /// `local_socks_addr` is where `ss-local` listens (e.g., `127.0.0.1:1080`).
    #[must_use]
    pub fn new(local_socks_addr: SocketAddr) -> Self {
        Self { local_socks_addr }
    }

    /// Create a transport using the default `ss-local` SOCKS5 address (`127.0.0.1:1080`).
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(SocketAddr::from(([127, 0, 0, 1], 1080)))
    }

    /// Return the local SOCKS5 address this transport connects through.
    #[must_use]
    pub fn local_socks_addr(&self) -> SocketAddr {
        self.local_socks_addr
    }
}

#[async_trait]
impl AnonTransport for ShadowsocksTransport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        debug!(
            "shadowsocks connect via {} to {target}:{port}",
            self.local_socks_addr
        );
        let proxy = self.local_socks_addr.to_string();
        let stream = Socks5Stream::connect(&*proxy, (target, port))
            .await
            .map_err(|e| {
                Error::Socks5(format!(
                    "shadowsocks connect to {target}:{port} via {}: {e}",
                    self.local_socks_addr
                ))
            })?;
        Ok(AnonStream::from_tcp(stream.into_inner()))
    }

    async fn connect_onion(&self, _onion_addr: &str, _port: u16) -> Result<AnonStream> {
        Err(Error::Transport(
            "Shadowsocks does not support .onion addresses".to_string(),
        ))
    }

    async fn resolve(&self, _hostname: &str) -> Result<Vec<IpAddr>> {
        Err(Error::Transport(
            "Shadowsocks does not support DNS resolution".to_string(),
        ))
    }

    async fn new_identity(&self) -> Result<()> {
        info!("shadowsocks new identity (no-op — reconnections use fresh tunnels)");
        Ok(())
    }

    fn name(&self) -> &str {
        "shadowsocks"
    }

    async fn is_ready(&self) -> bool {
        // Check if the local ss-local SOCKS5 port is reachable
        TcpStream::connect(self.local_socks_addr).await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_name() {
        let t = ShadowsocksTransport::default_config();
        assert_eq!(t.name(), "shadowsocks");
    }

    #[test]
    fn default_config_addr() {
        let t = ShadowsocksTransport::default_config();
        assert_eq!(
            t.local_socks_addr(),
            SocketAddr::from(([127, 0, 0, 1], 1080))
        );
    }

    #[tokio::test]
    async fn onion_not_supported() {
        let t = ShadowsocksTransport::default_config();
        let result = t.connect_onion("example.onion", 80).await;
        assert!(matches!(result, Err(Error::Transport(_))));
    }

    #[tokio::test]
    async fn resolve_not_supported() {
        let t = ShadowsocksTransport::default_config();
        let result = t.resolve("example.com").await;
        assert!(matches!(result, Err(Error::Transport(_))));
    }

    #[tokio::test]
    async fn not_ready_when_no_proxy() {
        let t = ShadowsocksTransport::new(SocketAddr::from(([127, 0, 0, 1], 19998)));
        assert!(!t.is_ready().await);
    }

    #[tokio::test]
    async fn new_identity_is_noop() {
        let t = ShadowsocksTransport::default_config();
        t.new_identity().await.unwrap();
    }
}
