# lz78

A pure Haskell implementation of the CMP01 LZ78 lossless compression
algorithm. The encoder builds an explicit dictionary of byte sequences and
emits `(dictionary index, next byte)` tokens; the decoder reconstructs the same
dictionary without receiving it on the wire.

## API

- `Token` is the fixed-width LZ78 token model.
- `emptyCursor`, `stepCursor`, `insertCursor`, and `resetCursor` expose the
  immutable trie cursor used by the encoder.
- `encode` and `decode` operate on token streams.
- `serialiseTokens` and `deserialiseTokens` implement the CMP01 big-endian
  header and four-byte token records.
- `compress`, `compressDefault`, and `decompress` provide the one-shot API.

Checked decoding returns `Either Lz78Error ByteString`, rejecting malformed
headers, truncated or overlong streams, non-zero reserved bytes, invalid
dictionary references, and decoded-length mismatches. Encoding and
serialization are deterministic and use only pure data transformations.

## Running the tests

```sh
cabal test all
```
