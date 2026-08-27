# Changelog

## Unreleased - 2026-08-26

- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


All notable changes to the vigenere-cipher Go package.

## [0.1.0] - 2026-04-04

### Added
- `Encrypt(plaintext, key)` -- Vigenere encryption with case preservation
- `Decrypt(ciphertext, key)` -- Vigenere decryption
- `FindKeyLength(ciphertext, maxLength)` -- IC-based key length estimation
- `FindKey(ciphertext, keyLength)` -- chi-squared key letter recovery
- `BreakCipher(ciphertext)` -- full automatic cipher breaking
- `EnglishFrequencies` variable with standard English letter frequencies
- Literate programming style with extensive inline documentation
- Full test suite with parity vectors matching all language implementations
