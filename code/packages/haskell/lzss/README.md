# lzss

Pure Haskell implementation of CMP02, the LZSS sliding-window compression
algorithm.

## API

- `encode` and `encodeWith` produce `Literal` and `Match` tokens.
- `decode` and `decodeWithLength` reconstruct bytes with overlap-safe copying.
- `serialiseTokens` and `deserialiseTokens` implement CMP02 flag blocks: an
  eight-byte big-endian header followed by groups of up to eight symbols.
- `compress`, `compressWith`, and `decompress` provide one-shot byte APIs.

Operations that can encounter invalid parameters, tokens, lengths, or wire data
return `Either String`. The strict parser rejects invalid match fields,
impossible block counts, truncated records, non-zero unused flag bits, and
trailing data.

## Defaults

- Window size: 4096 bytes
- Maximum match: 255 bytes
- Minimum match: 3 bytes

The encoder uses deterministic earliest-position tie-breaking. Matches can
overlap their destination, so runs such as `AAAAAAA` become one literal followed
by `Match 1 6`.

## Running the tests

```sh
cabal test all
```
