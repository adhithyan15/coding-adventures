# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `chacha20-poly1305` crate (RFC 8439).
- `chacha20_encrypt` — the ChaCha20 stream cipher (encrypt/decrypt).
- `poly1305_mac` — the Poly1305 one-time authenticator, using the
  "poly1305-donna" five-limb (26-bit) representation so no 128-bit integer is
  required; matches the RFC 8439 §2.5.2 vector.
- `aead_encrypt` / `aead_decrypt` — the ChaCha20-Poly1305 AEAD with
  constant-time tag verification; matches the RFC 8439 §2.8.2 vector.
- Overflow-guarded allocation in the AEAD MAC-data builder (`size_t` addition
  checks; `NULL` on overflow).
- Tests pinned to the RFC 8439 vectors plus round-trip and ciphertext/AAD
  tamper-detection checks, run under GCC and Clang via the `iso-harness`.
