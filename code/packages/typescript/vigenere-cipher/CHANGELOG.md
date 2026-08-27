# Changelog

## Unreleased - 2026-08-26

- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


## 0.1.0 -- 2026-04-04

### Added
- `encrypt(plaintext, key)` -- Vigenere encryption with case preservation
- `decrypt(ciphertext, key)` -- Vigenere decryption
- `findKeyLength(ciphertext, maxLength?)` -- IC-based key length estimation
- `findKey(ciphertext, keyLength)` -- Chi-squared key recovery
- `breakCipher(ciphertext)` -- Fully automatic cryptanalysis
- Comprehensive test suite with parity vectors and cryptanalysis tests
