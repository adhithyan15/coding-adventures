# Changelog

## [Unreleased]

### Security

- No code change: declared length is bounded by maxOriginalLength and the stream is strictly validated. Not run locally.
- Added test/hostile_headers_test.dart: hostile headers must be rejected with FormatException (never allocate or loop from the claim), plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no Dart SDK in the review environment).

## 0.1.0

- Added the initial Dart implementation of the CMP02 LZSS compression package.
- Added token-level `encode` and `decode` APIs for literals and backreferences.
- Added one-shot `compress` and `decompress` helpers using the CMP02 block wire format.
- Added malformed-input validation for invalid backreferences, truncated blocks, and length tampering.
- Added strict declared-output validation so deserialisation fails closed on extra or missing data.
- Added a configurable decompressed-size cap so hostile headers cannot force unbounded output allocation.
