# huffman-compression

Pure Haskell implementation of CMP04, the canonical Huffman byte codec.

## API

- `compress` counts byte frequencies, delegates deterministic tree construction
  to the Haskell `huffman-tree` package, and emits the self-contained CMP04 wire
  format.
- `decompress` reconstructs canonical codes from the transmitted length table
  and decodes the LSB-first bit stream.

Both functions return `Either String ByteString`. The decoder validates header
lengths, symbol counts, table ordering, duplicate symbols, code-length bounds,
canonical oversubscription, invalid prefixes, and exhausted streams.

## Wire format

The payload contains an eight-byte big-endian header, a two-byte
`(symbol, code length)` entry for every distinct byte, and an LSB-first packed
bit stream. Code-length entries are sorted by `(length, symbol)` so the decoder
can reconstruct the same canonical codes without transmitting the tree.

## Running the tests

```sh
cabal test all
```
