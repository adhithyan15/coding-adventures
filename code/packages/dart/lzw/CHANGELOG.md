# Changelog

## 0.1.0

- Added the initial Dart implementation of the CMP03 LZW compression package.
- Added logical-code `encodeCodes` and `decodeCodes` APIs with tricky-token support.
- Added packed-wire `packCodes` and `unpackCodes` helpers for LSB-first CMP03 bit streams.
- Added one-shot `compress` and `decompress` helpers plus strict malformed-input validation.
- Added a configurable decompressed-size cap so hostile headers cannot force unbounded output allocation.
