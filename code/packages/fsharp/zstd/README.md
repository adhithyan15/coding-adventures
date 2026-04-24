# CodingAdventures.Zstd.FSharp

Pure F# implementation of **Zstandard (ZStd)** lossless compression — CMP07 in
the coding-adventures series.

ZStd (RFC 8878) was created by Yann Collet at Facebook in 2015. It achieves
high compression ratios at high speed by combining:

- **LZ77 back-references** — the same sliding-window approach as DEFLATE, with
  a 32 KB window for better matches.
- **FSE (Finite State Entropy)** coding — an asymmetric numeral system that
  approaches the Shannon entropy limit in a single pass, replacing Huffman
  coding for sequence descriptors.
- **Predefined tables** — RFC 8878 Appendix B specifies fixed FSE distributions
  so short frames need zero table-description overhead.

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

```fsharp
open CodingAdventures.Zstd.FSharp

let data = System.Text.Encoding.UTF8.GetBytes("hello, ZStd world!")
let compressed   = Zstd.Compress(data)
let decompressed = Zstd.Decompress(compressed)
// decompressed = data
```

## Frame layout

```
┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
│ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
└────────┴─────┴──────────────────────┴────────┴──────────────────┘
```

Each block has a 3-byte header:

```
bit 0      = Last_Block flag
bits [2:1] = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
bits [23:3] = Block_Size
```

## Block selection strategy

For each 128 KB chunk of input, the encoder tries in order:

1. **RLE** — if all bytes are identical, store as 4 bytes total (header + 1 byte).
2. **Compressed** — LZ77 tokens via LZSS → FSE-coded sequence bitstream. Only
   used when the output is strictly smaller than the input.
3. **Raw** — verbatim fallback when neither RLE nor compression helps.

## FSE backward bit stream

The sequence bitstream is written *backwards*: the encoder writes sequences in
reverse order so the decoder can consume a forward-only stream. The stream ends
with a sentinel bit (the highest set bit in the last byte) that the decoder
uses to locate the start of valid data.

## Dependencies

- [`CodingAdventures.Lzss.FSharp`](../lzss/) — LZ77 token generation

## Limitations

- Only **Predefined FSE modes** are supported (modes byte must be `0x00`).
- Only **Raw_Literals** (type=0) are produced and accepted.
- No dictionary support.
- Decompression output is capped at 256 MB as a bomb guard.

## Running tests

```bash
cd code/packages/fsharp/zstd
bash BUILD
```

Or directly:

```bash
dotnet test tests/CodingAdventures.Zstd.Tests/CodingAdventures.Zstd.Tests.fsproj
```
