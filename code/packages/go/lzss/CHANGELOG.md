# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Token` struct with `KindLiteral` and `KindMatch` kinds.
- `Encode` / `Decode` token-level API.
- `Compress` / `Decompress` one-shot CMP02 wire-format API.
- Flag-byte block serialisation: 1 byte per literal, 3 bytes per match.
- Security: `block_count` capped against payload size to prevent DoS.
