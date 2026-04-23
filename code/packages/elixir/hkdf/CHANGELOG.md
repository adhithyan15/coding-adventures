# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial implementation of HKDF (RFC 5869) in Elixir.
- `extract(salt, ikm, hash)` — HKDF-Extract stage.
- `expand(prk, info, length, hash)` — HKDF-Expand stage.
- `hkdf(salt, ikm, info, length, hash)` — combined extract-and-expand.
- Hex convenience variants: `extract_hex`, `expand_hex`, `hkdf_hex`.
- Support for SHA-256 and SHA-512 hash algorithms.
- Empty salt defaults to HashLen zero bytes per RFC 5869.
- Error handling for invalid output lengths (L <= 0 or L > 255 * HashLen).
- Full RFC 5869 test vectors (Test Cases 1, 2, 3) plus edge case tests.
