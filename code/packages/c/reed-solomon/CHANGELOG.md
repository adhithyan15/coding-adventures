# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `reed-solomon` crate: Reed-Solomon error
  correction over GF(2^8), built on the sibling `gf256` package
  (`# build-tool: deps=c/gf256`).
- API: `rs_build_generator`, `rs_encode`, `rs_decode` (corrects up to
  t = n_check/2 errors; reports `RS_TOO_MANY_ERRORS`), `rs_syndromes`,
  `rs_error_locator`.
- Full decode pipeline — syndromes, Berlekamp-Massey, Chien search, Forney — all
  in fixed stack buffers (no heap allocation; polynomials bounded by the 255-byte
  GF(256) block size); caller-provided output buffers.
- Tests: generator polynomial, encode/decode round-trips, correction of 1/2/4
  errors, and too-many-errors detection, under GCC and Clang via `iso-harness`.
