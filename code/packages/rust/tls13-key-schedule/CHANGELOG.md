# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-10-29

### Added

- Cipher-suite-bound no-PSK TLS 1.3 Early, Handshake, and Master Secret
  transitions for a full ephemeral key exchange.
- Fixed wipe-on-drop intermediate storage and non-cloneable caller-owned output
  storage exposed only through a success-only borrowed view.
- Exact RFC 8448 SHA-256 evidence, an independent SHA-384 vector, closed shared
  secret bounds, fail-closed re-derivation, and redacted diagnostics.
