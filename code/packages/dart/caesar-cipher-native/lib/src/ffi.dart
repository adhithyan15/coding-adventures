// ffi.dart — dart:ffi bindings for the caesar-cipher-native Rust cdylib.
//
// LIBRARY LOADING
// ───────────────
// The shared library is located, in order of preference:
//   1. The absolute path in the CAESAR_CIPHER_NATIVE_PATH environment variable
//      (set by tools/run-tests.sh, which builds the cdylib with cargo first).
//   2. The platform default name on the loader search path
//      (libcaesar_cipher_native.so / .dylib / caesar_cipher_native.dll).
//
// THE C CONTRACT (see ../src/lib.rs)
// ──────────────────────────────────
//   char* caesar_encrypt(const char* text, int shift);
//   char* caesar_decrypt(const char* text, int shift);
//   char* caesar_rot13(const char* text);
//   char* caesar_frequency_analysis(const char* ct, int* out_shift);
//   void  caesar_free_string(char* s);
//
// Brute force is deliberately NOT a native entry point — the public bruteForce()
// composes it from 25 caesar_decrypt calls (see the library file), which is
// robust to any input. Serialising 25 arbitrary plaintexts into one C string
// cannot be made delimiter-safe.
//
// Every char* the library returns is Rust-owned heap memory; we copy it into a
// Dart String and immediately hand the pointer back to caesar_free_string so
// nothing leaks across the boundary.

import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

// ── Native function signatures (C side / Dart side) ──────────────────────────

typedef _StrIntToStrC = Pointer<Utf8> Function(Pointer<Utf8>, Int32);
typedef _StrIntToStrDart = Pointer<Utf8> Function(Pointer<Utf8>, int);

typedef _StrToStrC = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _StrToStrDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _FreqC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int32>);
typedef _FreqDart = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Int32>);

typedef _FreeC = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

/// Lazily-loaded handle to the caesar-cipher-native shared library.
final DynamicLibrary _lib = _load();

DynamicLibrary _load() {
  final envPath = Platform.environment['CAESAR_CIPHER_NATIVE_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError(
        'CAESAR_CIPHER_NATIVE_PATH must be an absolute path, got: $envPath',
      );
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) {
    return DynamicLibrary.open('libcaesar_cipher_native.dylib');
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('caesar_cipher_native.dll');
  }
  return DynamicLibrary.open('libcaesar_cipher_native.so');
}

bool _isAbsolute(String p) =>
    p.startsWith('/') || RegExp(r'^[A-Za-z]:[\\/]').hasMatch(p);

// ── Bound functions ──────────────────────────────────────────────────────────

final _encrypt =
    _lib.lookupFunction<_StrIntToStrC, _StrIntToStrDart>('caesar_encrypt');
final _decrypt =
    _lib.lookupFunction<_StrIntToStrC, _StrIntToStrDart>('caesar_decrypt');
final _rot13 = _lib.lookupFunction<_StrToStrC, _StrToStrDart>('caesar_rot13');
final _freq =
    _lib.lookupFunction<_FreqC, _FreqDart>('caesar_frequency_analysis');
final _free = _lib.lookupFunction<_FreeC, _FreeDart>('caesar_free_string');

// ── Marshalling helpers ──────────────────────────────────────────────────────

/// Copy a Rust-owned C string into a Dart String, then free the C allocation.
/// A null return is treated as the empty string.
String _takeString(Pointer<Utf8> ptr) {
  if (ptr == nullptr) return '';
  try {
    return ptr.toDartString();
  } finally {
    _free(ptr);
  }
}

/// Run a `(text, shift) -> String` native call, managing the input allocation.
String callShift(_StrIntToStrDart fn, String text, int shift) {
  final input = text.toNativeUtf8();
  try {
    return _takeString(fn(input, shift));
  } finally {
    malloc.free(input);
  }
}

// ── Public thin wrappers over the raw bindings ───────────────────────────────

String nativeEncrypt(String text, int shift) => callShift(_encrypt, text, shift);

String nativeDecrypt(String text, int shift) => callShift(_decrypt, text, shift);

String nativeRot13(String text) {
  final input = text.toNativeUtf8();
  try {
    return _takeString(_rot13(input));
  } finally {
    malloc.free(input);
  }
}

/// Returns `(shift, plaintext)` from the native frequency-analysis attack.
(int, String) nativeFrequencyAnalysis(String ciphertext) {
  final input = ciphertext.toNativeUtf8();
  final outShift = malloc<Int32>();
  try {
    final plaintext = _takeString(_freq(input, outShift));
    return (outShift.value, plaintext);
  } finally {
    malloc.free(input);
    malloc.free(outShift);
  }
}
