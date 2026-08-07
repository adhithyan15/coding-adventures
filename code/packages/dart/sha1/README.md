# coding_adventures_sha1

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in pure
Dart — no `crypto` package, no native code.

Dart port of the `sha1` package that already exists in Rust, Python, and other
languages in the monorepo; produces byte-identical digests to those ports and to
any conforming SHA-1.

> **Security:** SHA-1 is **broken** for collision resistance (SHAttered, 2017).
> Never use it for signatures or certificates. Legacy-protocol and
> non-adversarial checksum use only (e.g. git object names).

## API

| Function | Purpose |
|---|---|
| `sum1(List<int> data)` → `Uint8List` | 20-byte digest. |
| `hexString(List<int> data)` → `String` | 40-char lowercase hex digest. |
| `Sha1Digest` | Incremental: `update` / `sum1` / `hexDigest` / `cloneDigest`. |

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_sha1/coding_adventures_sha1.dart';

void main() {
  print(hexString(utf8.encode('abc')));
  // a9993e364706816aba3e25717850c26c9cd0d89d

  final h = Sha1Digest();
  h.update(utf8.encode('ab'));
  h.update(utf8.encode('c'));
  print(h.hexDigest()); // same digest, computed incrementally
}
```

## How it works

SHA-1 pads the message (append `0x80`, zero-fill to 56 mod 64, then the 64-bit
**big-endian** bit length) and folds each 64-byte block into a five-word state
through 80 rounds. It is big-endian like SHA-256 (the opposite of MD5). Because
Dart's `int` is 64-bit, every add and rotate is masked with `& 0xFFFFFFFF`.

## Running the tests

```
dart pub get
dart test
```
