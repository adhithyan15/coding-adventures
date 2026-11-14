# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- SEC-010: `Decrypt` negated the raw shift, and `-math.MinInt` wraps back to `math.MinInt`, so the undo shifted the wrong way and returned wrong plaintext. `Encrypt` was NOT safe either: `shiftRune` computed `pos+shift` before reducing mod 26, which overflows for shifts near `math.MaxInt`. Both now reduce the shift to 0..25 first (`normalizeShift`). Tests: `TestDecryptMinIntIsCorrect`, `TestEncryptMaxIntIsCorrect`, `TestDecryptInvertsEncryptAtIntExtremes`.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped all public functions (`Encrypt`, `Decrypt`, `Rot13`, `BruteForce`, `FrequencyAnalysis`) with the Operations system for automatic timing, structured logging, and panic recovery.

## [0.1.0] - 2026-03-29

### Added

- `Encrypt(text, shift)` — encrypt plaintext using Caesar shift
- `Decrypt(text, shift)` — decrypt ciphertext using Caesar shift
- `Rot13(text)` — apply ROT13 (shift=13, self-inverse)
- `BruteForce(ciphertext)` — try all 25 possible shifts
- `FrequencyAnalysis(ciphertext)` — guess shift using English letter frequency distribution
- Comprehensive test suite with 95%+ coverage
