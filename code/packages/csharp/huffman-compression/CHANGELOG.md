# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-10

### Security

- SEC-003: `Decompress` now treats the code-length table as hostile input and
  throws `InvalidOperationException` for `symbol_count > 256`, duplicate
  symbols, code lengths outside 1..56, tables not sorted by length,
  over-subscribed (Kraft > 1) tables, and a declared `original_length` larger
  than 8 x payload bytes (checked before allocating the output). Bits are read
  in O(1) from the payload and decoding fails once the accumulated bits reach
  the longest code length. Valid encoder output still round-trips.

## [0.1.0] - 2026-04-26

### Added

- Pure C# CMP04 Huffman compression package backed by the existing Huffman tree package for canonical code generation
- Language-compatible CMP04 wire format with big-endian header fields, sorted code-length entries, and LSB-first bit packing
- xUnit coverage for round trips, exact wire-format vectors, edge cases, compression effectiveness, determinism, and truncated streams
- BUILD scripts that isolate `.NET` artifacts and first-run state for Linux and Windows CI
