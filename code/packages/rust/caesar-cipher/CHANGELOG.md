# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `cipher::encrypt` -- shift each ASCII letter forward through the alphabet, preserving case and passing non-alphabetic characters through unchanged
- `cipher::decrypt` -- reverse a Caesar encryption by negating the shift
- `cipher::rot13` -- convenience function for the special shift-13 case that is its own inverse
- `analysis::ENGLISH_FREQUENCIES` -- constant array of 26 letter frequencies for English text
- `analysis::BruteForceResult` -- struct pairing a candidate shift with the resulting plaintext
- `analysis::brute_force` -- try all 25 non-trivial shifts and return candidate plaintexts
- `analysis::frequency_analysis` -- chi-squared scoring against English letter frequencies to automatically detect the most likely shift
- Comprehensive integration tests for both cipher and analysis modules
- Literate-programming-style documentation with truth tables, worked examples, and algorithmic explanations
