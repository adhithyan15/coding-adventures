# Changelog

All notable changes to `caesar-cipher` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `caesar-cipher` crate:
  `encrypt`, `decrypt`, `rot13`, `letter_counts`, `chi_squared`, `brute_force`
  (all 25 candidate decryptions), and `frequency_analysis` (best shift +
  plaintext via a chi-squared attack). `std::string`-based API mirroring the
  crate; shift normalisation and frequency table match it exactly.
- Tests (via the shared `iso-harness`) covering the shift-3 example, wraparound,
  ROT13 round-trips, normalisation, letter counts, brute-force, and the
  frequency-analysis attack — compiled and run under GCC, Clang, and MSVC with
  strict ISO-conformance flags.
