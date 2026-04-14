# @coding-adventures/brotli

**CMP06** — Brotli lossless compression (2013, RFC 7932), implemented from scratch in TypeScript for educational purposes.

Brotli is Google's general-purpose lossless compression algorithm, now the standard for HTTP `Content-Encoding: br` responses and WOFF2 font files. It improves on DEFLATE (CMP05) in three fundamental ways:

1. **Context-dependent literal trees** — instead of one Huffman tree for all literal bytes, Brotli uses 4 separate trees keyed by the *context* of the preceding byte. After a lowercase letter, the next byte is almost certainly another letter or space; after a digit, it's usually another digit. Separate trees per context are each finely calibrated to their specific byte distribution.

2. **Insert-and-copy commands (ICC)** — DEFLATE encodes literals and back-references as separate tokens. Brotli bundles the insert-length and copy-length into a single ICC Huffman symbol, saving overhead especially in text with many short matches.

3. **Larger sliding window** — 65,535 bytes (vs DEFLATE's 4,096), letting the compressor match repeated content from much further back.

This is part of the **CodingAdventures compression series**:

```
CMP00 (LZ77,     1977) — Sliding-window back-references
CMP01 (LZ78,     1978) — Explicit dictionary (trie)
CMP02 (LZSS,     1982) — LZ77 + flag bits
CMP03 (LZW,      1984) — LZ78 + pre-initialised alphabet
CMP04 (Huffman,  1952) — Entropy coding
CMP05 (DEFLATE,  1996) — LZSS + dual Huffman trees
CMP06 (Brotli,   2013) — Context modeling + ICC + large window  ← YOU ARE HERE
CMP07 (Zstd,     2016) — ANS/FSE + LZ4 matching
```

## Algorithm

### Pass 1: LZ Matching

Scan the input from left to right. At each position, search backward through the 65,535-byte sliding window for the longest repeated sequence (minimum 4 bytes, maximum 769 bytes). When a match is found, record `{ insertLength, copyLength, copyDistance, literals }` as a command. Remaining bytes with no match accumulate in the insert buffer.

### Pass 2: Frequency Tallying + Huffman Tree Building

Walk the commands to count:
- **Literal frequencies per context bucket** — which byte followed which category of byte?
- **ICC code frequencies** — how often was each (insertLen, copyLen) pair used?
- **Distance code frequencies** — how often was each distance range used?

Build one canonical Huffman tree (via DT27) per frequency table.

### Pass 3: Bit-Stream Encoding

Re-walk the commands, emitting:
- Each literal byte using its context bucket's Huffman tree
- Each ICC Huffman symbol + insert extra bits + copy extra bits
- Each distance code Huffman symbol + distance extra bits

Pack bits LSB-first and write the wire format.

## Context Modeling Detail

```
literalContext(lastByte):
  if lastByte in 'a'..'z':  return 3  (lowercase)
  if lastByte in 'A'..'Z':  return 2  (uppercase)
  if lastByte in '0'..'9':  return 1  (digit)
  else:                      return 0  (space/punct/other)
```

At stream start (no previous byte), bucket 0 is used.

## Wire Format

```
Header (10 bytes):
  [4B] original_length     big-endian uint32
  [1B] icc_entry_count     uint8
  [1B] dist_entry_count    uint8
  [1B] ctx0_entry_count    uint8
  [1B] ctx1_entry_count    uint8
  [1B] ctx2_entry_count    uint8
  [1B] ctx3_entry_count    uint8

ICC code-length table:   icc_entry_count  × (1B symbol, 1B code_length)
Distance code table:     dist_entry_count × (1B symbol, 1B code_length)
Literal tree 0 table:    ctx0_entry_count × (2B symbol BE, 1B code_length)
Literal tree 1–3 tables: same format
Bit stream:              LSB-first packed, zero-padded to byte boundary
```

Entries within each table are sorted `(code_length ASC, symbol ASC)` — the canonical Huffman ordering.

## Usage

```typescript
import { compress, decompress } from "@coding-adventures/brotli";

const original = new TextEncoder().encode("Hello, Brotli!");
const compressed = compress(original);
const restored = decompress(compressed);

console.log(new TextDecoder().decode(restored)); // "Hello, Brotli!"
console.log(`${original.length} → ${compressed.length} bytes`);
```

## API

### `compress(data: Uint8Array): Uint8Array`

Compress `data` using the CMP06 Brotli algorithm. Returns the compressed bytes in CMP06 wire format. For empty input, returns the canonical empty encoding (13 bytes).

### `decompress(data: Uint8Array): Uint8Array`

Decompress CMP06 wire-format bytes. Returns the original uncompressed bytes. Input must be produced by `compress()` or be a conforming CMP06 payload.

## Dependencies

- [`@coding-adventures/huffman-tree`](../huffman-tree) — DT27 canonical Huffman tree builder. Used to build and encode/decode the 6 Huffman trees (ICC, distance, 4 literal context trees).

## Spec

See [`code/specs/CMP06-brotli.md`](../../../../specs/CMP06-brotli.md) for the full algorithm specification including the complete ICC table, distance code table, and wire format definition.

## Differences from RFC 7932

This is an educational implementation. The following simplifications apply:

| Feature | RFC 7932 | CMP06 (CodingAdventures) |
|---------|----------|--------------------------|
| Context buckets | 64 (6-bit context function) | 4 (2-bit: space/digit/upper/lower) |
| Static dictionary | 122,784 word-form entries | Omitted |
| Window size | Up to 16 MiB | 65,535 bytes |
| Distance codes | Complex backreference + recent-dist cache | 32 codes, no recent-dist cache |
| Meta-blocks | Multiple blocks with different modes | Single block |
