# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `scytale-cipher` crate: the Scytale
  transposition cipher (columnar grid with space padding) plus brute force.
- API: `scytale_encrypt`, `scytale_decrypt` (malloc'd result; `""` for empty
  text; `NULL` on an invalid key or allocation failure), and
  `scytale_brute_force` / `scytale_brute_free`.
- Transposes whole UTF-8 characters (not bytes), matching the crate's
  `char`-based behaviour; malformed bytes become single-byte units so any input
  round-trips. Allocations are overflow-guarded (`calloc` checked multiply,
  `padded_len` and byte-total guards).
- Tests use the crate's own vectors — encrypt/decrypt cases, key validation,
  padding stripping, round trips over all valid keys, brute force, and a
  multibyte UTF-8 round trip — under GCC and Clang via `iso-harness`.
