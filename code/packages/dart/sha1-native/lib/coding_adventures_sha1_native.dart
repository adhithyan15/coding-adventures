/// SHA-1 — **native-through-Rust** Dart bindings.
///
/// Same API as the pure-Dart `coding_adventures_sha1` package, but every digest
/// is computed by the Rust `coding_adventures_sha1` crate through a C ABI
/// (`dart:ffi`). Shares a single Rust source of truth with the other bindings.
///
/// **Security note:** SHA-1 is broken for collision resistance — checksum/legacy
/// use only.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_sha1_native/coding_adventures_sha1_native.dart';
///
/// void main() {
///   print(hexString(utf8.encode('abc'))); // computed in Rust
///   // a9993e364706816aba3e25717850c26c9cd0d89d
///
///   final h = Sha1Digest()..update(utf8.encode('ab'))..update(utf8.encode('c'));
///   print(h.hexDigest());
///   h.dispose();
/// }
/// ```
///
/// The shared library is located via `SHA1_NATIVE_PATH` (an absolute path) or
/// the platform default name; `tools/run-tests.sh` sets it before running tests.
library coding_adventures_sha1_native;

import 'dart:typed_data';

import 'src/ffi.dart' as ffi;

/// Compute the SHA-1 digest of [data] (executed in Rust) as a 20-byte [Uint8List].
Uint8List sum1(List<int> data) => ffi.nativeSha1(data);

/// Compute SHA-1 and return the 40-character lowercase hex string (executed in Rust).
String hexString(List<int> data) => ffi.nativeSha1Hex(data);

/// A streaming SHA-1 hasher backed by a native Rust `Digest`.
///
/// The native handle is freed automatically on garbage collection (via a
/// `NativeFinalizer`); call [dispose] to release it eagerly. Using a disposed
/// digest throws [StateError].
class Sha1Digest {
  final ffi.NativeDigestHandle _handle;

  /// Create a new streaming hasher.
  Sha1Digest() : _handle = ffi.NativeDigestHandle.create();

  Sha1Digest._(this._handle);

  /// Feed more bytes into the hash.
  void update(List<int> data) => _handle.update(data);

  /// Return the 20-byte digest of all data fed so far (non-destructive).
  Uint8List sum1() => _handle.digest();

  /// Return the 40-character lowercase hex digest string.
  String hexDigest() {
    final d = sum1();
    final sb = StringBuffer();
    for (final b in d) {
      sb.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return sb.toString();
  }

  /// Return an independent copy (a separate native handle).
  Sha1Digest cloneDigest() => Sha1Digest._(_handle.clone());

  /// Free the native handle now instead of waiting for finalization. Idempotent.
  void dispose() => _handle.dispose();
}
