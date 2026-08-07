# coding_adventures_md5_native

**Native-through-Rust** Dart bindings for MD5. Same API as the pure-Dart
[`coding_adventures_md5`](../md5) package, but every digest is computed by the
Rust `coding_adventures_md5` crate through a C ABI, loaded via `dart:ffi`.

> **Security:** MD5 is cryptographically broken — checksum use only, never
> signatures or passwords.

## How it works

Same shape as `sha256-native`: binary byte buffers, a **caller-owned 16-byte
output buffer** for digests (no allocation crosses the boundary), and an opaque
streaming handle managed by a Dart `NativeFinalizer`.

```c
void  md5_digest(const uint8_t* data, size_t len, uint8_t* out16);
char* md5_hex(const uint8_t* data, size_t len);   // freed by md5_free_string
void  md5_free_string(char* s);
HASHER* md5_hasher_new(void);
void    md5_hasher_update(HASHER*, const uint8_t* data, size_t len);
void    md5_hasher_digest(const HASHER*, uint8_t* out16);
HASHER* md5_hasher_clone(const HASHER*);
void    md5_hasher_free(HASHER*);
```

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_md5_native/coding_adventures_md5_native.dart';

void main() {
  print(hexString(utf8.encode('abc'))); // computed in Rust
  // 900150983cd24fb0d6963f7d28e17f72

  final h = Md5Digest()
    ..update(utf8.encode('ab'))
    ..update(utf8.encode('c'));
  print(h.hexDigest());
  h.dispose(); // optional; the finalizer also frees it
}
```

## Building and testing

```
sh tools/run-tests.sh
```

builds the cdylib, sets `MD5_NATIVE_PATH`, and runs `dart test`. Windows CI is
skipped (cdylib cross-compile out of scope); Linux and macOS build and test.
