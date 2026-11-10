# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Security

- Hardening (CWE-789): the declared length is clamped instead of raw-cast; DecodeCodes stops once the declared length is reached. Not run locally.
- Added HostileHeadersTests (xunit): hostile headers with a wall-clock bound, plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no dotnet SDK in the review environment); written to the existing test project conventions.

## [0.1.0] - 2026-04-18

### Added

- Pure C# CMP03 LZW implementation with pre-seeded dictionary encoding and decoding
- Public `BitWriter` and `BitReader` helpers for LSB-first variable-width code streams
- Code-level `EncodeCodes`, `DecodeCodes`, `PackCodes`, and `UnpackCodes` helpers
- Byte-level `Compress` and `Decompress` helpers with big-endian original-length header
- xUnit coverage for spec vectors, tricky-token decoding, bit-packing symmetry, and binary round trips
