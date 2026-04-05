# Changelog

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
