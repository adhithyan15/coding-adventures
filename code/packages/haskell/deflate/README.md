# deflate (Haskell)

Pure Haskell implementation of the repository's CMP05 educational DEFLATE
format. It composes the native Haskell `lzss` tokeniser with the native
`huffman-tree` canonical-code implementation; it does not call zlib or another
compression library.

> CMP05 is a self-describing teaching format, not an RFC 1951 wire stream.

## API

- `compress` uses the standard 4,096-byte window, 255-byte maximum match, and
  three-byte minimum match.
- `compressWith` accepts explicit safe CMP05/LZSS parameters.
- `decompress` validates headers, alphabet bounds, canonical tables, bit
  prefixes, backreferences, and the declared output length.

All functions operate on strict `ByteString` values. Compression and
decompression return `Either String ByteString`, so malformed parameters and
payloads never require exceptions.

## Wire format

The stream stores a big-endian `u32` original length, big-endian `u16`
literal/length and distance table counts, sorted `(u16 symbol, u8 code length)`
entries, then an LSB-packed canonical Huffman bit stream. Length and distance
extra fields are also written least-significant bit first.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```
