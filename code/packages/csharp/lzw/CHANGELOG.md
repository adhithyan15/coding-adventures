# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure C# CMP03 LZW implementation with pre-seeded dictionary encoding and decoding
- Public `BitWriter` and `BitReader` helpers for LSB-first variable-width code streams
- Code-level `EncodeCodes`, `DecodeCodes`, `PackCodes`, and `UnpackCodes` helpers
- Byte-level `Compress` and `Decompress` helpers with big-endian original-length header
- xUnit coverage for spec vectors, tricky-token decoding, bit-packing symmetry, and binary round trips
