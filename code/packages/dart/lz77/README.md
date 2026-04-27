# coding_adventures_lz77

LZ77 sliding-window compression for Dart. This package implements the CMP00
specification from the coding-adventures compression series and exposes both a
token-level teaching API and a one-shot byte-oriented API.

## What It Provides

- `Token` values of the form `(offset, length, nextChar)`
- `encode` and `decode` for working directly with LZ77 token streams
- `compress` and `decompress` for one-shot binary compression
- `serialiseTokens` and `deserialiseTokens` for the fixed-width teaching format

## Usage

```dart
import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lz77/lz77.dart';

void main() {
  final data = Uint8List.fromList(utf8.encode('hello hello hello world'));

  final compressed = compress(data);
  final original = decompress(compressed);

  final tokens = encode(data);
  final decoded = decode(tokens);

  print(original.length == decoded.length);
}
```

## Building and Testing

```bash
dart pub get
dart test
```
