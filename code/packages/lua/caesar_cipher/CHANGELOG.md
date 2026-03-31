# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `encrypt(text, shift)` — shift letters forward, preserving case, passing through non-alpha characters
- `decrypt(text, shift)` — reverse of encrypt (shift letters backward)
- `rot13(text)` — Caesar cipher with shift=13 (self-inverse)
- `brute_force(ciphertext)` — try all 25 non-trivial shifts, return table of candidates
- `frequency_analysis(ciphertext)` — chi-squared analysis against English letter frequencies to find the most likely shift
- `ENGLISH_FREQUENCIES` table — expected letter frequencies for English text (A-Z)
- `VERSION` constant set to `"0.1.0"`
- Comprehensive test suite covering round-trip, case preservation, non-alpha passthrough, empty strings, negative shifts, shift wrapping, ROT13 self-inverse, brute force, and frequency analysis
- LuaRocks rockspec for package distribution
- Literate programming style with extensive inline documentation
