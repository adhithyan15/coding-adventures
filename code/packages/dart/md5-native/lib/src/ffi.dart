// ffi.dart — dart:ffi bindings for the md5-native Rust cdylib.
//
// Mirrors sha256-native (see that package for the design rationale): binary
// byte-buffer I/O with a caller-owned 16-byte digest buffer, plus an opaque
// streaming handle freed by a NativeFinalizer.
//
// THE C CONTRACT (see ../src/lib.rs)
//   void  md5_digest(const uint8_t* data, size_t len, uint8_t* out16);
//   char* md5_hex(const uint8_t* data, size_t len);   // freed by ↓
//   void  md5_free_string(char* s);
//   HASHER* md5_hasher_new(void);
//   void    md5_hasher_update(HASHER*, const uint8_t* data, size_t len);
//   void    md5_hasher_digest(const HASHER*, uint8_t* out16);
//   HASHER* md5_hasher_clone(const HASHER*);
//   void    md5_hasher_free(HASHER*);

import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

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
typedef _HasherFreeC = Void Function(Pointer<Void>);
typedef _HasherFreeDart = void Function(Pointer<Void>);

final DynamicLibrary _lib = _load();

DynamicLibrary _load() {
  final envPath = Platform.environment['MD5_NATIVE_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError('MD5_NATIVE_PATH must be an absolute path, got: $envPath');
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) return DynamicLibrary.open('libmd5_native.dylib');
  if (Platform.isWindows) return DynamicLibrary.open('md5_native.dll');
  return DynamicLibrary.open('libmd5_native.so');
}

bool _isAbsolute(String p) =>
    p.startsWith('/') || RegExp(r'^[A-Za-z]:[\\/]').hasMatch(p);

final _digest = _lib.lookupFunction<_DigestC, _DigestDart>('md5_digest');
final _hex = _lib.lookupFunction<_HexC, _HexDart>('md5_hex');
final _freeStr = _lib.lookupFunction<_FreeStrC, _FreeStrDart>('md5_free_string');
final _hasherNew =
    _lib.lookupFunction<_HasherNewC, _HasherNewDart>('md5_hasher_new');
final _hasherUpdate = _lib
    .lookupFunction<_HasherUpdateC, _HasherUpdateDart>('md5_hasher_update');
final _hasherDigest = _lib
    .lookupFunction<_HasherDigestC, _HasherDigestDart>('md5_hasher_digest');
final _hasherClone =
    _lib.lookupFunction<_HasherCloneC, _HasherCloneDart>('md5_hasher_clone');
final _hasherFree =
    _lib.lookupFunction<_HasherFreeC, _HasherFreeDart>('md5_hasher_free');

final NativeFinalizer _hasherFinalizer = NativeFinalizer(
    _lib.lookup<NativeFunction<Void Function(Pointer<Void>)>>('md5_hasher_free'));

Pointer<Uint8> _toNative(List<int> data) {
  if (data.isEmpty) return nullptr;
  final ptr = malloc<Uint8>(data.length);
  ptr.asTypedList(data.length).setAll(0, data);
  return ptr;
}

/// Compute the MD5 digest of [data] (executed in Rust) as 16 bytes.
Uint8List nativeMd5(List<int> data) {
  final input = _toNative(data);
  final out = malloc<Uint8>(16);
  try {
    _digest(input, data.length, out);
    return Uint8List.fromList(out.asTypedList(16));
  } finally {
    if (input != nullptr) malloc.free(input);
    malloc.free(out);
  }
}

/// Compute the 32-character lowercase hex digest of [data] (executed in Rust).
String nativeMd5Hex(List<int> data) {
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

/// Owns a native `Digest` handle, freed automatically via a [NativeFinalizer]
/// or eagerly via [dispose].
class NativeDigestHandle implements Finalizable {
  Pointer<Void> _handle;
  bool _disposed = false;

  NativeDigestHandle._(this._handle) {
    _hasherFinalizer.attach(this, _handle, detach: this);
  }

  factory NativeDigestHandle.create() => NativeDigestHandle._(_hasherNew());

  void _checkAlive() {
    if (_disposed) throw StateError('digest has been disposed');
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
    final out = malloc<Uint8>(16);
    try {
      _hasherDigest(_handle, out);
      return Uint8List.fromList(out.asTypedList(16));
    } finally {
      malloc.free(out);
    }
  }

  NativeDigestHandle clone() {
    _checkAlive();
    return NativeDigestHandle._(_hasherClone(_handle));
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _hasherFinalizer.detach(this);
    _hasherFree(_handle);
    _handle = nullptr;
  }
}
