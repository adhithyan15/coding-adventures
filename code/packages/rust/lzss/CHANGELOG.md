# Changelog

## [Unreleased]

### Added

- Add required_capabilities.json (pure computation).

### Security

- CWE-789: `decode` no longer uses the untrusted `original_length` header (up to 4 GiB) as the output capacity; the hint is capped at 4 bytes per token and the buffer grows on demand. Decoding stops once the declared length is reached.
- Added `tests/hostile_headers.rs` (allocation guard, hostile headers, exhaustive <=2-byte and seeded random round trips).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Token` enum with `Literal` and `Match` variants.
- `encode` / `decode` token-level API.
- `compress` / `decompress` one-shot CMP02 wire-format API.
- Security: `block_count` capped against payload size to prevent DoS.
