# Changelog

## Unreleased - 2026-08-26

- Execute all 26 normative Vigenere fixture objects through the generated native consumer.
- Align Vigenere transforms and cryptanalysis with CR03 ASCII, limit, ordering, 90%-threshold, tie, and exact-key-length behavior.
- Add direct native regressions for Unicode pass-through, preflight limits, and deterministic low-signal analysis.


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
