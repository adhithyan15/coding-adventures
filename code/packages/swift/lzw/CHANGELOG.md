# Changelog — LZW (Swift)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-11

### Added

- Initial Swift implementation of LZW (CMP03) lossless compression.
- `compress(_ data: [UInt8]) -> [UInt8]` — one-shot compression to CMP03 wire format.
- `decompress(_ data: [UInt8]) -> [UInt8]` — one-shot decompression from CMP03 wire format.
- `encodeCodes(_ data: [UInt8]) -> ([UInt], Int)` — encode input to LZW code sequence.
- `decodeCodes(_ codes: [UInt]) -> [UInt8]` — decode LZW code sequence to bytes.
- `packCodes(_ codes: [UInt], originalLength: Int) -> [UInt8]` — pack codes to wire bytes.
- `unpackCodes(_ data: [UInt8]) -> ([UInt], Int)` — unpack wire bytes to codes.
- Private `BitWriter` struct for LSB-first variable-width bit packing.
- Private `BitReader` struct for LSB-first variable-width bit unpacking.
- All 256 single-byte entries pre-seeded in the encoder/decoder dictionary (LZW invariant).
- Variable-width codes: start at 9 bits, grow to 16 bits maximum.
- CLEAR_CODE (256) emitted at stream start and on dictionary overflow.
- STOP_CODE (257) emitted at stream end.
- Tricky-token (SC == NC) edge case handled in decoder.
- 4-byte big-endian `original_length` header in wire format.
- 38-test XCTest suite covering spec vectors, round-trips, tricky token, wire
  format, compression effectiveness, and malformed-input safety.
- Literate inline comments explaining algorithm, edge cases, and bit I/O.
- `Package.swift` (swift-tools-version 5.9), `BUILD`, `BUILD_windows`, `.gitignore`.
