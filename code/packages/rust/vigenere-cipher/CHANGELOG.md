# Changelog

## Unreleased - 2026-08-26

- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


## 0.1.0 -- 2026-04-04

### Added
- `encrypt` / `decrypt` -- Vigenere encryption with case preservation
- `find_key_length` -- IC-based key length estimation
- `find_key` -- Chi-squared key recovery
- `break_cipher` -- Fully automatic cryptanalysis
- Comprehensive test suite with parity vectors and cryptanalysis tests
