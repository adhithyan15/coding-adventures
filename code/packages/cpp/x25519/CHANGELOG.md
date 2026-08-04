# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `x25519` crate, in
  namespace `ca::x25519`: X25519 (Curve25519 ECDH, RFC 7748) via a constant-time
  Montgomery ladder over GF(2^255 - 19).
- Public API returning `std::optional<Key>` (`Key` = `std::array<uint8_t, 32>`):
  `x25519`, `x25519_base`, `generate_keypair`, and the `BASE_POINT` constant.
  `std::nullopt` signals an all-zeros / low-order point output.
- Radix-2^51 field arithmetic with the same contained 128-bit emulation as the C
  sibling (no `__int128`, which pure ISO C++17 lacks).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert the authoritative
  RFC 7748 §5.2 vectors, the §6.1 worked example (both directions), and the
  1-round and 1000-round iterated stress vectors.
