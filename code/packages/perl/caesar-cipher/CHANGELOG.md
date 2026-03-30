# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `encrypt($plaintext, $shift)` -- Caesar cipher encryption with case preservation
- `decrypt($ciphertext, $shift)` -- decryption via negated shift
- `rot13($text)` -- self-inverse ROT13 convenience function
- `brute_force($ciphertext)` -- enumerate all 25 possible shifts
- `frequency_analysis($ciphertext)` -- chi-squared frequency analysis to detect shift
- English letter frequency table for cryptanalysis
- Literate programming style with extensive inline documentation
- Full test suite: module loading, cipher round-trips, and cryptanalysis
- cpanfile, Makefile.PL, BUILD files for the repo build system
