# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of LZSS compression (CMP02).
- `encode(_:...)` — sliding-window encoder; emits `.literal` or `.match` tokens (no `nextChar`).
- `decode(_:originalLength:)` — token decoder with byte-by-byte overlapping match copy.
- `compress(_:...)` — one-shot compress to CMP02 wire format.
- `decompress(_:)` — one-shot decompress from CMP02 wire format.
- `serialiseTokens(_:originalLength:)` — groups tokens into flag-byte blocks (8 tokens each).
- `deserialiseTokens(_:)` — parses CMP02 binary; caps `blockCount` to prevent DoS.
- `Token` enum with `.literal(UInt8)` and `.match(offset:UInt16, length:UInt8)` cases.
- 30+ XCTest tests covering spec vectors, round-trip, wire format, and compression effectiveness.
