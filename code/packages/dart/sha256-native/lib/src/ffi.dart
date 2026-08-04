// ffi.dart — dart:ffi bindings for the sha256-native Rust cdylib.
//
// LIBRARY LOADING
// ───────────────
// The shared library path comes from SHA256_NATIVE_PATH (an absolute path set
// by tools/run-tests.sh) or the platform default name on the loader search path
// (libsha256_native.so / .dylib / sha256_native.dll).
//
// THE C CONTRACT (see ../src/lib.rs)
// ──────────────────────────────────
//   void  sha256_digest(const uint8_t* data, size_t len, uint8_t* out32);
//   char* sha256_hex(const uint8_t* data, size_t len);   // freed by ↓
//   void  sha256_free_string(char* s);
//   HASHER* sha256_hasher_new(void);
//   void    sha256_hasher_update(HASHER*, const uint8_t* data, size_t len);
//   void    sha256_hasher_digest(const HASHER*, uint8_t* out32);
//   HASHER* sha256_hasher_clone(const HASHER*);
//   void    sha256_hasher_free(HASHER*);
//
// The digest functions write into a caller-owned 32-byte buffer (no allocation
// crosses the boundary). Only sha256_hex allocates, and its result is freed
// immediately after copying. The opaque HASHER handle is freed by a
// NativeFinalizer attached to the Dart wrapper, plus an explicit dispose().

import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

// ── Native signatures ────────────────────────────────────────────────────────

typedef _DigestC = Void Function(Pointer<Uint8>, Size, Pointer<Uint8>);
typedef _DigestDart = void Function(Pointer<Uint8>, int, Pointer<Uint8>);

typedef _HexC = Pointer<Utf8> Function(Pointer<Uint8>, Size);
typedef _HexDart = Pointer<Utf8> Function(Pointer<Uint8>, int);

typedef _FreeStrC = Void Function(Pointer<Utf8>);
typedef _FreeStrDart = void Function(Pointer<Utf8>);

typedef _HasherNewC = Pointer<Void> Function();
typedef _HasherNewDart = Pointer<Void> Function();

typedef _HasherUpdateC = Void Function(Pointer<Void>, Pointer<Uint8>, Size);
typedef _HasherUpdateDart = void Function(Pointer<Void>, Pointer<Uint8>, int);

typedef _HasherDigestC = Void Function(Pointer<Void>, Pointer<Uint8>);
typedef _HasherDigestDart = void Function(Pointer<Void>, Pointer<Uint8>);

typedef _HasherCloneC = Pointer<Void> Function(Pointer<Void>);
typedef _HasherCloneDart = Pointer<Void> Function(Pointer<Void>);

// ── Library + bound functions ────────────────────────────────────────────────

final DynamicLibrary _lib = _load();

DynamicLibrary _load() {
  final envPath = Platform.environment['SHA256_NATIVE_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError('SHA256_NATIVE_PATH must be an absolute path, got: $envPath');
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) return DynamicLibrary.open('libsha256_native.dylib');
  if (Platform.isWindows) return DynamicLibrary.open('sha256_native.dll');
  return DynamicLibrary.open('libsha256_native.so');
}

bool _isAbsolute(String p) =>
    p.startsWith('/') || RegExp(r'^[A-Za-z]:[\\/]').hasMatch(p);

final _digest = _lib.lookupFunction<_DigestC, _DigestDart>('sha256_digest');
final _hex = _lib.lookupFunction<_HexC, _HexDart>('sha256_hex');
final _freeStr =
    _lib.lookupFunction<_FreeStrC, _FreeStrDart>('sha256_free_string');
final _hasherNew =
    _lib.lookupFunction<_HasherNewC, _HasherNewDart>('sha256_hasher_new');
final _hasherUpdate = _lib
    .lookupFunction<_HasherUpdateC, _HasherUpdateDart>('sha256_hasher_update');
final _hasherDigest = _lib
    .lookupFunction<_HasherDigestC, _HasherDigestDart>('sha256_hasher_digest');
final _hasherClone =
    _lib.lookupFunction<_HasherCloneC, _HasherCloneDart>('sha256_hasher_clone');

/// The native `sha256_hasher_free` as a finalizer callback, so handles are
/// reclaimed automatically when the Dart wrapper is garbage-collected.
final NativeFinalizer _hasherFinalizer = NativeFinalizer(
    _lib.lookup<NativeFunction<Void Function(Pointer<Void>)>>(
        'sha256_hasher_free'));

// ── Byte-buffer helpers ──────────────────────────────────────────────────────

/// Copy [data] into freshly `malloc`ed native memory. The caller frees it.
/// Returns a null pointer for empty input (the C side treats null+0 as empty).
Pointer<Uint8> _toNative(List<int> data) {
  if (data.isEmpty) return nullptr;
  final ptr = malloc<Uint8>(data.length);
  ptr.asTypedList(data.length).setAll(0, data);
  return ptr;
}

// ── One-shot API ─────────────────────────────────────────────────────────────

/// Compute the SHA-256 digest of [data] (executed in Rust) as 32 bytes.
Uint8List nativeSha256(List<int> data) {
  final input = _toNative(data);
  final out = malloc<Uint8>(32);
  try {
    _digest(input, data.length, out);
    // Copy out of native memory into a Dart-owned list before freeing.
    return Uint8List.fromList(out.asTypedList(32));
  } finally {
    if (input != nullptr) malloc.free(input);
    malloc.free(out);
  }
}

/// Compute the 64-character lowercase hex digest of [data] (executed in Rust).
String nativeSha256Hex(List<int> data) {
  final input = _toNative(data);
  try {
    final ptr = _hex(input, data.length);
    if (ptr == nullptr) return '';
    try {
      return ptr.toDartString();
    } finally {
      _freeStr(ptr);
    }
  } finally {
    if (input != nullptr) malloc.free(input);
  }
}

// ── Streaming hasher (opaque handle wrapper) ─────────────────────────────────

/// Owns a native `Sha256Hasher` handle. Freed automatically via a
/// [NativeFinalizer], or eagerly via [dispose].
class NativeHasherHandle implements Finalizable {
  Pointer<Void> _handle;
  bool _disposed = false;

  NativeHasherHandle._(this._handle) {
    _hasherFinalizer.attach(this, _handle, detach: this);
  }

  /// Allocate a fresh native hasher.
  factory NativeHasherHandle.create() =>
      NativeHasherHandle._(_hasherNew());

  void _checkAlive() {
    if (_disposed) throw StateError('hasher has been disposed');
  }

  void update(List<int> data) {
    _checkAlive();
    final input = _toNative(data);
    try {
      _hasherUpdate(_handle, input, data.length);
    } finally {
      if (input != nullptr) malloc.free(input);
    }
  }

  Uint8List digest() {
    _checkAlive();
    final out = malloc<Uint8>(32);
    try {
      _hasherDigest(_handle, out);
      return Uint8List.fromList(out.asTypedList(32));
    } finally {
      malloc.free(out);
    }
  }

  NativeHasherHandle clone() {
    _checkAlive();
    return NativeHasherHandle._(_hasherClone(_handle));
  }

  /// Free the native handle now instead of waiting for finalization. Idempotent.
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _hasherFinalizer.detach(this);
    // sha256_hasher_free via the finalizer function pointer is not directly
    // callable here; use the bound lookup instead.
    _hasherFree(_handle);
    _handle = nullptr;
  }
}

typedef _HasherFreeC = Void Function(Pointer<Void>);
typedef _HasherFreeDart = void Function(Pointer<Void>);
final _hasherFree =
    _lib.lookupFunction<_HasherFreeC, _HasherFreeDart>('sha256_hasher_free');
