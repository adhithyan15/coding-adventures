# Changelog

All notable changes to `bitset` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `bitset` crate:
  `ca::bitset` with `set`/`clear`/`toggle`/`test` (auto-grow), `operator&`/`|`/
  `^`/`~` and `and_not`, `popcount`, `size`, `capacity`, `any`/`all`/`none`/
  `empty`, `to_integer` (`std::optional`), `to_binary_string`, and the
  `from_integer` / `from_binary_string` factories.
- Tests (via the shared `iso-harness`) covering bit operations, auto-grow, the
  bitwise operators across sizes, and integer/binary-string conversions —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
