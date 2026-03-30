# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-29

### Added

- Core Atbash cipher implementation with `encrypt()` and `decrypt()` functions
- Full alphabet reversal using Perl's efficient `tr///` operator
- Case preservation: uppercase letters stay uppercase, lowercase stay lowercase
- Non-alphabetic character passthrough: digits, punctuation, whitespace unchanged
- Self-inverse property: encrypt(encrypt($text)) eq $text
- Exporter support for importing encrypt/decrypt functions
- Comprehensive Test2::V0 test suite with 80+ assertions
- Literate programming style with extensive comments and POD documentation
