# CodingAdventures.Zstd — C# ZStd Compression (CMP07)

Pure C# implementation of Zstandard (ZStd) lossless compression, part of the
CMP07 learning series.

## What is ZStd?

Zstandard (RFC 8878) is a high-ratio, fast compression format created by Yann
Collet at Facebook (2015). It combines:

- **LZ77 back-references** — "copy from earlier in the output" to exploit
  repetition (same idea as DEFLATE, but with a larger window and faster match
  finding).
- **FSE (Finite State Entropy)** — instead of Huffman trees, ZStd uses an
  asymmetric numeral system that approaches the Shannon entropy limit in a
  single pass over the data.
- **Predefined tables** (RFC 8878 Appendix B) — no per-frame table description
  overhead for short frames.

## Where it fits

```
CMP00 (LZ77)    — Sliding-window back-references
CMP01 (LZ78)    — Explicit dictionary (trie)
CMP02 (LZSS)    — LZ77 + flag bits           ← this package depends on LZSS
CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman) — Entropy coding
CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)  — DEFLATE + context modelling + static dict
CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package
```

## API

```csharp
using CodingAdventures.Zstd;

byte[] data = System.Text.Encoding.UTF8.GetBytes("the quick brown fox ".repeat(50));

// Compress to a valid ZStd frame (decompressable by the `zstd` CLI tool).
byte[] compressed = Zstd.Compress(data);

// Decompress back to the original bytes.
byte[] original = Zstd.Decompress(compressed);
```

## Frame layout

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE              │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Each block:
```
bits [0]    = Last_Block flag
bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed)
bits [23:3] = Block_Size
```

## Compression strategy

1. Split data into 128 KB blocks.
2. For each block:
   - **RLE** if all bytes are identical (4 bytes total output).
   - **Compressed** (LZ77 + FSE) if the result is smaller than the input.
   - **Raw** as a fallback.

## Limitations

- Only **Predefined FSE mode** (RFC 8878 Appendix B) is supported for
  sequence descriptors. Frames using per-frame FSE or Huffman-coded literals
  (written by the `zstd` CLI at higher compression levels) will be rejected.
- No dictionary support.
- No content checksum generation/validation.

## Running tests

```bash
cd code/packages/csharp/zstd
mkdir -p .dotnet .artifacts
HOME="$PWD/.dotnet" DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1 DOTNET_CLI_HOME="$PWD/.dotnet" \
  dotnet test tests/CodingAdventures.Zstd.Tests/CodingAdventures.Zstd.Tests.csproj \
  --disable-build-servers --artifacts-path .artifacts
```
