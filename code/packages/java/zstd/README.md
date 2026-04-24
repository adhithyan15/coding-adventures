# java/zstd — CMP07

Pure-Java implementation of the Zstandard (ZStd) lossless compression algorithm
(RFC 8878). This is package CMP07 in the compression series.

## What it does

ZStd combines:

- **LZ77 back-references** (via the `lzss` package) to exploit repetition in the
  data — the same "copy from earlier in the output" trick as DEFLATE, but with a
  32 KB window.
- **FSE (Finite State Entropy)** coding instead of Huffman for the sequence
  descriptor symbols. FSE is an asymmetric numeral system that approaches the
  Shannon entropy limit in a single pass.
- **Predefined decode tables** (RFC 8878 Appendix B) so short frames need no
  table description overhead.

## Where it fits

```
CMP00 (LZ77)    — Sliding-window back-references
CMP01 (LZ78)    — Explicit dictionary (trie)
CMP02 (LZSS)    — LZ77 + flag bits              ← dependency
CMP03 (LZW)     — LZ78 + pre-init alphabet; GIF
CMP04 (Huffman) — Entropy coding
CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)  — DEFLATE + context modelling + static dict
CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package
```

## Usage

```java
import com.codingadventures.zstd.Zstd;

byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();

// Compress
byte[] compressed = Zstd.compress(data);

// Decompress
byte[] restored = Zstd.decompress(compressed);

assert java.util.Arrays.equals(data, restored);
```

## Dependencies

- `com.codingadventures:lzss` — LZSS LZ77 tokeniser (CMP02)

## Frame format

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE              │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Each block has a 3-byte header:

- bit 0 = Last_Block flag
- bits [2:1] = Block_Type (00=Raw, 01=RLE, 10=Compressed)
- bits [23:3] = Block_Size

## Building and testing

```bash
cd code/packages/java/zstd
gradle test
```
