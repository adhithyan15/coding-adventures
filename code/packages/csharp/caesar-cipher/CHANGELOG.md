# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- SEC-010: `decrypt` negated the raw shift (C# `int`). `decrypt` now reduces the shift to 0..25 before negating it. `-int.MinValue` wraps back to `int.MinValue`, so the undo shifted by +2 instead of -2 and returned wrong plaintext. Tests: `decrypt` at the minimum shift against a by-hand value, plus `decrypt(encrypt(t, s), s) == t` for s in {min, min+1, -1, 0, 25, 26, max}. `Encrypt` was already safe (`NormalizeShift` reduces first).

## [0.1.0] - 2026-04-15

### Added

- Caesar cipher implementation with case preservation, wraparound behavior,
  negative shift support, and ROT13
- Brute-force helpers plus chi-squared English-frequency analysis
- High-coverage unit tests for encryption, decryption, attacks, and statistical
  tables
