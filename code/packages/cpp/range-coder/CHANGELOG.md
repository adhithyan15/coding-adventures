# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `range-coder` crate, in namespace
  `ca::range_coder`: the VP8 boolean range coder (RFC 6386 §7) — a binary
  arithmetic coder.
- `BoolEncoder`: `write_bit` / `write_bits` / `finish` (returns
  `std::vector<std::uint8_t>`). `BoolDecoder`: `read_bit` / `read_bits` /
  `is_exhausted`, constructed from a `std::vector<std::uint8_t>` or `(data, len)`.
- The C++ `BoolDecoder` owns a copy of the input (the Rust crate borrows
  `&[u8]`) so that constructing from a temporary — e.g.
  `BoolDecoder(enc.finish())` — is lifetime-safe rather than dangling.
- Arithmetic in `std::uint64_t`/`std::uint32_t` (no 128-bit integers, no libm).
- Tests round-trip single bits, mixed/skewed probability sequences, and
  `write_bits`/`read_bits` for 8/16/32-bit fields, plus seeding, exhaustion, and
  determinism, under GCC and Clang via `iso-harness`.
