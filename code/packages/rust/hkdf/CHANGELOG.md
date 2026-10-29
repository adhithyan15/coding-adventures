# Changelog

All notable changes to this crate will be documented in this file.

## Unreleased

### Added

- Added an exact-length import boundary for already-derived SHA-256 and
  SHA-384 pseudorandom keys. It validates HashLen before copying, returns the
  existing fixed wipe-on-drop `HkdfPrk` owner, keeps errors payload-free, and
  explicitly does not substitute for HKDF-Extract or establish input
  provenance.
- Added data-driven, fixed-memory `hkdf_expand_fixed` support for repository
  HMAC-SHA-256 and HMAC-SHA-384. It binds algorithm choice to `HkdfPrk`, streams
  borrowed HMAC segments without an aggregate message allocation, retains one
  fixed wipe-on-drop previous block, wipes each tag in its original HMAC result
  slot, and writes into caller-provided output storage. An optimized-IR gate
  rejects allocator calls or lost result-slot and previous-block scrubs.
- Added data-driven, fixed-memory `hkdf_extract_fixed` support for repository
  HMAC-SHA-256 and HMAC-SHA-384, returning a metadata-only-debuggable,
  wipe-on-drop PRK owner without changing the allocation-backed compatibility
  APIs.

### Changed

- Hardened fixed PRK construction so Extract and exact import copy into the
  caller-owned result while retaining and wiping their original 48-byte
  staging storage. The optimized-IR gate now rejects missing PRK-stage scrubs
  and allocator calls across Extract, import, and Expand.

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
