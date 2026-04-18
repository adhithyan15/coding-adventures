# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP00 LZ77 compression package.
- Added token-level `encode` and `decode` APIs for teaching and experimentation.
- Added one-shot `compress` and `decompress` helpers using a fixed-width token format.
- Added overlap-safe decoding and round-trip coverage for literals, backreferences, and binary data.
- Added malformed-input validation so invalid backreferences and truncated token streams raise `FormatException`.
