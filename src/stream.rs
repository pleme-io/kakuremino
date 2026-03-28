use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

pin_project! {
    /// Anonymous bidirectional stream.
    ///
    /// Wraps different transport-specific stream types behind a unified
    /// `AsyncRead + AsyncWrite` interface.
    #[project = AnonStreamProj]
    pub enum AnonStream {
        /// Plain TCP stream (direct or SOCKS5-proxied).
        Tcp { #[pin] inner: TcpStream },
        /// Boxed async stream for transport-specific implementations (e.g., Tor DataStream).
        Boxed { #[pin] inner: Pin<Box<dyn AsyncReadWrite>> },
    }
}

/// Trait alias for async streams that are both readable and writable.
pub trait AsyncReadWrite: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncReadWrite for T {}

impl AnonStream {
    /// Create from a TCP stream.
    #[must_use]
    pub fn from_tcp(stream: TcpStream) -> Self {
        Self::Tcp { inner: stream }
    }

    /// Create from a boxed async stream.
    #[must_use]
    pub fn from_boxed(stream: impl AsyncRead + AsyncWrite + Send + Unpin + 'static) -> Self {
        Self::Boxed {
            inner: Box::pin(stream),
        }
    }
}

impl AsyncRead for AnonStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.project() {
            AnonStreamProj::Tcp { inner } => inner.poll_read(cx, buf),
            AnonStreamProj::Boxed { inner } => inner.poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for AnonStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.project() {
            AnonStreamProj::Tcp { inner } => inner.poll_write(cx, buf),
            AnonStreamProj::Boxed { inner } => inner.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            AnonStreamProj::Tcp { inner } => inner.poll_flush(cx),
            AnonStreamProj::Boxed { inner } => inner.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            AnonStreamProj::Tcp { inner } => inner.poll_shutdown(cx),
            AnonStreamProj::Boxed { inner } => inner.poll_shutdown(cx),
        }
    }
}
