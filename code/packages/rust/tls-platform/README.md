# tls-platform

`tls-platform` is the first TLS client substrate for secure outbound network
transports.

It defines the durable `TlsConnector` / `TlsStream` trait surface from the
TLS platform spec and ships an initial `RustlsConnector` backend with bundled
WebPKI roots, explicit SNI validation, timeout-aware TCP dialing, and redacted
handshake summaries. Higher level crates such as a future `https-transport` can
compose it with `http1` and `http-core` instead of each package carrying its own
TLS choices.

## What It Provides

- TLS 1.2/1.3 client handshakes through `rustls`
- `TlsConnector` and `TlsStream` traits for future OS backends
- Default bundled WebPKI root trust store in the Rustls backend
- Server-name validation before any socket is opened
- Configurable ALPN protocols, with a `https_default()` helper for `http/1.1`
- Read/write implementations for the secured stream
- Redacted endpoint and connection summaries for telemetry

## What It Does Not Provide

This package is not the internal `secure-host-channel`. Host-to-host control
traffic continues to use the ratcheted secure channel described in
`secure-host-channel.md`. This crate fills the external TLS transport gap for
HTTPS calls such as `api.weather.gov`, Hue bridge HTTPS, and cloud/provider
clients.

This first backend also is not the final per-OS TLS backend set from the spec.
`RootStore::SystemDefault` currently resolves to the Rustls bundled roots; the
future Schannel, Network.framework, and OpenSSL-backed crates should honor each
platform's true system trust store.

## Development

```bash
bash BUILD
```
