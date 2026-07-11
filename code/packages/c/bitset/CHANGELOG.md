# Changelog

All notable changes to `bitset` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `bitset` crate: `bitset_init`,
  `bitset_from_integer`, `bitset_from_binary_str`, `set`/`clear`/`toggle`/`test`
  (with auto-grow), `and`/`or`/`xor`/`not`/`and_not`, `popcount`, `len`,
  `capacity`, `any`/`all`/`none`/`is_empty`, `to_integer`, `to_binary_str`.
- Tests (via the shared `iso-harness`) covering bit operations, auto-grow, the
  bitwise set operations across differing sizes, and the integer/binary-string
  conversions — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
