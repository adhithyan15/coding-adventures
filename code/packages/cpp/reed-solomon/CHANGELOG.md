# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `reed-solomon` crate, in namespace
  `ca::reed_solomon`: Reed-Solomon error correction over GF(2^8), built on the
  sibling header-only `gf256` package (`# build-tool: deps=cpp/gf256`).
- `build_generator`, `encode`, `decode` (→ `std::optional`; nullopt when
  unrecoverable), `syndromes`, `error_locator`. Invalid arguments throw
  `std::invalid_argument`.
- Full decode pipeline (syndromes, Berlekamp-Massey, Chien search, Forney) with
  `std::vector` polynomials.
- Tests: generator polynomial, encode/decode round-trips, correction of 1/2/4
  errors, and too-many-errors handling, under GCC and Clang via `iso-harness`.
