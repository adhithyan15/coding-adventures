# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `aes-modes` crate, in
  namespace `ca::aes_modes`: ECB, CBC, CTR, and GCM over the sibling header-only
  `aes` block cipher, with PKCS#7 padding.
- Byte-wise GF(2^128) GHASH for GCM authentication (no 128-bit integers).
- `std::vector<std::uint8_t>` in/out; GCM returns `{ciphertext, 16-byte tag}`.
- Validation throws `std::invalid_argument`; a GCM tag mismatch throws
  `AuthenticationError`; decryption verifies the tag before returning plaintext.
- Cross-package `deps=` onto `cpp/aes` → `cpp/gf256`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using NIST SP 800-38A
  (ECB/CBC) and the classic GCM test-case vectors, matching the Rust crate.
