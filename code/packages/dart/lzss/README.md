# coding_adventures_lzss

LZSS sliding-window compression for Dart. This package implements the CMP02
specification from the coding-adventures compression series and exposes both a
token-level teaching API and a one-shot byte-oriented API.

## What It Provides

- `Literal` and `Match` tokens for inspecting LZSS streams directly
- `encode` and `decode` for working at the token layer
- `compress` and `decompress` for one-shot binary compression
- `serialiseTokens` and `deserialiseTokens` for the CMP02 block wire format
- Overlap-safe decoding plus strict malformed-input validation
- A configurable decompressed-size cap for hostile or untrusted inputs

## Usage

```dart
import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lzss/lzss.dart';

void main() {
  final data = Uint8List.fromList(utf8.encode('ABABABABAB'));

  final tokens = encode(data);
  final decoded = decode(tokens, data.length);

  final compressed = compress(data);
  final original = decompress(compressed);
  final trusted = decompress(compressed, data.length);

  print(tokens);
  print(decoded.length == original.length && trusted.length == data.length);
}
```

## Building and Testing

```bash
dart pub get
dart test
```
