# coding_adventures_lzw

LZW lossless compression for Dart. This package implements the CMP03
specification from the coding-adventures compression series and exposes both
the logical code stream and the packed byte-oriented wire format.

## What It Provides

- `encodeCodes` and `decodeCodes` for working with logical LZW code streams
- `packCodes` and `unpackCodes` for the CMP03 bit-packed wire format
- `compress` and `decompress` for one-shot binary compression
- `BitWriter` and `BitReader` helpers for LSB-first variable-width codes
- Strict malformed-input validation, including header checks and padding checks
- A configurable decompressed-size cap for hostile or untrusted inputs

## Usage

```dart
import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_lzw/lzw.dart';

void main() {
  final data = Uint8List.fromList(utf8.encode('ABABABABAB'));

  final codes = encodeCodes(data);
  final packed = packCodes(codes, data.length);
  final original = decompress(packed);

  print(codes);
  print(utf8.decode(original));
}
```

## Building and Testing

```bash
dart pub get
dart test
```
