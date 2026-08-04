# lz77

Pure Haskell implementation of CMP00, the foundational LZ77 sliding-window
compression algorithm.

## API

- `encode` and `encodeWith` turn a strict `ByteString` into literal and
  backreference `Token` values.
- `decode` and `decodeWithInitialBuffer` reconstruct bytes, including
  self-referential overlapping matches.
- `serialiseTokens` and `deserialiseTokens` implement the CMP00 teaching wire
  format: a big-endian token count followed by four bytes per token.
- `compress`, `compressWith`, and `decompress` provide one-shot byte APIs.

Operations that can encounter invalid parameters, malformed tokens, or bad
wire data return `Either String`. The decoder rejects impossible offsets,
zero-offset backreferences, non-zero literal offsets, truncated headers, and
truncated token streams.

## Defaults

- Window size: 4096 bytes
- Maximum match: 255 bytes
- Minimum match: 3 bytes

The encoder uses deterministic earliest-position tie-breaking and reserves one
byte after every match for `next_char`, as required by CMP00.

## Running the tests

```sh
cabal test all
```
