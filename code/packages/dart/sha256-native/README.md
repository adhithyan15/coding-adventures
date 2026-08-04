# coding_adventures_sha256_native

**Native-through-Rust** Dart bindings for SHA-256. Same API as the pure-Dart
[`coding_adventures_sha256`](../sha256) package, but every digest is computed by
the Rust `coding_adventures_sha256` crate through a C ABI, loaded via `dart:ffi`.

## How it works

SHA-256 is binary-in, 32-bytes-out, so the C ABI works with byte buffers rather
than C strings:

```c
void  sha256_digest(const uint8_t* data, size_t len, uint8_t* out32);
char* sha256_hex(const uint8_t* data, size_t len);   // freed by ↓
void  sha256_free_string(char* s);

// streaming, via an opaque handle:
HASHER* sha256_hasher_new(void);
void    sha256_hasher_update(HASHER*, const uint8_t* data, size_t len);
void    sha256_hasher_digest(const HASHER*, uint8_t* out32);
HASHER* sha256_hasher_clone(const HASHER*);
void    sha256_hasher_free(HASHER*);
```

- **Digest** functions write into a **caller-owned 32-byte buffer** — no
  allocation crosses the boundary, so there is nothing to free on that path.
- **Hex** is the only allocating call; its `char*` is freed immediately after
  copying to a Dart `String`.
- The **streaming hasher** is an opaque `HASHER*`. The Dart wrapper attaches a
  [`NativeFinalizer`](https://api.dart.dev/dart-ffi/NativeFinalizer-class.html)
  so the handle is freed when the object is garbage-collected; `dispose()` frees
  it eagerly. This is the reusable pattern for FFI objects with a lifecycle.

## Usage

```dart
import 'dart:convert';
import 'package:coding_adventures_sha256_native/coding_adventures_sha256_native.dart';

void main() {
  print(sha256Hex(utf8.encode('abc'))); // computed in Rust
  // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad

  final h = Sha256Hasher()
    ..update(utf8.encode('ab'))
    ..update(utf8.encode('c'));
  print(h.hexDigest());
  h.dispose(); // optional; the finalizer also frees it
}
```

## Building and testing

`tools/run-tests.sh` builds the cdylib with cargo, points the FFI loader at it
via `SHA256_NATIVE_PATH`, and runs `dart test`:

```
sh tools/run-tests.sh
```

Windows CI is skipped (cdylib cross-compile out of scope); Linux and macOS build
and test.
