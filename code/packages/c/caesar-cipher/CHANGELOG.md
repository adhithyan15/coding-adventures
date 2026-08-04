# Changelog

All notable changes to `caesar-cipher` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `caesar-cipher` crate: `caesar_encrypt`,
  `caesar_decrypt`, `caesar_rot13`, `caesar_letter_counts`, `caesar_chi_squared`,
  and `caesar_frequency_analysis` (chi-squared attack against English letter
  frequencies). Buffer-based string API; shift normalisation and frequency table
  match the Rust crate exactly.
- Tests (via the shared `iso-harness`) covering the shift-3 example, wraparound,
  ROT13 round-trips, shift normalisation, buffer-too-small handling, letter
  counts, and the frequency-analysis attack — compiled and run under GCC, Clang,
  and MSVC with strict ISO-conformance flags.
