# Changelog

All notable changes to this crate will be documented in this file.

## Unreleased

### Added

- Added data-driven, fixed-memory `hkdf_extract_fixed` support for repository
  HMAC-SHA-256 and HMAC-SHA-384, returning a metadata-only-debuggable,
  wipe-on-drop PRK owner without changing the allocation-backed compatibility
  APIs.

## [0.1.0] - 2026-04-12

### Added

- HKDF-Extract: concentrate entropy from IKM into a pseudorandom key using HMAC
- HKDF-Expand: derive output keying material from PRK using chained HMAC blocks
- Combined HKDF (extract-then-expand) convenience function
- `HashAlgorithm` enum for SHA-256 and SHA-512 selection
- `HkdfError` type for invalid output length errors
- Empty salt handling (uses HashLen zero bytes per RFC 5869)
- All three RFC 5869 Appendix A test vectors (SHA-256)
- Edge case tests for boundary lengths, error conditions, and domain separation
- Literate programming style with extensive inline documentation
