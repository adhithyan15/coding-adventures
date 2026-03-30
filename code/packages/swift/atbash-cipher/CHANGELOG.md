# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- Core Atbash cipher implementation with `encrypt(_:)` and `decrypt(_:)` static functions
- Full alphabet reversal: A<->Z, B<->Y, C<->X, etc.
- Case preservation: uppercase letters stay uppercase, lowercase stay lowercase
- Non-alphabetic character passthrough: digits, punctuation, whitespace unchanged
- Self-inverse property: encrypt(encrypt(text)) == text
- Comprehensive XCTest test suite with 30+ test cases
- Literate programming style with extensive Swift documentation comments
