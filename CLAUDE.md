# kakuremino — Anonymous Transport Library

Pure Rust anonymous transport abstraction. Like `todoku` (HTTP client) but for
anonymous/privacy networks. Unified `AnonTransport` trait with feature-gated backends.

**Tests:** 62

## Architecture

```
AnonTransport trait (connect, connect_onion, resolve, new_identity, is_ready, capabilities, connect_isolated)
    ├── TorTransport      (arti-client, feature: tor, default)
    ├── Socks5Transport   (tokio-socks, always available)
    ├── DirectTransport   (tokio TCP, always available — testing baseline)
    ├── MockTransport     (always available — deterministic testing)
    ├── I2pTransport      (yosemite SAMv3, feature: i2p — TODO)
    ├── ShadowsocksTransport (feature: shadowsocks — TODO)
    └── NymTransport      (nym-sdk, feature: nym — TODO)
```

`AnonStream` wraps transport-specific streams behind `AsyncRead + AsyncWrite`.

## Key Types

| Type | Kind | Description |
|------|------|-------------|
| `AnonTransport` | Trait | Core transport abstraction (connect, connect_onion, resolve, new_identity, is_ready, capabilities, connect_isolated) |
| `TransportCapability` | Enum | 5 flags: Onion, Resolve, NewIdentity, Isolation, MultiHop |
| `ConnectionHealth` | Struct | Connection health metrics (latency, uptime, errors) |
| `IsolationToken` | Struct | Stream isolation token for circuit separation |
| `ErrorKind` | Enum | 7 variants: NotReady, Connection, Bootstrap, Dns, Timeout, Protocol, Other |
| `Error` | Struct | Clone + PartialEq + is_retryable() + kind() method |
| `AnonStream` | Enum | Tcp / Boxed with pin-project |
| `MockTransport` | Struct | Deterministic transport for testing |

## Key Files

| File | Purpose |
|------|---------|
| `src/transport/mod.rs` | `AnonTransport` trait definition |
| `src/transport/tor.rs` | Tor backend (arti-client 0.28, rustls) |
| `src/transport/socks5.rs` | SOCKS5 proxy backend (tokio-socks) |
| `src/transport/direct.rs` | Direct TCP passthrough (testing) |
| `src/transport/mock.rs` | Mock transport for deterministic testing |
| `src/stream.rs` | `AnonStream` enum (Tcp / Boxed) with pin-project |
| `src/error.rs` | Error types with ErrorKind enum, Clone + PartialEq + is_retryable() |

## Build

```bash
cargo test                          # unit tests (no network)
cargo test --features tor -- --ignored  # Tor integration tests (~30s)
nix run .#check-all                 # fmt + clippy + test
nix build                           # Nix sandbox build
```

## Conventions

- Edition 2024, Rust 1.89.0, MIT, clippy pedantic
- Feature-gated backends — `default = ["tor"]`
- No FFI — arti-client uses rustls, rusqlite uses `bundled` feature
- Tor state persisted by arti-client (SQLite via rusqlite bundled)
- `AnonStream` uses `pin_project_lite` for zero-cost enum projection
- Display on all public enums, PartialEq on all transport structs

## Adding a New Backend

1. Create `src/transport/{name}.rs` implementing `AnonTransport`
2. Add feature gate in `Cargo.toml`
3. Re-export in `src/lib.rs` under `#[cfg(feature = "name")]`
4. Add tests (unit + `#[ignore]` integration)

## Consumers

- `kurayami` — Privacy DNS resolver (resolve through Tor)
- `kakureyado` — Onion service hosting
- `kagami` — Dark web monitor (crawl through Tor)
- `kagemusha` — Network privacy analyzer
- `mamorigami` — VPN→Tor chaining integration
