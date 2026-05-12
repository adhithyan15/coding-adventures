# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Added

- Added the `TlsConnector` / `TlsStream` trait surface plus a Rustls-backed TLS
  client substrate with WebPKI roots, SNI validation, ALPN policy,
  timeout-aware TCP dialing, peer-certificate access, close-notify support, and
  redacted handshake summaries.
