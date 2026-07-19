# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-18

### Added

- Pure C# CMP07 Zstandard educational codec with raw, RLE, and compressed blocks
- Raw literal sections and predefined FSE tables for literal-length, match-length, and offset codes
- One-shot `Compress` and `Decompress` helpers backed by the native C# LZSS package
- xUnit coverage for multi-block frames, header variants, malformed input, compression ratios, and deterministic output
