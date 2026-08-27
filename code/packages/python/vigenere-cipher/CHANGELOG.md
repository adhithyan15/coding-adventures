# Changelog

## Unreleased - 2026-08-26

- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


All notable changes to the vigenere-cipher Python package.

## [0.1.0] - 2026-04-04

### Added
- `encrypt(plaintext, key)` -- Vigenere encryption with case preservation
- `decrypt(ciphertext, key)` -- Vigenere decryption
- `find_key_length(ciphertext, max_length)` -- IC-based key length estimation
- `find_key(ciphertext, key_length)` -- chi-squared key letter recovery
- `break_cipher(ciphertext)` -- full automatic cipher breaking
- `ENGLISH_FREQUENCIES` constant for the 26 English letter frequencies
- Literate programming style with extensive inline documentation
- Full test suite with parity vectors matching all 9 language implementations
