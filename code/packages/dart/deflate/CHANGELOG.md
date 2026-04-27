# Changelog

## [0.1.0] - 2026-04-18

### Added

- Added the Dart CMP05 DEFLATE implementation built on the existing LZSS and Huffman tree packages.
- Added LL and distance code helpers, LSB-first bit packing, and strict decoder validation.
- Added round-trip and malformed-input tests covering literals, matches, and padding behavior.
