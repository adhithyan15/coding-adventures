# coding_adventures_md5

MD5 message-digest algorithm (RFC 1321) implemented from scratch in pure Dart —
no `crypto` package, no native code.

Dart port of the `md5` package that already exists in Rust, Python, Swift, and
other languages in the coding-adventures monorepo; produces byte-identical
digests to those ports and to any conforming MD5.

> **Security:** MD5 is cryptographically **broken** — practical collision
> attacks exist. Never use it for digital signatures, certificates, or password
> storage. It remains fine as a fast, non-adversarial integrity checksum.

## API

| Function | Purpose |
|---|---|
| `sumMd5(List<int> data)` → `Uint8List` | 16-byte digest. |
| `hexString(List<int> data)` → `String` | 32-char lowercase hex digest. |
| `Md5Digest` | Incremental hashing: `update` / `sumMd5` / `hexDigest` / `cloneDigest`. |

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_md5/coding_adventures_md5.dart';

void main() {
  print(hexString(utf8.encode('abc')));
  // 900150983cd24fb0d6963f7d28e17f72

  final h = Md5Digest();
  h.update(utf8.encode('ab'));
  h.update(utf8.encode('c'));
  print(h.hexDigest()); // same digest, computed incrementally
}
```

## How it works

MD5 pads the message (append `0x80`, zero-fill to 56 mod 64, then the 64-bit
**little-endian** bit length) and folds each 64-byte block into a four-word
state through 64 rounds. MD5 is **little-endian** throughout — block words and
the output digest read least-significant byte first, the opposite of
SHA-1/SHA-256. Because Dart's `int` is 64-bit, every add and rotate is masked
with `& 0xFFFFFFFF`.

## Running the tests

```
dart pub get
dart test
```
