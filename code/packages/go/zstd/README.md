# go/zstd — CMP07

Zstandard (ZStd) lossless compression and decompression in pure Go, implementing
[RFC 8878](https://www.rfc-editor.org/rfc/rfc8878).

ZStd was created by Yann Collet at Facebook (2015). It beats DEFLATE on both
compression ratio and speed by combining LZ77 back-references with FSE
(Finite State Entropy) instead of Huffman coding for the sequence symbols.

## Position in the series

```
CMP00 (LZ77)     — Sliding-window back-references
CMP01 (LZ78)     — Explicit dictionary (trie)
CMP02 (LZSS)     — LZ77 + flag bits
CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman)  — Entropy coding
CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)   — DEFLATE + context modelling + static dict
CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package
```

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/zstd"

// Compress
data := []byte("the quick brown fox jumps over the lazy dog")
compressed := zstd.Compress(data)

// Decompress
original, err := zstd.Decompress(compressed)
if err != nil {
    log.Fatal(err)
}
```

## Frame layout

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE uint64        │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Each **block** carries a 3-byte little-endian header:

```
bit 0       = Last_Block flag
bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
bits [23:3] = Block_Size
```

## Compression strategy

1. Split input into 128 KB blocks (`maxBlockSize`).
2. For each block, try in order:
   - **RLE** — if all bytes are identical, emit 1 byte payload.
   - **Compressed** — run LZSS (32 KB window) to find LZ77 matches, encode
     sequences with predefined FSE tables. Accept only if smaller than raw.
   - **Raw** — verbatim fallback.

## FSE (Finite State Entropy)

FSE encodes the per-sequence symbols (literal length code, match length code,
offset code) using the predefined tables from RFC 8878 Appendix B:

| Stream      | Table size | Accuracy log |
|-------------|-----------|--------------|
| Literal Len | 64 slots  | 6            |
| Match Len   | 64 slots  | 6            |
| Offset      | 32 slots  | 5            |

The sequence bitstream is written **backwards**: the encoder writes the last
sequence first so the decoder, reading forward, sees sequences in natural order.
A sentinel bit in the last byte marks the boundary of valid data.

## Security

- Output is capped at **256 MB** (`maxOutput`) to prevent decompression bombs.
- All index arithmetic is bounds-checked before table lookups.
- Truncated or malformed input returns descriptive errors rather than panicking.

## Dependencies

- [`go/lzss`](../lzss) — provides `Encode(data, windowSize, maxMatch, minMatch)`
  for LZ77 match finding.

## Running tests

```bash
go test ./... -v -cover
```

Expected: 51 tests pass, coverage ≥ 93%.
