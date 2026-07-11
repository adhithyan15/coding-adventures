# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `rans` crate, in namespace
  `ca::rans`: table-based rANS entropy coding with the same largest-remainder
  normalisation, reverse-order encoder, and O(1)-lookup decoder.
- `AnsTable::build` (throws `std::invalid_argument` on bad counts) with `m` /
  `log2m` / `alphabet_size` / `freq` / `cumfreq` (`std::optional`) accessors;
  `RansEncoder` (`put` / `finish`, `put` throws `std::out_of_range`);
  `RansDecoder` (`get` / `is_exhausted`).
- The `RansDecoder` owns a copy of the input bytes (the Rust borrows `&[u8]`) so
  constructing from a temporary — `RansDecoder(table, enc.finish())` — is
  lifetime-safe. The decode table guarantees `slot - cumfreq ∈ [0, freq)` so the
  decoder never goes out of bounds. Arithmetic is 64-bit.
- Tests cover the table vectors, error cases, short-data rejection, symbol round
  trips (including skewed distributions), and determinism, under GCC and Clang
  via `iso-harness`.
