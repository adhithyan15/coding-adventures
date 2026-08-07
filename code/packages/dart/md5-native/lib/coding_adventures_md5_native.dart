/// MD5 — **native-through-Rust** Dart bindings.
///
/// Same API as the pure-Dart `coding_adventures_md5` package, but every digest
/// is computed by the Rust `coding_adventures_md5` crate through a C ABI
/// (`dart:ffi`). Shares a single Rust source of truth with the other bindings.
///
/// **Security note:** MD5 is cryptographically broken — checksum use only.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_md5_native/coding_adventures_md5_native.dart';
///
/// void main() {
///   print(hexString(utf8.encode('abc'))); // computed in Rust
///   // 900150983cd24fb0d6963f7d28e17f72
///
///   final h = Md5Digest()
///     ..update(utf8.encode('ab'))
///     ..update(utf8.encode('c'));
///   print(h.hexDigest());
///   h.dispose(); // optional; the finalizer also frees it
/// }
/// ```
///
/// The shared library is located via `MD5_NATIVE_PATH` (an absolute path) or the
/// platform default name; `tools/run-tests.sh` builds the cdylib and sets that
/// variable before running the tests.
library coding_adventures_md5_native;

import 'dart:typed_data';

import 'src/ffi.dart' as ffi;

/// Compute the MD5 digest of [data] (executed in Rust) as a 16-byte [Uint8List].
Uint8List sumMd5(List<int> data) => ffi.nativeMd5(data);

/// Compute MD5 and return the 32-character lowercase hex string (executed in
/// Rust).
String hexString(List<int> data) => ffi.nativeMd5Hex(data);

/// A streaming MD5 hasher backed by a native Rust `Digest`.
///
/// The native handle is freed automatically on garbage collection (via a
/// `NativeFinalizer`); call [dispose] to release it eagerly. Using a disposed
/// digest throws [StateError].
class Md5Digest {
  final ffi.NativeDigestHandle _handle;

  /// Create a new streaming hasher.
  Md5Digest() : _handle = ffi.NativeDigestHandle.create();

  Md5Digest._(this._handle);

  /// Feed more bytes into the hash.
  void update(List<int> data) => _handle.update(data);

  /// Return the 16-byte digest of all data fed so far (non-destructive).
  Uint8List sumMd5() => _handle.digest();

  /// Return the 32-character lowercase hex digest string.
  String hexDigest() {
    final d = sumMd5();
    final sb = StringBuffer();
    for (final b in d) {
      sb.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return sb.toString();
  }

  /// Return an independent copy (a separate native handle).
  Md5Digest cloneDigest() => Md5Digest._(_handle.clone());

  /// Free the native handle now instead of waiting for finalization. Idempotent.
  void dispose() => _handle.dispose();
}
