# Huffman Compression

Canonical Huffman compression and decompression for Dart.

## What it does

`coding_adventures_huffman_compression` implements CMP04 in the Dart lane. It
counts byte frequencies, builds a DT27 `HuffmanTree`, serialises the canonical
code lengths into the CMP04 header, and packs the compressed bit stream
LSB-first.

## Wire format

Each payload is:

- 4 bytes: original length as big-endian uint32
- 4 bytes: symbol count as big-endian uint32
- `2 * symbol_count` bytes: `(symbol, code_length)` pairs sorted by
  `(code_length, symbol)`
- remaining bytes: the canonical Huffman bit stream packed LSB-first

The decoder fails closed on malformed headers, duplicate symbols, invalid code
lengths, impossible canonical tables, truncated streams, invalid prefixes, and
non-zero padding bits.

## Usage

```dart
import 'dart:typed_data';

import 'package:coding_adventures_huffman_compression/huffman_compression.dart';

void main() {
  final input = Uint8List.fromList('AAABBC'.codeUnits);
  final compressed = compress(input);
  final recovered = decompress(compressed);
  print(String.fromCharCodes(recovered)); // AAABBC
}
```

## How it fits in the stack

This package is the Dart CMP04 compression layer and depends on
`coding_adventures_huffman_tree` for deterministic tree construction and
canonical code generation.

## Direct dependencies

huffman-tree
