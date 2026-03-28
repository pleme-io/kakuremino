use std::net::{IpAddr, SocketAddr};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, info};

use crate::error::{Error, Result};
use crate::stream::AnonStream;
use crate::transport::{AnonTransport, TransportCapability};

/// Default SAMv3 bridge address (I2P router SAM port).
const DEFAULT_SAM_ADDR: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    7656,
);

/// I2P transport backend using the SAMv3 protocol.
///
/// Connects to an I2P router's SAM bridge to establish anonymous streams
/// over the I2P network. The SAMv3 protocol is a simple text-based protocol
/// spoken over TCP to the local I2P router.
///
/// # Prerequisites
///
/// A running I2P router with the SAM bridge enabled (default port 7656).
///
/// # Limitations
///
/// I2P is an overlay network — it does not support clearnet (regular internet)
/// connections. Use `connect_onion()` with `.i2p` (base32) addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2pTransport {
    /// Address of the local SAMv3 bridge.
    sam_addr: SocketAddr,
}

impl I2pTransport {
    /// Create a new I2P transport connecting through the SAM bridge at the given address.
    #[must_use]
    pub fn new(sam_addr: SocketAddr) -> Self {
        Self { sam_addr }
    }

    /// Create a transport using the default SAM bridge address (`127.0.0.1:7656`).
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(DEFAULT_SAM_ADDR)
    }

    /// Return the SAM bridge address this transport connects through.
    #[must_use]
    pub fn sam_addr(&self) -> SocketAddr {
        self.sam_addr
    }

    /// Perform the SAMv3 HELLO handshake on a connected stream.
    ///
    /// Sends `HELLO VERSION` and validates the reply.
    async fn sam_hello(stream: &mut BufReader<TcpStream>) -> Result<()> {
        let inner = stream.get_mut();
        inner
            .write_all(b"HELLO VERSION MIN=3.1 MAX=3.3\n")
            .await
            .map_err(|e| Error::Connection(format!("SAM hello write: {e}")))?;
        inner
            .flush()
            .await
            .map_err(|e| Error::Connection(format!("SAM hello flush: {e}")))?;

        let mut reply = String::new();
        stream
            .read_line(&mut reply)
            .await
            .map_err(|e| Error::Connection(format!("SAM hello read: {e}")))?;

        if !reply.contains("RESULT=OK") {
            return Err(Error::Bootstrap(format!("SAM hello failed: {reply}")));
        }

        debug!("SAM handshake OK: {}", reply.trim());
        Ok(())
    }

    /// Create a transient SAM session and return the stream positioned after the
    /// `SESSION STATUS` reply.
    async fn create_session(
        &self,
        session_id: &str,
    ) -> Result<BufReader<TcpStream>> {
        let tcp = TcpStream::connect(self.sam_addr)
            .await
            .map_err(|e| Error::Connection(format!("SAM connect to {}: {e}", self.sam_addr)))?;

        let mut stream = BufReader::new(tcp);
        Self::sam_hello(&mut stream).await?;

        let cmd = format!("SESSION CREATE STYLE=STREAM ID={session_id} DESTINATION=TRANSIENT\n");
        stream
            .get_mut()
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| Error::Connection(format!("SAM session create write: {e}")))?;
        stream
            .get_mut()
            .flush()
            .await
            .map_err(|e| Error::Connection(format!("SAM session create flush: {e}")))?;

        let mut reply = String::new();
        stream
            .read_line(&mut reply)
            .await
            .map_err(|e| Error::Connection(format!("SAM session status read: {e}")))?;

        if !reply.contains("RESULT=OK") {
            return Err(Error::Bootstrap(format!("SAM session create failed: {reply}")));
        }

        debug!("SAM session created: {}", reply.trim());
        Ok(stream)
    }
}

#[async_trait]
impl AnonTransport for I2pTransport {
    async fn connect(&self, _target: &str, _port: u16) -> Result<AnonStream> {
        Err(Error::Transport(
            "I2P is an overlay network and does not support clearnet connections — use connect_onion() with .i2p addresses".to_string(),
        ))
    }

    async fn connect_onion(&self, onion_addr: &str, _port: u16) -> Result<AnonStream> {
        if !onion_addr.ends_with(".i2p") {
            return Err(Error::InvalidAddress(format!(
                "expected .i2p address, got: {onion_addr}"
            )));
        }

        debug!("i2p connect via SAM {} to {onion_addr}", self.sam_addr);

        // Open a fresh SAM connection for the STREAM CONNECT
        let tcp = TcpStream::connect(self.sam_addr)
            .await
            .map_err(|e| Error::Connection(format!("SAM connect to {}: {e}", self.sam_addr)))?;

        let mut stream = BufReader::new(tcp);
        Self::sam_hello(&mut stream).await?;

        // Create a transient session for this connection
        let session_id = "kakuremino";
        let session_cmd =
            format!("SESSION CREATE STYLE=STREAM ID={session_id} DESTINATION=TRANSIENT\n");
        stream
            .get_mut()
            .write_all(session_cmd.as_bytes())
            .await
            .map_err(|e| Error::Connection(format!("SAM session write: {e}")))?;
        stream
            .get_mut()
            .flush()
            .await
            .map_err(|e| Error::Connection(format!("SAM session flush: {e}")))?;

        let mut reply = String::new();
        stream
            .read_line(&mut reply)
            .await
            .map_err(|e| Error::Connection(format!("SAM session read: {e}")))?;

        if !reply.contains("RESULT=OK") {
            return Err(Error::Bootstrap(format!("SAM session failed: {reply}")));
        }

        // Now connect to the I2P destination on a new stream
        let connect_tcp = TcpStream::connect(self.sam_addr)
            .await
            .map_err(|e| Error::Connection(format!("SAM stream connect to {}: {e}", self.sam_addr)))?;

        let mut connect_stream = BufReader::new(connect_tcp);
        Self::sam_hello(&mut connect_stream).await?;

        let connect_cmd = format!("STREAM CONNECT ID={session_id} DESTINATION={onion_addr}\n");
        connect_stream
            .get_mut()
            .write_all(connect_cmd.as_bytes())
            .await
            .map_err(|e| Error::Connection(format!("SAM stream connect write: {e}")))?;
        connect_stream
            .get_mut()
            .flush()
            .await
            .map_err(|e| Error::Connection(format!("SAM stream connect flush: {e}")))?;

        let mut connect_reply = String::new();
        connect_stream
            .read_line(&mut connect_reply)
            .await
            .map_err(|e| Error::Connection(format!("SAM stream status read: {e}")))?;

        if !connect_reply.contains("RESULT=OK") {
            return Err(Error::Connection(format!(
                "SAM stream connect to {onion_addr} failed: {connect_reply}"
            )));
        }

        info!("i2p connected to {onion_addr} via SAM");

        // The underlying TCP stream is now a tunneled I2P connection
        Ok(AnonStream::from_tcp(connect_stream.into_inner()))
    }

    async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>> {
        debug!("i2p resolve {hostname} via SAM NAMING LOOKUP");

        let tcp = TcpStream::connect(self.sam_addr)
            .await
            .map_err(|e| Error::Connection(format!("SAM connect to {}: {e}", self.sam_addr)))?;

        let mut stream = BufReader::new(tcp);
        Self::sam_hello(&mut stream).await?;

        let cmd = format!("NAMING LOOKUP NAME={hostname}\n");
        stream
            .get_mut()
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| Error::Connection(format!("SAM naming lookup write: {e}")))?;
        stream
            .get_mut()
            .flush()
            .await
            .map_err(|e| Error::Connection(format!("SAM naming lookup flush: {e}")))?;

        let mut reply = String::new();
        stream
            .read_line(&mut reply)
            .await
            .map_err(|e| Error::Connection(format!("SAM naming reply read: {e}")))?;

        if !reply.contains("RESULT=OK") {
            return Err(Error::Connection(format!(
                "SAM naming lookup for {hostname} failed: {reply}"
            )));
        }

        // I2P naming lookup returns a base64 destination, not an IP address.
        // We return an empty vec since I2P destinations are not IP-based.
        // Callers should use the returned destination string with connect_onion().
        debug!("SAM naming lookup reply: {}", reply.trim());
        Err(Error::Transport(
            "I2P naming lookup returns base64 destinations, not IP addresses — use connect_onion() with the .i2p address directly".to_string(),
        ))
    }

    async fn new_identity(&self) -> Result<()> {
        info!("i2p new identity (creating new transient destination)");
        // Creating a new session with DESTINATION=TRANSIENT gives us a new
        // identity. The next connect_onion() call will create a fresh session.
        // Validate the SAM bridge is reachable.
        let session = self.create_session("kakuremino-identity").await?;
        // Session created successfully with a new transient destination — drop it
        drop(session);
        Ok(())
    }

    fn name(&self) -> &str {
        "i2p"
    }

    async fn is_ready(&self) -> bool {
        // Try the SAM HELLO handshake to verify the I2P router is running
        let Ok(tcp) = TcpStream::connect(self.sam_addr).await else {
            return false;
        };
        let mut stream = BufReader::new(tcp);
        Self::sam_hello(&mut stream).await.is_ok()
    }

    fn capabilities(&self) -> Vec<TransportCapability> {
        // I2P provides IP anonymity and DPI resistance through its garlic routing.
        // It does not support .onion addresses (Tor-specific) and uses its own
        // naming system instead of DNS.
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
        let t = I2pTransport::default_config();
        assert_eq!(t.name(), "i2p");
    }

    #[test]
    fn default_sam_addr() {
        let t = I2pTransport::default_config();
        assert_eq!(t.sam_addr(), SocketAddr::from(([127, 0, 0, 1], 7656)));
    }

    #[tokio::test]
    async fn clearnet_not_supported() {
        let t = I2pTransport::default_config();
        let result = t.connect("example.com", 80).await;
        assert!(matches!(result, Err(Error::Transport(_))));
    }

    #[tokio::test]
    async fn invalid_i2p_rejected() {
        let t = I2pTransport::new(SocketAddr::from(([127, 0, 0, 1], 19997)));
        let result = t.connect_onion("not-an-i2p-addr.com", 80).await;
        assert!(matches!(result, Err(Error::InvalidAddress(_))));
    }

    #[tokio::test]
    async fn not_ready_when_no_router() {
        let t = I2pTransport::new(SocketAddr::from(([127, 0, 0, 1], 19997)));
        assert!(!t.is_ready().await);
    }

    #[tokio::test]
    async fn custom_sam_addr() {
        let addr = SocketAddr::from(([192, 168, 1, 100], 7656));
        let t = I2pTransport::new(addr);
        assert_eq!(t.sam_addr(), addr);
    }

    #[test]
    fn capabilities_has_anonymity_and_dpi() {
        let t = I2pTransport::default_config();
        let caps = t.capabilities();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&TransportCapability::IpAnonymity));
        assert!(caps.contains(&TransportCapability::DpiResistant));
    }
}
