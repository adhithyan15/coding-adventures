# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added
- Initial implementation of the Vigenere cipher in Swift.
- `encrypt(_:key:)` -- shift each letter forward by keyword amount.
- `decrypt(_:key:)` -- shift each letter backward by keyword amount.
- `findKeyLength(_:maxLength:)` -- IC-based key length detection.
- `findKey(_:keyLength:)` -- chi-squared key recovery per position.
- `breakCipher(_:)` -- automatic full break combining both analyses.
- Comprehensive test suite with parity vectors and cryptanalysis tests.
- Literate programming style with inline explanations.
