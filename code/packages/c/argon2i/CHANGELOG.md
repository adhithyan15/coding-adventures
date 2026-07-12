# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `argon2i` crate (RFC 9106): the
  data-independent memory-hard password-hashing function.
- `argon2i(...)` writing the tag into a caller-provided buffer with an
  `Argon2iStatus` result; full parameter validation matching the Rust variants.
- Argon2 G compression, permutation P, the H' variable-length BLAKE2b extender,
  the RFC 9106 §3.4 `index_alpha` reference mapping, and data-independent segment
  fill.
- BLAKE2b via the sibling `blake2b` package (cross-package `deps=`); the H'
  extender is layered on the digest-size-1..64 one-shot.
- Checked `calloc` for the working matrix; `H'` input-length overflow guard.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) using the RFC 9106 §5.2
  known-answer plus determinism / sensitivity / validation cases from the Rust
  crate's tests.
