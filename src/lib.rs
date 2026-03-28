pub mod error;
pub mod stream;
pub mod transport;

pub use error::{Error, ErrorKind, Result};
pub use stream::AnonStream;
pub use transport::direct::DirectTransport;
pub use transport::socks5::Socks5Transport;
pub use transport::{AnonTransport, ConnectionHealth, IsolationToken, MockTransport, TransportCapability};

#[cfg(feature = "i2p")]
pub use transport::i2p::I2pTransport;
#[cfg(feature = "nym")]
pub use transport::nym::NymTransport;
#[cfg(feature = "shadowsocks")]
pub use transport::shadowsocks::ShadowsocksTransport;
#[cfg(feature = "tor")]
pub use transport::tor::TorTransport;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_variants() {
        assert_eq!(
            Error::NotReady("init".into()).to_string(),
            "transport not ready: init"
        );
        assert_eq!(
            Error::Connection("refused".into()).to_string(),
            "connection failed: refused"
        );
        assert_eq!(
            Error::Bootstrap("fail".into()).to_string(),
            "bootstrap failed: fail"
        );
        assert_eq!(
            Error::InvalidAddress("bad".into()).to_string(),
            "invalid address: bad"
        );
        assert_eq!(Error::Io("broken".into()).to_string(), "I/O error: broken");
        assert_eq!(
            Error::Socks5("err".into()).to_string(),
            "SOCKS5 error: err"
        );
        assert_eq!(Error::Tor("err".into()).to_string(), "Tor error: err");
        assert_eq!(Error::Timeout.to_string(), "operation timed out");
        assert_eq!(
            Error::Transport("err".into()).to_string(),
            "transport error: err"
        );
    }

    #[test]
    fn error_clone_and_eq() {
        let e1 = Error::Timeout;
        let e2 = e1.clone();
        assert_eq!(e1, e2);

        let e3 = Error::Connection("a".into());
        let e4 = Error::Connection("b".into());
        assert_ne!(e3, e4);
    }

    #[test]
    fn error_is_retryable() {
        assert!(Error::Connection("refused".into()).is_retryable());
        assert!(Error::Timeout.is_retryable());
        assert!(Error::Io("broken pipe".into()).is_retryable());
        assert!(!Error::InvalidAddress("bad".into()).is_retryable());
        assert!(!Error::Bootstrap("fail".into()).is_retryable());
        assert!(!Error::Transport("err".into()).is_retryable());
        assert!(!Error::NotReady("init".into()).is_retryable());
        assert!(!Error::Socks5("err".into()).is_retryable());
        assert!(!Error::Tor("err".into()).is_retryable());
    }

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    // --- ErrorKind tests ---

    #[test]
    fn error_kind_display() {
        assert_eq!(ErrorKind::TransientNetwork.to_string(), "transient network error");
        assert_eq!(
            ErrorKind::PermanentConfig.to_string(),
            "permanent configuration error"
        );
        assert_eq!(
            ErrorKind::AuthenticationFailed.to_string(),
            "authentication failed"
        );
        assert_eq!(ErrorKind::ProtocolViolation.to_string(), "protocol violation");
        assert_eq!(ErrorKind::ResourceExhausted.to_string(), "resource exhausted");
        assert_eq!(ErrorKind::Timeout.to_string(), "timeout");
        assert_eq!(ErrorKind::Unsupported.to_string(), "unsupported operation");
    }

    #[test]
    fn error_kind_clone_eq_hash() {
        let a = ErrorKind::TransientNetwork;
        let b = a;
        assert_eq!(a, b);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ErrorKind::TransientNetwork);
        set.insert(ErrorKind::Timeout);
        set.insert(ErrorKind::TransientNetwork);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn error_kind_mapping() {
        assert_eq!(
            Error::Connection("refused".into()).kind(),
            ErrorKind::TransientNetwork
        );
        assert_eq!(Error::Io("broken".into()).kind(), ErrorKind::TransientNetwork);
        assert_eq!(Error::Timeout.kind(), ErrorKind::Timeout);
        assert_eq!(
            Error::Bootstrap("fail".into()).kind(),
            ErrorKind::PermanentConfig
        );
        assert_eq!(
            Error::InvalidAddress("bad".into()).kind(),
            ErrorKind::PermanentConfig
        );
        assert_eq!(
            Error::NotReady("init".into()).kind(),
            ErrorKind::PermanentConfig
        );
        assert_eq!(
            Error::Socks5("err".into()).kind(),
            ErrorKind::ProtocolViolation
        );
        assert_eq!(Error::Tor("err".into()).kind(), ErrorKind::ProtocolViolation);
        assert_eq!(Error::Transport("err".into()).kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn error_kind_consistency_with_retryable() {
        // Transient network + timeout should be retryable
        let retryable_errors = [
            Error::Connection("refused".into()),
            Error::Timeout,
            Error::Io("broken".into()),
        ];
        for err in &retryable_errors {
            assert!(err.is_retryable());
            assert!(
                err.kind() == ErrorKind::TransientNetwork || err.kind() == ErrorKind::Timeout
            );
        }

        // Non-retryable errors
        let non_retryable_errors = [
            Error::Bootstrap("fail".into()),
            Error::InvalidAddress("bad".into()),
            Error::NotReady("init".into()),
            Error::Socks5("err".into()),
            Error::Tor("err".into()),
            Error::Transport("err".into()),
        ];
        for err in &non_retryable_errors {
            assert!(!err.is_retryable());
        }
    }
}
