# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP02 LZSS compression package.
- Added token-level `encode` and `decode` APIs for literals and backreferences.
- Added one-shot `compress` and `decompress` helpers using the CMP02 block wire format.
- Added malformed-input validation for invalid backreferences, truncated blocks, and length tampering.
- Added strict declared-output validation so deserialisation fails closed on extra or missing data.
- Added a configurable decompressed-size cap so hostile headers cannot force unbounded output allocation.
