# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-17

### Added

- Allocation-free TLS 1.3 sealing and opening for the repository-owned
  `TLS_AES_128_GCM_SHA256` route.
- Exact five-byte ciphertext-header authentication, selected-suite-bound key
  and nonce reservations, explicit inner-content policy, and fixed 16-byte tag
  handling.
- Fail-closed sequence consumption, exact written-prefix scratch clearing after
  inner encoding, and a non-cloneable opened-record view that wipes
  authenticated plaintext on drop.
