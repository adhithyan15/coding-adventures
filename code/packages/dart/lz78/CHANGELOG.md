# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP01 LZ78 compression package.
- Added a trie-based dictionary cursor plus token-level `encode` and `decode` APIs.
- Added one-shot `compress` and `decompress` helpers using the CMP01 wire format.
- Added malformed-input validation for invalid dictionary indexes and truncated token streams.
- Added strict wire-length validation so incomplete headers and trailing bytes are rejected.
- Added declared-output-length validation so tampered streams cannot underflow or overflow the expected decode size.
