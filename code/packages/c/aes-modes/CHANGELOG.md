# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `aes-modes` crate: ECB, CBC, CTR, and
  GCM modes over the sibling `aes` block cipher, with PKCS#7 padding.
- Byte-wise GF(2^128) GHASH for GCM authentication (no 128-bit integers).
- malloc-owned outputs with an `AesmStatus` result; constant-time PKCS#7 and
  GCM tag checks; GCM decryption verifies the tag before returning plaintext.
- Cross-package `deps=` onto `aes` → `gf256` (leaf-to-root source list).
- PKCS#7 pad-length concatenation guarded against size_t overflow.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using NIST SP 800-38A
  (ECB/CBC) and the classic GCM test-case vectors, matching the Rust crate.
