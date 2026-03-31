# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-03-29

### Added

- `encrypt(_:shift:)` — Encrypts plaintext using a Caesar shift with case
  preservation and non-alphabetic passthrough.
- `decrypt(_:shift:)` — Decrypts ciphertext by reversing the Caesar shift.
- `rot13(_:)` — Applies ROT13 encoding (shift of 13), which is its own inverse.
- `bruteForce(_:)` — Tries all 26 possible shifts and returns all decryptions.
- `frequencyAnalysis(_:)` — Uses chi-squared comparison against English letter
  frequencies to automatically determine the most likely shift.
- `BruteForceResult` struct for brute-force and frequency analysis results.
- `englishFrequencies` table with letter frequency data from English corpora.
- Comprehensive test suite with 40+ test cases covering encryption, decryption,
  round-trips, edge cases, ROT13, brute force, and frequency analysis.
- Literate programming style with extensive inline documentation explaining
  the history, math, and implementation of each function.
