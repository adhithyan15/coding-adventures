# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Added `TlsConnector::connect_addr` so reviewed callers can pin a socket
  address while retaining the canonical server name for SNI and certificate
  verification.
- Added the `TlsConnector` / `TlsStream` trait surface plus a Rustls-backed TLS
  client substrate with WebPKI roots, SNI validation, ALPN policy,
  timeout-aware TCP dialing, peer-certificate access, close-notify support, and
  redacted handshake summaries.
