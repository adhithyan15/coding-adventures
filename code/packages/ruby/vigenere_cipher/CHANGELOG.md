# Changelog

All notable changes to the vigenere_cipher Ruby gem.

## [0.1.0] - 2026-04-04

### Added
- `encrypt(plaintext, key)` -- Vigenere encryption with case preservation
- `decrypt(ciphertext, key)` -- Vigenere decryption
- `find_key_length(ciphertext, max_length)` -- IC-based key length estimation
- `find_key(ciphertext, key_length)` -- chi-squared key letter recovery
- `break_cipher(ciphertext)` -- full automatic cipher breaking
- `ENGLISH_FREQUENCIES` constant for the 26 English letter frequencies
- Literate programming style with extensive inline documentation
- Full test suite with parity vectors matching all language implementations
