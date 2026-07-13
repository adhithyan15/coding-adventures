# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `x25519` crate: X25519 (Curve25519 ECDH,
  RFC 7748) via a constant-time Montgomery ladder over GF(2^255 - 19).
- Public API: `x25519`, `x25519_base`, `x25519_generate_keypair` (32-byte
  little-endian keys into a caller buffer; `-1` on an all-zeros / low-order
  point output), and the `X25519_BASE_POINT` constant.
- Field arithmetic in radix-2^51 (add / sub / mul / square / invert / (de)serialize
  / constant-time cswap) with a small contained 128-bit emulation (64×64→128
  multiply plus add/shift) replacing the Rust `u128` — no `__int128` needed.
- Rust `Result` becomes a status return; otherwise byte-identical semantics.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert the authoritative
  RFC 7748 §5.2 vectors, the §6.1 Diffie-Hellman worked example (both
  directions), and the 1-round and 1000-round iterated stress vectors.
