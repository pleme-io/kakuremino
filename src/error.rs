use std::fmt;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Categorization of errors by their underlying cause.
///
/// Inspired by Arti's `HasKind` pattern, this provides a richer classification
/// than the simple `is_retryable()` predicate, allowing callers to implement
/// fine-grained retry and fallback strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// A transient network issue that may resolve on retry.
    TransientNetwork,
    /// A permanent configuration error that will not resolve on retry.
    PermanentConfig,
    /// Authentication or authorization failed.
    AuthenticationFailed,
    /// The remote end violated the expected protocol.
    ProtocolViolation,
    /// A resource limit (circuits, file descriptors, memory) was exceeded.
    ResourceExhausted,
    /// The operation timed out.
    Timeout,
    /// The requested operation is not supported by this transport.
    Unsupported,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransientNetwork => write!(f, "transient network error"),
            Self::PermanentConfig => write!(f, "permanent configuration error"),
            Self::AuthenticationFailed => write!(f, "authentication failed"),
            Self::ProtocolViolation => write!(f, "protocol violation"),
            Self::ResourceExhausted => write!(f, "resource exhausted"),
            Self::Timeout => write!(f, "timeout"),
            Self::Unsupported => write!(f, "unsupported operation"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
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
    Io(String),

    #[error("SOCKS5 error: {0}")]
    Socks5(String),

    #[error("Tor error: {0}")]
    Tor(String),

    #[error("operation timed out")]
    Timeout,

    #[error("transport error: {0}")]
    Transport(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl Error {
    /// Returns `true` for transient errors that may succeed on retry.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Connection(_) | Self::Timeout | Self::Io(_))
    }

    /// Categorize this error into an [`ErrorKind`].
    ///
    /// This provides richer classification than `is_retryable()`, enabling
    /// callers to distinguish between transient network issues, permanent
    /// configuration problems, protocol violations, and more.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Connection(_) | Self::Io(_) => ErrorKind::TransientNetwork,
            Self::Timeout => ErrorKind::Timeout,
            Self::Bootstrap(_) | Self::InvalidAddress(_) | Self::NotReady(_) => {
                ErrorKind::PermanentConfig
            }
            Self::Socks5(_) => ErrorKind::ProtocolViolation,
            Self::Tor(_) => ErrorKind::ProtocolViolation,
            Self::Transport(_) => ErrorKind::Unsupported,
        }
    }
}
