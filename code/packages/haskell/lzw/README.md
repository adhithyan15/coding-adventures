# lzw

Pure Haskell implementation of CMP03, the LZW dictionary compression
algorithm.

## API

- `encode` turns strict bytes into logical LZW codes, including the leading
  `clearCode` and trailing `stopCode`.
- `decode` reconstructs bytes and handles the classic self-referential
  `code == nextCode` case.
- `packCodes` and `unpackCodes` implement the CMP03 wire format: a four-byte
  big-endian original length followed by variable-width, LSB-first codes.
- `compress` and `decompress` provide strict one-shot byte APIs.

Operations that can encounter invalid lengths, codes, or wire data return
`Either String`. The parser requires an initial CLEAR code, a STOP code, and
zero-only terminal padding. Decoded data must be at least as long as the stored
length before authoritative length trimming is applied.

## Format constants

- Single-byte codes: 0 through 255
- CLEAR code: 256
- STOP code: 257
- First dynamic code: 258
- Initial width: 9 bits
- Maximum width: 16 bits

The width grows after the first 512 code values are exhausted. A full 16-bit
dictionary causes the encoder to emit CLEAR and restart from the pre-seeded
dictionary.

## Running the tests

```sh
cabal test all
```
