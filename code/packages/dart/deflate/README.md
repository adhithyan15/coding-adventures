# Deflate

DEFLATE lossless compression algorithm (1996) for Dart.

## What it does

`coding_adventures_deflate` implements CMP05 in the Dart lane. It first uses
the existing Dart `lzss` package to tokenise input into literals and
backreferences, then applies canonical Huffman coding over the DEFLATE
literal/length and distance alphabets.

## Wire format

Each payload contains:

- 4 bytes: original length as big-endian uint32
- 2 bytes: LL code-length entry count as big-endian uint16
- 2 bytes: distance code-length entry count as big-endian uint16
- `3 * ll_entry_count` bytes: LL `(symbol, code_length)` pairs
- `3 * dist_entry_count` bytes: distance `(symbol, code_length)` pairs
- remaining bytes: the shared LSB-first bit stream

The decoder rejects malformed tables, invalid symbols, truncated bit streams,
invalid backreferences, and non-zero trailing padding bits.

## Usage

```dart
import 'dart:typed_data';

import 'package:coding_adventures_deflate/deflate.dart';

void main() {
  final input = Uint8List.fromList('hello hello hello'.codeUnits);
  final compressed = compress(input);
  final recovered = decompress(compressed);
  print(String.fromCharCodes(recovered)); // hello hello hello
}
```

## How it fits in the stack

This package is the Dart CMP05 composition layer built on top of
`coding_adventures_lzss` and `coding_adventures_huffman_tree`.

## Direct dependencies

huffman-tree, lzss
