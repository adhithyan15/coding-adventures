# coding_adventures_sha1_native

**Native-through-Rust** Dart bindings for SHA-1. Same API as the pure-Dart
[`coding_adventures_sha1`](../sha1) package, but every digest is computed by the
Rust `coding_adventures_sha1` crate through a C ABI, loaded via `dart:ffi`.

> **Security:** SHA-1 is broken for collision resistance — checksum/legacy use
> only, never signatures.

## How it works

Same shape as `sha256-native`/`md5-native`: binary byte buffers, a caller-owned
**20-byte** output buffer for digests, and an opaque streaming handle managed by
a Dart `NativeFinalizer`.

```c
void  sha1_digest(const uint8_t* data, size_t len, uint8_t* out20);
char* sha1_hex(const uint8_t* data, size_t len);   // freed by sha1_free_string
void  sha1_free_string(char* s);
HASHER* sha1_hasher_new(void);
void    sha1_hasher_update(HASHER*, const uint8_t* data, size_t len);
void    sha1_hasher_digest(const HASHER*, uint8_t* out20);
HASHER* sha1_hasher_clone(const HASHER*);
void    sha1_hasher_free(HASHER*);
```

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_sha1_native/coding_adventures_sha1_native.dart';

void main() {
  print(hexString(utf8.encode('abc'))); // computed in Rust
  // a9993e364706816aba3e25717850c26c9cd0d89d

  final h = Sha1Digest()..update(utf8.encode('ab'))..update(utf8.encode('c'));
  print(h.hexDigest());
  h.dispose();
}
```

## Building and testing

```
sh tools/run-tests.sh
```

builds the cdylib, sets `SHA1_NATIVE_PATH`, and runs `dart test`. Windows CI is
skipped (cdylib cross-compile out of scope); Linux and macOS build and test.
