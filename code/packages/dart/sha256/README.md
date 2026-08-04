# coding_adventures_sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in
pure Dart — no `crypto` package, no native code.

This is the Dart port of the `sha256` package that already exists in Rust,
Python, and other languages in the coding-adventures monorepo. It produces
byte-identical digests to those ports and to any conforming SHA-256.

## API

| Function | Purpose |
|---|---|
| `sha256(List<int> data)` → `Uint8List` | 32-byte digest. |
| `sha256Hex(List<int> data)` → `String` | 64-char lowercase hex digest. |
| `Sha256Hasher` | Incremental hashing via `update` / `digest` / `hexDigest` / `cloneHasher`. |

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart';

void main() {
  print(sha256Hex(utf8.encode('abc')));
  // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

  // Stream data that does not fit in memory at once:
  final h = Sha256Hasher();
  h.update(utf8.encode('ab'));
  h.update(utf8.encode('c'));
  print(h.hexDigest()); // same digest, computed incrementally
}
```

## How it works

SHA-256 pads the message to a multiple of 64 bytes (append `0x80`, zero-fill to
56 mod 64, then the 64-bit big-endian bit length), then folds each 64-byte block
into an eight-word state through a 64-round compression function. SHA-256 is
defined over unsigned 32-bit words; because Dart's `int` is 64-bit, every add,
shift, and rotate is masked with `& 0xFFFFFFFF` to stay within 32 bits.

## Security note

SHA-256 remains cryptographically secure, but a hash alone is **not** a password
scheme — use a purpose-built KDF (scrypt, argon2, PBKDF2) for passwords. This
implementation is written for clarity and correctness against the FIPS 180-4
test vectors, not side-channel resistance.

## Running the tests

```
dart pub get
dart test
```
