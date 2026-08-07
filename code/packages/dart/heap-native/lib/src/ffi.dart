// ffi.dart — dart:ffi bindings for the heap-native Rust cdylib (i64 heaps).
//
// The pure crate is generic; the C ABI fixes the element type to i64. Two
// opaque handle types (MinHeap<i64>, MaxHeap<i64>) with a bool-return +
// out-parameter convention for pop/peek, plus three i64-array algorithms.
//
//   HANDLE heap_min_new(void); heap_max_new(void);
//   void   heap_{min,max}_push(HANDLE, int64_t);
//   bool   heap_{min,max}_pop(HANDLE, int64_t* out);   // false if empty
//   bool   heap_{min,max}_peek(HANDLE, int64_t* out);
//   size_t heap_{min,max}_len(HANDLE);
//   bool   heap_{min,max}_is_empty(HANDLE);
//   void   heap_{min,max}_free(HANDLE);
//   void   heap_sort_i64(const int64_t*, size_t, int64_t* out);
//   size_t heap_nlargest_i64(const int64_t*, size_t, size_t n, int64_t* out);
//   size_t heap_nsmallest_i64(const int64_t*, size_t, size_t n, int64_t* out);

import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

typedef _NewC = Pointer<Void> Function();
typedef _NewDart = Pointer<Void> Function();
typedef _PushC = Void Function(Pointer<Void>, Int64);
typedef _PushDart = void Function(Pointer<Void>, int);
typedef _PopC = Bool Function(Pointer<Void>, Pointer<Int64>);
typedef _PopDart = bool Function(Pointer<Void>, Pointer<Int64>);
typedef _LenC = Size Function(Pointer<Void>);
typedef _LenDart = int Function(Pointer<Void>);
typedef _EmptyC = Bool Function(Pointer<Void>);
typedef _EmptyDart = bool Function(Pointer<Void>);
typedef _FreeC = Void Function(Pointer<Void>);
typedef _FreeDart = void Function(Pointer<Void>);
typedef _SortC = Void Function(Pointer<Int64>, Size, Pointer<Int64>);
typedef _SortDart = void Function(Pointer<Int64>, int, Pointer<Int64>);
typedef _NkC = Size Function(Pointer<Int64>, Size, Size, Pointer<Int64>);
typedef _NkDart = int Function(Pointer<Int64>, int, int, Pointer<Int64>);

final DynamicLibrary _lib = _load();

DynamicLibrary _load() {
  final envPath = Platform.environment['HEAP_NATIVE_PATH'];
  if (envPath != null && envPath.isNotEmpty) {
    if (!_isAbsolute(envPath)) {
      throw ArgumentError('HEAP_NATIVE_PATH must be an absolute path, got: $envPath');
    }
    return DynamicLibrary.open(envPath);
  }
  if (Platform.isMacOS) return DynamicLibrary.open('libheap_native.dylib');
  if (Platform.isWindows) return DynamicLibrary.open('heap_native.dll');
  return DynamicLibrary.open('libheap_native.so');
}

bool _isAbsolute(String p) =>
    p.startsWith('/') || RegExp(r'^[A-Za-z]:[\\/]').hasMatch(p);

/// The bound C functions for one concrete heap flavour (min or max).
class HeapVtable {
  final _NewDart create;
  final _PushDart push;
  final _PopDart pop;
  final _PopDart peek;
  final _LenDart len;
  final _EmptyDart isEmpty;
  final _FreeDart free;
  final NativeFinalizer finalizer;
  HeapVtable(String prefix)
      : create = _lib.lookupFunction<_NewC, _NewDart>('${prefix}_new'),
        push = _lib.lookupFunction<_PushC, _PushDart>('${prefix}_push'),
        pop = _lib.lookupFunction<_PopC, _PopDart>('${prefix}_pop'),
        peek = _lib.lookupFunction<_PopC, _PopDart>('${prefix}_peek'),
        len = _lib.lookupFunction<_LenC, _LenDart>('${prefix}_len'),
        isEmpty = _lib.lookupFunction<_EmptyC, _EmptyDart>('${prefix}_is_empty'),
        free = _lib.lookupFunction<_FreeC, _FreeDart>('${prefix}_free'),
        finalizer = NativeFinalizer(
            _lib.lookup<NativeFunction<Void Function(Pointer<Void>)>>('${prefix}_free'));
}

final HeapVtable minVtable = HeapVtable('heap_min');
final HeapVtable maxVtable = HeapVtable('heap_max');

final _sort = _lib.lookupFunction<_SortC, _SortDart>('heap_sort_i64');
final _nlargest = _lib.lookupFunction<_NkC, _NkDart>('heap_nlargest_i64');
final _nsmallest = _lib.lookupFunction<_NkC, _NkDart>('heap_nsmallest_i64');

/// Owns a native heap handle of one flavour, freed via a [NativeFinalizer] or
/// eagerly via [dispose].
class NativeHeapHandle implements Finalizable {
  final HeapVtable _v;
  Pointer<Void> _handle;
  bool _disposed = false;

  NativeHeapHandle(this._v) : _handle = _v.create() {
    _v.finalizer.attach(this, _handle, detach: this);
  }

  void _checkAlive() {
    if (_disposed) throw StateError('heap has been disposed');
  }

  void push(int value) {
    _checkAlive();
    _v.push(_handle, value);
  }

  int? pop() {
    _checkAlive();
    final out = malloc<Int64>();
    try {
      return _v.pop(_handle, out) ? out.value : null;
    } finally {
      malloc.free(out);
    }
  }

  int? peek() {
    _checkAlive();
    final out = malloc<Int64>();
    try {
      return _v.peek(_handle, out) ? out.value : null;
    } finally {
      malloc.free(out);
    }
  }

  int get length {
    _checkAlive();
    return _v.len(_handle);
  }

  bool get isEmpty {
    _checkAlive();
    return _v.isEmpty(_handle);
  }

  void dispose() {
    if (_disposed) return;
    _disposed = true;
    _v.finalizer.detach(this);
    _v.free(_handle);
    _handle = nullptr;
  }
}

// ── Array algorithms over List<int> ──────────────────────────────────────────

Pointer<Int64> _toNative(List<int> data) {
  if (data.isEmpty) return nullptr;
  final ptr = malloc<Int64>(data.length);
  ptr.asTypedList(data.length).setAll(0, data);
  return ptr;
}

List<int> nativeHeapSort(List<int> data) {
  if (data.isEmpty) return <int>[];
  final input = _toNative(data);
  final out = malloc<Int64>(data.length);
  try {
    _sort(input, data.length, out);
    return List<int>.of(out.asTypedList(data.length));
  } finally {
    malloc.free(input);
    malloc.free(out);
  }
}

List<int> _nk(_NkDart fn, List<int> data, int n) {
  if (n <= 0 || data.isEmpty) return <int>[];
  final cap = n < data.length ? n : data.length;
  final input = _toNative(data);
  final out = malloc<Int64>(cap);
  try {
    final k = fn(input, data.length, n, out);
    return List<int>.of(out.asTypedList(k));
  } finally {
    malloc.free(input);
    malloc.free(out);
  }
}

List<int> nativeNLargest(List<int> data, int n) => _nk(_nlargest, data, n);
List<int> nativeNSmallest(List<int> data, int n) => _nk(_nsmallest, data, n);
