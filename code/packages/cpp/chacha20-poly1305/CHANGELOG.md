# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `chacha20-poly1305` crate
  (RFC 8439), in namespace `ca`.
- `chacha20_encrypt` — the ChaCha20 stream cipher (encrypt/decrypt).
- `poly1305_mac` — the Poly1305 one-time authenticator, using the
  "poly1305-donna" five-limb (26-bit) representation so no 128-bit integer is
  required; matches the RFC 8439 §2.5.2 vector.
- `aead_encrypt` (returns `{ciphertext, tag}`) / `aead_decrypt` (returns
  `std::optional<std::vector<std::uint8_t>>`, empty on tag failure) — the
  ChaCha20-Poly1305 AEAD with constant-time tag verification; matches the
  RFC 8439 §2.8.2 vector.
- Tests pinned to the RFC 8439 vectors plus round-trip and ciphertext/AAD
  tamper-detection checks, run under GCC and Clang via the `iso-harness`.
