# kakuremino

Anonymous transport library for Rust.

Provides a unified `AnonTransport` trait with feature-gated backends for Tor,
SOCKS5, I2P, Shadowsocks, Nym, and direct TCP. Each backend exposes the same
async interface for connecting, resolving, identity rotation, and stream
isolation so callers never depend on a specific anonymity network.

## Quick Start

```bash
cargo test                                    # unit tests (no network)
cargo test --features tor -- --ignored        # Tor integration tests
nix build                                     # Nix hermetic build
```

## Architecture

Single crate, feature-gated backends:

| Backend | Feature | Status | Transport |
|---------|---------|--------|-----------|
| `TorTransport` | `tor` (default) | Complete | arti-client + rustls |
| `Socks5Transport` | always | Complete | tokio-socks |
| `DirectTransport` | always | Complete | tokio TCP |
| `MockTransport` | always | Complete | deterministic testing |
| `I2pTransport` | `i2p` | Planned | SAMv3 |
| `ShadowsocksTransport` | `shadowsocks` | Planned | -- |
| `NymTransport` | `nym` | Planned | nym-sdk |

## Usage

```rust
use kakuremino::{AnonTransport, TransportCapability, IsolationToken};

// Connect through Tor (default feature)
#[cfg(feature = "tor")]
let transport = kakuremino::TorTransport::new(Default::default()).await?;

// Check what the backend supports
let caps = transport.capabilities();
assert!(caps.contains(&TransportCapability::Onion));

// Connect to a .onion address
let stream = transport.connect_onion("examplesite.onion", 80).await?;

// Isolated stream on a separate circuit
let token = IsolationToken::new();
let isolated = transport.connect_isolated("example.com", 443, &token).await?;

// Rotate identity (new Tor circuit)
transport.new_identity().await?;
```

```rust
// SOCKS5 backend (always available, no feature gate)
let proxy = kakuremino::Socks5Transport::new("127.0.0.1:9050");
let stream = proxy.connect("example.com", 80).await?;
```

## License

MIT
