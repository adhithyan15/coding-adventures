# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `argon2d` crate
  (RFC 9106), in namespace `ca`: the data-dependent memory-hard password-hashing
  function.
- `argon2d(...)` / `argon2d_hex(...)` returning `std::vector<std::uint8_t>` /
  `std::string`; `Argon2dOptions` for key / associated data / version. Throws
  `std::invalid_argument` on invalid parameters.
- Argon2 G compression, permutation P, the H' variable-length BLAKE2b extender,
  `index_alpha` reference mapping, and data-dependent segment fill.
- BLAKE2b via the sibling header-only `blake2b` package (cross-package `deps=`).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using the RFC 9106 §5.1
  known-answer plus determinism / sensitivity / validation cases.
