# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-29

### Added

- `CodingAdventures::CaesarCipher.encrypt(text, shift)` — encrypt plaintext using a Caesar cipher with any integer shift. Preserves case, passes non-alphabetic characters through unchanged, handles negative and large shifts via modular arithmetic.
- `CodingAdventures::CaesarCipher.decrypt(text, shift)` — decrypt ciphertext by reversing the shift. Implemented as `encrypt(text, -shift)` for simplicity.
- `CodingAdventures::CaesarCipher.rot13(text)` — the special case of Caesar cipher with shift 13, which is its own inverse (applying it twice returns the original text).
- `CodingAdventures::CaesarCipher.brute_force(ciphertext)` — try all 25 non-trivial shifts (1..25) and return an array of `[shift, plaintext]` pairs for human inspection.
- `CodingAdventures::CaesarCipher.frequency_analysis(ciphertext)` — automatically crack a Caesar cipher using chi-squared comparison against English letter frequencies. Returns the best `[shift, plaintext]` pair.
- `CodingAdventures::CaesarCipher::ENGLISH_FREQUENCIES` — frozen hash of letter frequencies in English text, used by frequency analysis.
- RBS type signatures in `sig/coding_adventures/caesar_cipher.rbs`.
- Comprehensive Minitest test suite with 95%+ coverage across cipher operations and analysis.
- Literate programming style: every method includes extensive inline documentation with examples, diagrams, and mathematical explanations.
