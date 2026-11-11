# Changelog

## [Unreleased]

### Security

- No code change: header token count must match the data length exactly and tokens are validated. Not run locally.
- Added test/hostile_headers_test.dart: hostile headers must be rejected with FormatException (never allocate or loop from the claim), plus round trips over every byte string of length <= 2 and a seeded random sample. NOT RUN locally (no Dart SDK in the review environment).

## 0.1.0

- Added the initial Dart implementation of the CMP01 LZ78 compression package.
- Added a trie-based dictionary cursor plus token-level `encode` and `decode` APIs.
- Added one-shot `compress` and `decompress` helpers using the CMP01 wire format.
- Added malformed-input validation for invalid dictionary indexes and truncated token streams.
- Added strict wire-length validation so incomplete headers and trailing bytes are rejected.
- Added declared-output-length validation so tampered streams cannot underflow or overflow the expected decode size.
