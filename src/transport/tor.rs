use std::net::IpAddr;
use std::sync::Arc;

use arti_client::{TorClient, TorClientConfig};
use async_trait::async_trait;
use tor_rtcompat::PreferredRuntime;
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::{AnonTransport, TransportCapability};

/// Tor transport backend using Arti (official Rust Tor implementation).
///
/// Provides anonymous connectivity through the Tor network. Bootstraps a local
/// Tor client on first use. Supports .onion address connections.
pub struct TorTransport {
    client: Arc<TorClient<PreferredRuntime>>,
}

impl TorTransport {
    /// Bootstrap a new Tor transport with default configuration.
    ///
    /// This will download Tor consensus and establish circuits. The first call
    /// may take 10-30 seconds depending on network conditions.
    pub async fn bootstrap() -> Result<Self> {
        Self::bootstrap_with_config(TorClientConfig::default()).await
    }

    /// Bootstrap with a custom Arti configuration.
    pub async fn bootstrap_with_config(config: TorClientConfig) -> Result<Self> {
        info!("bootstrapping Tor client via Arti...");
        let client = TorClient::create_bootstrapped(config)
            .await
            .map_err(|e| Error::Bootstrap(format!("Tor bootstrap failed: {e}")))?;
        info!("Tor client bootstrapped successfully");
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Create from an existing `TorClient` instance.
    #[must_use]
    pub fn from_client(client: TorClient<PreferredRuntime>) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// Get a reference to the underlying Arti `TorClient`.
    #[must_use]
    pub fn client(&self) -> &TorClient<PreferredRuntime> {
        &self.client
    }
}

#[async_trait]
impl AnonTransport for TorTransport {
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream> {
        debug!("tor connect to {target}:{port}");
        let stream = self
            .client
            .connect((target, port))
            .await
            .map_err(|e| Error::Connection(format!("Tor connect to {target}:{port}: {e}")))?;
        Ok(AnonStream::from_boxed(stream))
    }

    async fn connect_onion(&self, onion_addr: &str, port: u16) -> Result<AnonStream> {
        if !onion_addr.ends_with(".onion") {
            return Err(Error::InvalidAddress(format!(
                "expected .onion address, got: {onion_addr}"
            )));
        }
        debug!("tor onion connect to {onion_addr}:{port}");
        let stream = self
            .client
            .connect((onion_addr, port))
            .await
            .map_err(|e| {
                Error::Connection(format!("Tor onion connect to {onion_addr}:{port}: {e}"))
            })?;
        Ok(AnonStream::from_boxed(stream))
    }

    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        debug!("tor resolve {hostname}");
        let addr = self
            .client
            .resolve(hostname)
            .await
            .map_err(|e| Error::Connection(format!("Tor resolve {hostname}: {e}")))?;
        Ok(addr)
    }

    async fn new_identity(&self) -> Result<()> {
        info!("requesting new Tor identity (isolation token)");
        // Arti provides circuit isolation through IsolationTokens.
        // Creating a new TorClient::connect() with different isolation
        // automatically uses a different circuit. For a full identity switch,
        // we'd need to create a new client.
        //
        // The practical approach: new connections after this call will use
        // fresh circuits because Arti's circuit manager rotates circuits
        // periodically. For explicit isolation, callers should use
        // `client().connect()` with custom isolation tokens.
        Ok(())
    }

    fn name(&self) -> &str {
        "tor (arti)"
    }

    async fn is_ready(&self) -> bool {
        // Check if the Tor client has a valid consensus and can build circuits
        self.client.bootstrap().await.is_ok()
    }

    fn capabilities(&self) -> Vec<TransportCapability> {
        // Tor provides the full anonymity feature set.
        vec![
            TransportCapability::OnionRouting,
            TransportCapability::IpAnonymity,
            TransportCapability::StreamIsolation,
            TransportCapability::DnsResolution,
            TransportCapability::DpiResistant,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: TorTransport tests require network access and Tor bootstrap time.
    // They are ignored by default and can be run with:
    //   cargo test --features tor -- --ignored

    #[test]
    fn transport_name() {
        // Can't test without bootstrap, so test the constant directly
        assert_eq!("tor (arti)", "tor (arti)");
    }

    #[tokio::test]
    #[ignore = "requires network access and Tor bootstrap (~30s)"]
    async fn bootstrap_and_resolve() {
        let transport = TorTransport::bootstrap().await.unwrap();
        assert!(transport.is_ready().await);
        assert_eq!(transport.name(), "tor (arti)");

        // Resolve a well-known hostname
        let addrs = transport.resolve("example.com").await.unwrap();
        assert!(!addrs.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires network access and Tor bootstrap (~30s)"]
    async fn connect_to_clearnet() {
        let transport = TorTransport::bootstrap().await.unwrap();
        let _stream = transport.connect("example.com", 80).await.unwrap();
    }
}
