# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `Encrypt(text, shift)` — encrypt plaintext using Caesar shift
- `Decrypt(text, shift)` — decrypt ciphertext using Caesar shift
- `Rot13(text)` — apply ROT13 (shift=13, self-inverse)
- `BruteForce(ciphertext)` — try all 25 possible shifts
- `FrequencyAnalysis(ciphertext)` — guess shift using English letter frequency distribution
- Comprehensive test suite with 95%+ coverage
