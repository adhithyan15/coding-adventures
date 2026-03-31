# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- Core Atbash cipher implementation with `encrypt()` and `decrypt()` functions
- Full alphabet reversal: A<->Z, B<->Y, C<->X, etc.
- Case preservation: uppercase letters stay uppercase, lowercase stay lowercase
- Non-alphabetic character passthrough: digits, punctuation, whitespace unchanged
- Self-inverse property: encrypt(encrypt(text)) == text
- Type hints throughout (PEP 561 compliant with py.typed marker)
- Comprehensive test suite with 30+ test cases covering all edge cases
- Literate programming style with extensive inline documentation
