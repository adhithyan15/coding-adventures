# @coding-adventures/zstd

ZStd (Zstandard, RFC 8878) lossless compression and decompression — CMP07.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo compression series:

```
CMP02 (LZSS)    — LZ77 + flag bits           ← dependency
CMP07 (ZStd)    — LZ77 + FSE; high ratio     ← this package
```

## What is ZStd?

Zstandard was created by Yann Collet at Facebook in 2015 and published as
[RFC 8878](https://www.rfc-editor.org/rfc/rfc8878). It combines two ideas:

1. **LZ77 back-references** — Find repeated substrings and encode them as
   `(offset, length)` pairs instead of copying the bytes.  The LZSS package
   does this search with a 32 KB sliding window.

2. **FSE (Finite State Entropy)** — Encode the stream of `(literal_length,
   match_length, offset_code)` triples using asymmetric numeral systems.  FSE
   achieves near-Shannon entropy in a single pass with no per-symbol prefix
   lookups.

The result: high compression ratios with very fast decode speeds, making it
ideal for network transfer, databases, and build artifact caching.

## Installation

```bash
npm install @coding-adventures/zstd
```

Or from the monorepo:

```bash
# install the lzss dep first, then this package
cd code/packages/typescript/lzss && npm install
cd ../zstd && npm install
```

## Usage

```ts
import { compress, decompress } from "@coding-adventures/zstd";

const encoder = new TextEncoder();
const decoder = new TextDecoder();

// Compress
const original   = encoder.encode("the quick brown fox jumps over the lazy dog ".repeat(100));
const compressed = compress(original);
console.log(`${original.length} → ${compressed.length} bytes`);

// Decompress
const recovered = decompress(compressed);
console.log(decoder.decode(recovered)); // original text
```

`decompress` throws an `Error` if the input is not a valid ZStd frame.

## API

### `compress(data: Uint8Array): Uint8Array`

Compress `data` into a ZStd frame (RFC 8878). The output is a self-contained
frame with a magic number, Frame Content Size, and one or more blocks. It can
be decompressed by the `zstd` CLI or any conforming implementation.

### `decompress(data: Uint8Array): Uint8Array`

Decompress a ZStd frame. Supports:
- Raw, RLE, and Compressed block types
- Single-segment and multi-segment layouts
- Predefined FSE modes (no per-frame table description)

Throws on bad magic, truncated data, or unsupported features (non-predefined
FSE tables, Huffman-coded literals).

## How it works — Frame structure

```
┌────────┬─────┬──────────────────────┬────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │
│ 4 B LE │ 1 B │ 8 B LE               │ ...    │
└────────┴─────┴──────────────────────┴────────┘
```

Each **block** has a 3-byte header:
```
bit 0       = Last_Block flag
bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed)
bits [23:3] = Block_Size
```

Compressed blocks contain:
```
[Literals section (Raw_Literals)]
[Sequence count (1–3 bytes)]
[Symbol modes byte = 0x00 (all Predefined)]
[FSE bitstream (backward bit stream)]
```

## Backward bit stream

ZStd sequences are encoded in a **reversed** bitstream. The encoder writes bits
for the last sequence first. The decoder reads them in forward order because the
bitstream is reversed — last written = first read.

The stream ends with a **sentinel bit** (the highest set bit of the last byte)
so the decoder knows where to start.

Example:
```
Write: bits A (3), then B (8), then C (1)
Buffer (bytes): [byte0, ..., sentinel_byte]
Read:  C first, then B, then A
```

## FSE (Finite State Entropy)

FSE is a tANS (tabled Asymmetric Numeral System). The encoder and decoder share
a table built from a probability distribution. The table has `2^accLog` slots.

**Decode:** Given state S, look up `tbl[S]` → `{sym, nb, base}`. Output `sym`.
Read `nb` bits. New state = `base + bits`.

**Encode:** Given symbol `sym`, compute how many bits to emit, emit them, and
look up the new state. Done in reverse order so the decoder sees sequences
forward.

ZStd uses *predefined* distributions (RFC 8878 Appendix B) for the three
sequence fields, so no table description is needed in the frame.

## Stack fit

```
┌────────────────────────────────────┐
│      zstd (this package)           │  ← CMP07
│  compress / decompress             │
│  RevBitWriter / RevBitReader       │
│  FSE encode / decode tables        │
├────────────────────────────────────┤
│      lzss                          │  ← CMP02 (dependency)
│  encode(data, window, max, min)    │
└────────────────────────────────────┘
```

## Running tests

```bash
npm test               # run once
npm run test:coverage  # with coverage report
```

Target: >95% line coverage.

## Compression levels

This implementation uses a fixed compression strategy (no level parameter):
- Window size: 32 KB
- Max match: 255 bytes
- Min match: 3 bytes
- Always Raw_Literals (no Huffman coding of literals)

Higher compression is possible by adding Huffman-coded literals, larger
windows, and content-aware parsing — left as exercises.
