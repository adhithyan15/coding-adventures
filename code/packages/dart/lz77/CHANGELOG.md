# Changelog

## [Unreleased]

### Security

- No code change: the loop over the header count throws FormatException at the first truncated token, so it is bounded by the data; tokens are validated before decode. Not run locally.
- Added test/hostile_headers_test.dart: hostile headers must be rejected with FormatException (never allocate or loop from the claim), plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no Dart SDK in the review environment).

## 0.1.0

- Added the initial Dart implementation of the CMP00 LZ77 compression package.
- Added token-level `encode` and `decode` APIs for teaching and experimentation.
- Added one-shot `compress` and `decompress` helpers using a fixed-width token format.
- Added overlap-safe decoding and round-trip coverage for literals, backreferences, and binary data.
- Added malformed-input validation so invalid backreferences and truncated token streams raise `FormatException`.
