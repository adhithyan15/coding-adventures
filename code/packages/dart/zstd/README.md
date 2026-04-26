# coding_adventures_zstd

Zstandard (ZStd, RFC 8878) lossless compression for Dart. This package
implements the CMP07 entry in the coding-adventures compression series — a
complete ZStd compressor and decompressor written from scratch with no
native dependencies.

## What It Provides

- `compress(Uint8List data) → Uint8List` — produce a valid RFC 8878 ZStd frame
- `decompress(Uint8List data) → Uint8List` — decode any supported ZStd frame

### Algorithm Highlights

ZStd achieves high compression ratios at fast speeds by combining two ideas:

1. **LZ77 back-references** (via the `coding_adventures_lzss` package) — the
   same "copy from earlier in the output" trick used by DEFLATE and gzip, but
   with a 32 KB window for better ratio.

2. **FSE (Finite State Entropy)** coding — replaces Huffman coding for the
   sequence descriptor symbols (literal length, match length, offset). FSE is
   an asymmetric numeral system that approaches the Shannon entropy limit in a
   single forward pass, faster than arithmetic coding.

### Frame Layout (RFC 8878 §3)

```
┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
│ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
│ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
└────────┴─────┴──────────────────────┴────────┴──────────────────┘
```

Each block has a 3-byte header encoding the last-block flag, block type
(Raw/RLE/Compressed), and block size.

### Block Types

- **Raw** — verbatim copy, used as fallback when compression is not beneficial.
- **RLE** — single byte repeated N times; compresses identical-byte runs to
  4 bytes total (3-byte header + 1 payload byte).
- **Compressed** — LZ77 back-references encoded with predefined FSE tables;
  used when the result is smaller than the input.

### Compression Series

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

```dart
import 'dart:convert';
import 'dart:typed_data';
import 'package:coding_adventures_zstd/coding_adventures_zstd.dart';

void main() {
  final original = Uint8List.fromList(
    utf8.encode('the quick brown fox jumps over the lazy dog ' * 20),
  );

  final compressed = compress(original);
  final restored = decompress(compressed);

  print('Original:   ${original.length} bytes');
  print('Compressed: ${compressed.length} bytes');
  print('Ratio:      ${(compressed.length / original.length * 100).toStringAsFixed(1)}%');
  print('Match:      ${restored.length == original.length}');
}
```

## Building and Testing

```bash
dart pub get
dart test
```

## Limitations

- Only **predefined FSE mode** is supported (no per-frame table descriptions).
- Only **raw literals** encoding is emitted (Huffman literals are not produced,
  but the decoder will error if it encounters them from another encoder).
- No content checksum validation.
- No custom dictionary support.
- Inputs larger than 256 MB will be rejected on decompression (bomb guard).
