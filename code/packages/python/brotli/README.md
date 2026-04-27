# coding-adventures-brotli

**CMP06 — Brotli lossless compression algorithm (2013, RFC 7932)**

Part of the [coding-adventures](https://github.com/adhithya/coding-adventures)
monorepo compression series.

## What Is Brotli?

Brotli is a general-purpose lossless compression algorithm developed at Google
by Jyrki Alakuijärvi and Zoltán Szabadka. It became the dominant algorithm for
HTTP `Content-Encoding: br` (2015) and the WOFF2 font format (2014).

Compared to DEFLATE (CMP05), Brotli achieves better compression ratios through
three innovations:

1. **Context-dependent literal trees** — instead of one Huffman tree for all
   literals, each literal is assigned to one of 4 context buckets based on the
   preceding byte (space/punct=0, digit=1, uppercase=2, lowercase=3). Each
   bucket has its own Huffman tree, tuned to its specific context.

2. **Insert-and-copy commands** — instead of flat literal + back-reference
   tokens, Brotli uses commands that bundle an insert run (raw literals) with a
   copy operation. The lengths are encoded together as a single ICC Huffman
   symbol, reducing overhead.

3. **Larger sliding window** — 65535 bytes (CMP06) versus DEFLATE's 4096 bytes.

## Compression Series

```
CMP00 (LZ77,     1977) — Sliding-window backreferences.
CMP01 (LZ78,     1978) — Explicit dictionary (trie).
CMP02 (LZSS,     1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet; GIF.
CMP04 (Huffman,  1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE,  1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib.
CMP06 (Brotli,   2013) — Context modeling + insert-copy + large window. ← HERE
CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching; modern universal codec.
```

## Usage

```python
from coding_adventures_brotli import compress, decompress

# Compress any bytes object
data = b"Hello, world! " * 100
compressed = compress(data)
print(f"Original: {len(data)} bytes, Compressed: {len(compressed)} bytes")

# Decompress back to the original
original = decompress(compressed)
assert original == data
```

## Algorithm

### Compression

1. **Pass 1 — LZ matching**: Scan the input for matches of length ≥ 4 in the
   65535-byte sliding window. Build insert-and-copy commands.

2. **Pass 2a — Frequency counting**: Walk commands to tally symbol frequencies
   for 4 literal context trees, 1 ICC tree, and 1 distance tree.

3. **Pass 2b — Huffman construction**: Build canonical Huffman trees using
   `coding-adventures-huffman-tree` (DT27).

4. **Pass 2c — Encoding**: Encode each command's literals (per-context trees),
   then the ICC code + extra bits + distance code + extra bits. Pack LSB-first.

5. **Wire format**: 10-byte header + 6 code-length tables + bit stream.

### Context Buckets

```
Context function (last byte p1):
  bucket 0 — p1 is space/punctuation (start-of-stream or 0x00–0x2F etc.)
  bucket 1 — p1 is a digit ('0'–'9')
  bucket 2 — p1 is an uppercase letter ('A'–'Z')
  bucket 3 — p1 is a lowercase letter ('a'–'z')
```

### Wire Format

```
Header (10 bytes):
  [4B] original_length  — big-endian uint32
  [1B] icc_entry_count  — uint8 (1–64)
  [1B] dist_entry_count — uint8 (0–32)
  [1B] ctx0_entry_count — uint8 (0–256)
  [1B] ctx1_entry_count — uint8 (0–256)
  [1B] ctx2_entry_count — uint8 (0–256)
  [1B] ctx3_entry_count — uint8 (0–256)

ICC code-length table   — icc_entry_count  × 2 bytes (symbol uint8, len uint8)
Distance code-length    — dist_entry_count × 2 bytes (symbol uint8, len uint8)
Literal tree 0          — ctx0_count × 3 bytes (symbol BE uint16, len uint8)
Literal tree 1          — ctx1_count × 3 bytes
Literal tree 2          — ctx2_count × 3 bytes
Literal tree 3          — ctx3_count × 3 bytes
Bit stream              — LSB-first packed bits, zero-padded to byte boundary
```

## Dependencies

- `coding-adventures-huffman-tree` (DT27) — canonical Huffman tree builder

## Layer Position

```
DT04: heap            ← used by huffman-tree
DT27: huffman-tree    ← used by brotli
CMP06: brotli         ← [YOU ARE HERE]
```

## Development

```bash
uv venv .venv --no-project --clear
uv pip install --python .venv ../heap ../huffman-tree
uv pip install --python .venv -e .[dev]
.venv/bin/python -m pytest tests/ -v
```

## Performance

On typical English text (educational implementation, no static dictionary):

```
Original:         100,000 bytes
CMP05 (DEFLATE):   ~38,000 bytes   (62% reduction)
CMP06 (Brotli):    ~31,000 bytes   (69% reduction)
Real brotli:       ~28,000 bytes   (72% reduction, includes static dict)
```
