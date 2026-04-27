# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure F# CMP06 Brotli-style implementation with insert-copy commands, literal contexts, and long-distance matches
- Canonical Huffman table emission for ICC, distance, and per-context literal alphabets
- One-shot `Compress` and `Decompress` helpers using the repo's teaching wire format
- xUnit coverage for spec vectors, deterministic output, manual wire payloads, long-distance copies, and compression sanity checks
