# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Data-driven sealing and opening dispatch for the reviewed
  `TLS_AES_128_GCM_SHA256` and `TLS_CHACHA20_POLY1305_SHA256` record routes.
- Fail-closed rejection of `TLS_AES_256_GCM_SHA384` before scratch, output, or
  sequence mutation.
- Stable route-specific errors and one algorithm-erased opened-record view
  that preserves authenticated-plaintext clearing on drop.
