use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("transport not ready: {0}")]
    NotReady(String),

    #[error("connection failed: {0}")]
    Connection(String),

    #[error("bootstrap failed: {0}")]
    Bootstrap(String),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SOCKS5 error: {0}")]
    Socks5(String),

    #[error("Tor error: {0}")]
    Tor(String),

    #[error("operation timed out")]
    Timeout,

    #[error("transport error: {0}")]
    Transport(String),
}
