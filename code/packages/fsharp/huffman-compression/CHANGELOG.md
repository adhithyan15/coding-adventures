# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-26

### Added

- Pure F# CMP04 Huffman compression package backed by the existing Huffman tree package for canonical code generation
- Language-compatible CMP04 wire format with big-endian header fields, sorted code-length entries, and LSB-first bit packing
- xUnit coverage for round trips, exact wire-format vectors, edge cases, compression effectiveness, determinism, and truncated streams
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
