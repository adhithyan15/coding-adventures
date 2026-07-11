/// SHA-256 — **native-through-Rust** Dart bindings.
///
/// Same API as the pure-Dart `coding_adventures_sha256` package, but every
/// digest is computed by the Rust `coding_adventures_sha256` crate through a C
/// ABI (`dart:ffi`). The pure port is the readable reference; this one shares a
/// single Rust source of truth with the Rust, Python, and other bindings.
///
/// ## Usage
///
/// ```dart
/// import 'dart:convert';
/// import 'package:coding_adventures_sha256_native/coding_adventures_sha256_native.dart';
///
/// void main() {
///   print(sha256Hex(utf8.encode('abc'))); // computed in Rust
///   // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
///
///   final h = Sha256Hasher()
///     ..update(utf8.encode('ab'))
///     ..update(utf8.encode('c'));
///   print(h.hexDigest());
///   h.dispose(); // optional: free the native handle eagerly
/// }
/// ```
///
/// The shared library is located via `SHA256_NATIVE_PATH` (an absolute path) or
/// the platform default name; `tools/run-tests.sh` builds the cdylib and sets
/// that variable before running the tests.
library coding_adventures_sha256_native;

import 'dart:typed_data';

import 'src/ffi.dart' as ffi;

/// Compute the SHA-256 digest of [data] (executed in Rust) as a 32-byte
/// [Uint8List].
Uint8List sha256(List<int> data) => ffi.nativeSha256(data);

/// Compute SHA-256 and return the 64-character lowercase hex string (executed
/// in Rust).
String sha256Hex(List<int> data) => ffi.nativeSha256Hex(data);

/// A streaming SHA-256 hasher backed by a native Rust `Sha256Hasher`.
///
/// The underlying native handle is freed automatically when this object is
/// garbage-collected (via a `NativeFinalizer`); call [dispose] to release it
/// eagerly. Using a disposed hasher throws [StateError].
class Sha256Hasher {
  final ffi.NativeHasherHandle _handle;

  /// Create a new streaming hasher.
  Sha256Hasher() : _handle = ffi.NativeHasherHandle.create();

  Sha256Hasher._(this._handle);

  /// Feed more bytes into the hash.
  void update(List<int> data) => _handle.update(data);

  /// Return the 32-byte digest of all data fed so far (non-destructive).
  Uint8List digest() => _handle.digest();

  /// Return the 64-character lowercase hex digest string.
  String hexDigest() {
    final d = digest();
    final sb = StringBuffer();
    for (final b in d) {
      sb.write(b.toRadixString(16).padLeft(2, '0'));
    }
    return sb.toString();
  }

  /// Return an independent copy of the current hasher (a separate native
  /// handle); hashing either afterwards does not affect the other.
  Sha256Hasher cloneHasher() => Sha256Hasher._(_handle.clone());

  /// Free the native handle now instead of waiting for finalization. Idempotent.
  void dispose() => _handle.dispose();
}
