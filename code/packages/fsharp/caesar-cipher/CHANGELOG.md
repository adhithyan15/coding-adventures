# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- SEC-010: `decrypt` negated the raw shift (F# `int`). `decrypt` now reduces the shift to 0..25 before negating it. `-Int32.MinValue` wraps back to `Int32.MinValue`, so the undo shifted by +2 instead of -2 and returned wrong plaintext. Tests: `decrypt` at the minimum shift against a by-hand value, plus `decrypt(encrypt(t, s), s) == t` for s in {min, min+1, -1, 0, 25, 26, max}. `encrypt` was already safe (`normalizeShift` reduces first).

## [0.1.0] - 2026-04-15

### Added

- Caesar cipher implementation with wraparound shifts, ROT13, and case
  preservation
- Brute-force helpers and chi-squared English-frequency analysis
- Unit tests covering round-trips, statistical attack behavior, and frequency
  table invariants
