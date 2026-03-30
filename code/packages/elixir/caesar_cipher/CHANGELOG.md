# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `encrypt/2` — Caesar cipher encryption with shift wrapping and case preservation
- `decrypt/2` — Decryption via negated-shift encryption
- `rot13/1` — ROT13 convenience function (self-inverse Caesar with shift 13)
- `brute_force/1` — Try all 25 non-trivial shifts, return `{shift, plaintext}` tuples
- `frequency_analysis/1` — Chi-squared frequency analysis against English letter frequencies
- Literate programming documentation with history, worked examples, and truth tables
- Comprehensive ExUnit test suite covering round-trips, edge cases, and frequency analysis
