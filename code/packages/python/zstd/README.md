# coding-adventures-zstd

Zstandard (ZStd) lossless compression algorithm (RFC 8878, CMP07) implemented
from scratch in Python. Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
compression series.

## What is ZStd?

Zstandard is a high-ratio, fast compression format created by Yann Collet at
Facebook (2015). It combines:

- **LZ77 back-references** via a 32 KB sliding window (using the `coding-adventures-lzss` package)
- **FSE (Finite State Entropy)** coding for sequence descriptors — an asymmetric numeral system approaching Shannon entropy
- **Predefined decode tables** (RFC 8878 Appendix B) so frames need no table description overhead

## How It Fits in the Stack

```
coding-adventures-lzss   ← LZ77/LZSS match-finding (direct dependency)
         ↑
coding-adventures-zstd   ← this package
```

ZStd uses `lzss.encode()` for LZ77 tokenisation, then applies FSE entropy coding
to the resulting sequence descriptors. The output is a valid ZStd frame compatible
with the reference `zstd` CLI and any RFC 8878 implementation.

## Usage

```python
from coding_adventures_zstd import compress, decompress

# Compress
data = b"the quick brown fox jumps over the lazy dog " * 100
compressed = compress(data)
print(f"Compressed {len(data)} bytes to {len(compressed)} bytes")

# Decompress
original = decompress(compressed)
assert original == data
```

## Algorithm Overview

### Frame Format (RFC 8878 §3)

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE (u64)        │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

### Block Types

Each 128 KB block is attempted in order:
1. **RLE** — all bytes identical → 4 bytes total (3-byte header + 1 byte value)
2. **Compressed** — LZ77 + FSE → smaller than raw
3. **Raw** — verbatim fallback

### Compressed Block Internals

```
[Literals section]   Raw bytes that precede each back-reference
[Sequence count]     Variable-length count (1–3 bytes)
[Modes byte: 0x00]  Predefined FSE tables
[FSE bitstream]      Backward-written sequence symbols + extra bits
```

### FSE (Finite State Entropy)

FSE is the entropy coder at the heart of ZStd. It uses predefined probability
distributions for three symbol streams:
- **LL** (Literal Length): how many literal bytes precede each match
- **ML** (Match Length): how long each back-reference match is
- **OF** (Offset): how far back the match starts

The "Predefined" mode means decoder and encoder both use the same fixed
distributions from RFC 8878 Appendix B — no per-frame table transmission needed.

## Compression Series

```
CMP00 (LZ77,    1977) — Sliding-window back-references
CMP01 (LZ78,    1978) — Explicit trie dictionary
CMP02 (LZSS,    1982) — LZ77 + flag bits
CMP03 (LZW,     1984) — LZ78 + pre-initialised alphabet; GIF
CMP04 (Huffman, 1952) — Entropy coding
CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
CMP06 (Brotli)        — DEFLATE + context modelling + static dict
CMP07 (ZStd)          — LZ77 + FSE; this package
```

## Development

```bash
# Install with dev dependencies
uv pip install -e .[dev]

# Run tests
python -m pytest tests/ -v

# Lint
ruff check src/ tests/
```

## References

- [RFC 8878 — Zstandard Compression](https://datatracker.ietf.org/doc/html/rfc8878)
- Collet, Y. (2015). Zstandard — Real-time data compression algorithm. Facebook.
