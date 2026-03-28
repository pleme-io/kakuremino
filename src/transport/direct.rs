use std::net::IpAddr;

use async_trait::async_trait;
use tokio::net::TcpStream;
use tracing::debug;

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::AnonTransport;

/// Direct TCP transport — no anonymity layer.
///
/// Used for testing and as a baseline. Connects directly to the target
/// without any proxy or onion routing.
pub struct DirectTransport;

impl DirectTransport {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DirectTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AnonTransport for DirectTransport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        debug!("direct connect to {target}:{port}");
        let stream = TcpStream::connect(format!("{target}:{port}"))
            .await
            .map_err(|e| Error::Connection(format!("{target}:{port}: {e}")))?;
        Ok(AnonStream::from_tcp(stream))
    }

    async fn connect_onion(&self, _onion_addr: &str, _port: u16) -> Result<AnonStream> {
        Err(Error::Transport(
            "direct transport does not support .onion addresses".to_string(),
        ))
    }

    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        debug!("direct resolve {hostname}");
        let addrs: Vec<IpAddr> = tokio::net::lookup_host(format!("{hostname}:0"))
            .await
            .map_err(|e| Error::Connection(format!("DNS resolve {hostname}: {e}")))?
            .map(|addr| addr.ip())
            .collect();

        if addrs.is_empty() {
            return Err(Error::Connection(format!(
                "no addresses found for {hostname}"
            )));
        }

        Ok(addrs)
    }

    async fn new_identity(&self) -> Result<()> {
        // No-op for direct transport
        Ok(())
    }

    fn name(&self) -> &str {
        "direct"
    }

    async fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_name() {
        let t = DirectTransport::new();
        assert_eq!(t.name(), "direct");
    }

    #[tokio::test]
    async fn is_always_ready() {
        let t = DirectTransport::new();
        assert!(t.is_ready().await);
    }

    #[tokio::test]
    async fn new_identity_is_noop() {
        let t = DirectTransport::new();
        t.new_identity().await.unwrap();
    }

    #[tokio::test]
    async fn onion_not_supported() {
        let t = DirectTransport::new();
        let result = t.connect_onion("example.onion", 80).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn connect_to_invalid_host_fails() {
        let t = DirectTransport::new();
        let result = t.connect("192.0.2.1", 1).await;
        assert!(result.is_err());
    }
}
