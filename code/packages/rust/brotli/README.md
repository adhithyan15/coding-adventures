# brotli (Rust)

**CMP06 — Brotli lossless compression (2013)**

Brotli is Google's lossless compression algorithm (RFC 7932), used for HTTP
`Content-Encoding: br` and WOFF2 fonts. It builds on DEFLATE's foundation
with three major improvements:

1. **Context-dependent literal trees** — four Huffman trees, one per literal
   context bucket (space/punct, digit, uppercase, lowercase), tuned to the
   actual byte distribution in each context.

2. **Insert-and-copy commands (ICC)** — commands bundle a literal-insert run
   with a copy back-reference into a single Huffman symbol, cutting overhead
   compared to DEFLATE's separate literal and match tokens.

3. **Larger sliding window** — 65535 bytes (vs DEFLATE's 4096), allowing
   matches across much longer distances.

This is the CodingAdventures educational implementation (CMP06). The RFC 7932
static dictionary (122,784 entries) is intentionally omitted to keep the
implementation tractable and cross-language consistent.

## Usage

```rust
use brotli::{compress, decompress};

let data = b"the quick brown fox jumps over the lazy dog";
let compressed = compress(data);
let original = decompress(&compressed).unwrap();
assert_eq!(original, data);
```

## Wire Format

```
Header (10 bytes):
  [4B] original_length    — big-endian uint32
  [1B] icc_entry_count    — entries in ICC code-length table (1–64)
  [1B] dist_entry_count   — entries in dist code-length table (0–32)
  [1B] ctx0_entry_count   — entries in literal tree 0
  [1B] ctx1_entry_count   — entries in literal tree 1
  [1B] ctx2_entry_count   — entries in literal tree 2
  [1B] ctx3_entry_count   — entries in literal tree 3

ICC code-length table (icc_entry_count × 2 bytes):
  [1B] symbol (0–63)  [1B] code_length (1–16)
  Sorted by (code_length ASC, symbol ASC)

Distance code-length table (dist_entry_count × 2 bytes):
  [1B] symbol (0–31)  [1B] code_length (1–16)
  Sorted by (code_length ASC, symbol ASC)

Literal trees 0–3 (ctx_entry_count × 3 bytes each):
  [2B BE] symbol (0–255)  [1B] code_length (1–16)
  Sorted by (code_length ASC, symbol ASC)

Bit stream (remaining bytes):
  LSB-first packed bits, zero-padded to byte boundary.
```

## Series

```
CMP00 (LZ77,    1977) — Sliding-window backreferences.
CMP01 (LZ78,    1978) — Explicit dictionary (trie).
CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF.
CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
CMP05 (DEFLATE, 1996) — LZSS + dual Huffman; ZIP/gzip/PNG/zlib.
CMP06 (Brotli,  2013) — Context modeling + insert-copy + large window. ← this crate
CMP07 (Zstd,    2016) — ANS/FSE + LZ4 matching; modern universal codec.
```
