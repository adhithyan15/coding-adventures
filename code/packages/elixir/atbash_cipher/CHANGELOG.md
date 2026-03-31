# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- Core Atbash cipher implementation with `encrypt/1` and `decrypt/1` functions
- Full alphabet reversal: A<->Z, B<->Y, C<->X, etc.
- Case preservation: uppercase letters stay uppercase, lowercase stay lowercase
- Non-alphabetic character passthrough: digits, punctuation, whitespace unchanged
- Self-inverse property: encrypt(encrypt(text)) == text
- Comprehensive ExUnit test suite with 30+ test cases and doctests
- Literate programming style with extensive @moduledoc and @doc strings
