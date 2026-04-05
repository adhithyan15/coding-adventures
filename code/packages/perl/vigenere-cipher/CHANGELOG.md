# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added
- Initial implementation of the Vigenere cipher in Perl.
- `encrypt($plaintext, $key)` -- shift each letter forward by keyword amount.
- `decrypt($ciphertext, $key)` -- shift each letter backward by keyword amount.
- `find_key_length($ciphertext, $max_length)` -- IC-based key length detection.
- `find_key($ciphertext, $key_length)` -- chi-squared key recovery per position.
- `break_cipher($ciphertext)` -- automatic full break combining both analyses.
- Comprehensive test suite with parity vectors and cryptanalysis tests.
- Literate programming style with inline explanations.
