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

use std::net::IpAddr;

use async_trait::async_trait;

use crate::error::Result;
use crate::stream::AnonStream;

/// Core trait for anonymous transport backends.
///
/// Implementations provide connectivity through different anonymity networks
/// (Tor, SOCKS5 proxy, or direct passthrough for testing).
#[async_trait]
pub trait AnonTransport: Send + Sync {
    /// Connect to a destination anonymously.
    async fn connect(&self, target: &str, port: u16) -> Result<AnonStream>;

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
}
