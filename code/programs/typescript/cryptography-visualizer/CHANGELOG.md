# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- Initial scaffold for the cryptography visualizer web app
- Caesar cipher integration via `@coding-adventures/caesar-cipher` package
- Atbash cipher integration via `@coding-adventures/atbash-cipher` package
- Substitution table showing the full A-Z cipher mapping
- Step-by-step panel showing each character transformation
- Frequency analysis chart comparing ciphertext vs English letter distributions
- Brute force panel listing all 25 possible Caesar decryptions
- ROT13 quick-action button for the Caesar cipher
- Shift slider (1-25) for Caesar cipher key selection
- Cipher selector dropdown (Caesar, Atbash)
- Lattice-authored warm paper aesthetic styles
- Tests covering default render, cipher switching, shift changes, and edge cases
