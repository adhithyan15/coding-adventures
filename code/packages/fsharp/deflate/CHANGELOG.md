# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-18

### Added

- Pure F# CMP05 DEFLATE implementation built from the native `.NET` `lzss` and `huffman-tree` packages
- Combined literal/length and distance canonical Huffman coding with LSB-first bit packing
- CMP05 header and code-length table encoding plus matching decompression support
- xUnit coverage for empty input, literal-only streams, match-bearing streams, binary round trips, and compression sanity checks
