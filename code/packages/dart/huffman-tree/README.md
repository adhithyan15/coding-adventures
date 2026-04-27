# Huffman Tree

Deterministic Huffman tree construction and canonical code tables for Dart.

## What it does

`coding_adventures_huffman_tree` implements DT27 in the Dart lane. It builds a
full binary Huffman tree from `(symbol, frequency)` pairs, exposes the
walk-based and canonical code tables, decodes bit strings back to symbols, and
lets tests verify the core structural invariants.

## Why it exists

CMP04 Huffman compression depends on DT27 rather than rebuilding tree logic in
every compression package. That keeps the layering honest:

- `huffman-tree` owns deterministic tree construction and code derivation
- `huffman-compression` owns the wire format and bit packing

## Usage

```dart
import 'package:coding_adventures_huffman_tree/huffman_tree.dart';

void main() {
  final tree = HuffmanTree.build(<(int, int)>[(65, 3), (66, 2), (67, 1)]);
  print(tree.codeTable());          // {65: 0, 67: 10, 66: 11}
  print(tree.canonicalCodeTable()); // {65: 0, 66: 10, 67: 11}
  print(tree.decodeAll('001011', 4)); // [65, 65, 67, 66]
}
```

## How it fits in the stack

This package is part of the Dart data-structure lane and serves as the
foundation for Dart CMP04 Huffman compression.
