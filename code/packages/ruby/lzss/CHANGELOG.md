# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Literal` and `Match` structs as the two token types.
- `encode` / `decode` token-level API.
- `compress` / `decompress` one-shot CMP02 wire-format API.
- Security: `block_count` capped against payload size to prevent DoS.
