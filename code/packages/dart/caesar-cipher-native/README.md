# coding_adventures_caesar_cipher_native

**Native-through-Rust** Dart bindings for the Caesar cipher. This package
exposes the same API as the pure-Dart
[`coding_adventures_caesar_cipher`](../caesar-cipher) package, but every call is
executed by the Rust `caesar-cipher` crate through a C ABI, loaded via
`dart:ffi`.

It is the companion of the pure port. The two demonstrate the repo's two
porting strategies side by side:

| | Pure port | Native-through-Rust (this package) |
|---|---|---|
| Implementation | Reimplemented in Dart | The Rust `caesar-cipher` crate |
| Source of truth | Its own code | Shared with Rust/Python/… |
| Dependency | none | `package:ffi` + a Rust cdylib |
| When to prefer | readable reference, no toolchain | one implementation, many languages |

## How it works

```
Dart  ──toNativeUtf8──▶  caesar_encrypt(ptr, shift)   [extern "C", Rust]
      ◀──toDartString──  char*  ──caesar_free_string──▶ (freed)
```

The Rust side (`src/lib.rs`) is a thin `cdylib` exposing five `extern "C"`
functions over the pure `caesar-cipher` crate:

```c
char* caesar_encrypt(const char* text, int shift);
char* caesar_decrypt(const char* text, int shift);
char* caesar_rot13(const char* text);
char* caesar_frequency_analysis(const char* ciphertext, int* out_shift);
void  caesar_free_string(char* s);
```

Every returned `char*` is Rust-owned heap memory; the Dart layer copies it into
a `String` and immediately calls `caesar_free_string`, so nothing leaks.

`bruteForce` is composed on the Dart side from 25 native `caesar_decrypt` calls
(each executed in Rust) rather than a dedicated native function: serialising 25
arbitrary plaintexts into one C string cannot be made delimiter-safe, since a
plaintext may contain any non-letter byte. Composing keeps it correct for *any*
input.

## Usage

```dart
import 'package:coding_adventures_caesar_cipher_native/coding_adventures_caesar_cipher_native.dart';

void main() {
  final ct = encrypt('Attack at dawn!', 3); // runs in Rust → 'Dwwdfn dw gdzq!'
  print(decrypt(ct, 3));                     // → 'Attack at dawn!'
  print(rot13('Hello'));                     // → 'Uryyb'

  final r = frequencyAnalysis(encrypt('THE QUICK BROWN FOX', 7));
  print('${r.shift}: ${r.plaintext}');       // → '7: THE QUICK BROWN FOX'
}
```

## Building and testing

`tools/run-tests.sh` builds the cdylib with cargo, points the FFI loader at it
via `CAESAR_CIPHER_NATIVE_PATH`, and runs `dart test`:

```
sh tools/run-tests.sh
```

The shared library is located from `CAESAR_CIPHER_NATIVE_PATH` (an absolute
path) if set, otherwise from the platform default name on the loader search
path (`libcaesar_cipher_native.so` / `.dylib`).

## Caveats

- **NUL bytes**: a C string cannot carry an interior `\0`, so text containing a
  literal NUL is treated as empty across the boundary. The pure port has no such
  limit. Every other byte (punctuation, digits, UTF-8, tabs, newlines)
  round-trips unchanged.
- **Windows CI** is skipped for this package (the cdylib cross-compile setup is
  out of scope); it builds and tests on Linux and macOS.
