# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- `encrypt(text, shift)` -- encrypt plaintext using Caesar shift
- `decrypt(text, shift)` -- decrypt ciphertext using Caesar shift
- `rot13(text)` -- apply ROT13 (shift=13, self-inverse)
- `brute_force(ciphertext)` -- try all 25 possible shifts
- `frequency_analysis(ciphertext)` -- guess shift using English letter frequency distribution
- Full type annotations (PEP 561 py.typed marker)
- Comprehensive test suite with 95%+ coverage
