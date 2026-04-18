# coding_adventures_lz78

LZ78 explicit-dictionary compression for Dart. This package implements the
CMP01 specification from the coding-adventures compression series and exposes
both a token-level teaching API and a one-shot byte-oriented API.

## What It Provides

- `Token` values of the form `(dictIndex, nextChar)`
- `TrieCursor` for step-by-step trie-based dictionary construction
- `encode` and `decode` for working directly with LZ78 token streams
- `compress` and `decompress` for one-shot binary compression
- `serialiseTokens` and `deserialiseTokens` for the fixed-width CMP01 wire format

## Usage

```dart
import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lz78/lz78.dart';

void main() {
  final data = Uint8List.fromList(utf8.encode('hello hello hello world'));

  final compressed = compress(data);
  final original = decompress(compressed);

  final tokens = encode(data);
  final decoded = decode(tokens, data.length);

  print(original.length == decoded.length);
}
```

## Building and Testing

```bash
dart pub get
dart test
```
