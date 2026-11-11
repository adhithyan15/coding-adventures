# Changelog

All notable changes to `coding-adventures-lzss` are documented here.

## [Unreleased]

### Security

- No code change: block_count was already capped against the payload size and the output buffer is not preallocated.
- Added test_hostile_headers.py: hostile headers (0xFFFFFFFF counts/lengths, counts beyond the data) checked for bounded time and memory, plus round trips over every byte string of length <= 2 and a seeded random sample.

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS encoding and decoding (CMP02 spec).
- `Literal` and `Match` dataclasses as the two token types.
- `encode()` — sliding-window encoder using flag-bit token stream.
- `decode()` — byte-by-byte decoder with overlap-safe copy.
- `compress()` / `decompress()` — one-shot CMP02 wire-format API.
- CMP02 wire format: 8-byte header + flag-byte blocks (1 byte per literal,
  3 bytes per match, 1 flag byte per 8 symbols).
- Security: `block_count` from wire format capped against actual payload
  size to prevent DoS from crafted headers.
- 95%+ test coverage with spec test vectors, round-trip tests, and
  compression effectiveness tests.
