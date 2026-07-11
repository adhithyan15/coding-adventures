/// Binary heaps — **native-through-Rust** Dart bindings (`int`/`i64` elements).
///
/// Same shape as the pure-Dart `coding_adventures_heap` package, but backed by
/// the Rust `heap` crate through a C ABI (`dart:ffi`). Because a C ABI cannot be
/// generic, the element type is fixed to [int] (`i64`) — the common case for a
/// native priority queue.
///
/// ```dart
/// import 'package:coding_adventures_heap_native/coding_adventures_heap_native.dart';
///
/// void main() {
///   final h = MinHeap()..push(5)..push(1)..push(3);
///   print(h.pop()); // 1  (computed in Rust)
///   h.dispose();
///
///   print(heapSort([3, 1, 2]));          // [1, 2, 3]
///   print(nLargest([5, 1, 4, 2, 3], 2)); // [5, 4]
/// }
/// ```
///
/// The shared library is located via `HEAP_NATIVE_PATH` (absolute) or the
/// platform default name; `tools/run-tests.sh` sets it before running tests.
library coding_adventures_heap_native;

import 'src/ffi.dart' as ffi;

/// A native `i64` min-heap (smallest element at the root).
///
/// The underlying handle is freed automatically on garbage collection; call
/// [dispose] to release it eagerly. Using a disposed heap throws [StateError].
class MinHeap {
  final ffi.NativeHeapHandle _h;

  /// Create an empty native min-heap.
  MinHeap() : _h = ffi.NativeHeapHandle(ffi.minVtable);

  /// Insert [value].
  void push(int value) => _h.push(value);

  /// Remove and return the smallest element, or `null` if empty.
  int? pop() => _h.pop();

  /// The smallest element without removing it, or `null` if empty.
  int? peek() => _h.peek();

  /// The number of elements.
  int get length => _h.length;

  /// True when the heap has no elements.
  bool get isEmpty => _h.isEmpty;

  /// Free the native handle eagerly. Idempotent.
  void dispose() => _h.dispose();
}

/// A native `i64` max-heap (largest element at the root). See [MinHeap].
class MaxHeap {
  final ffi.NativeHeapHandle _h;

  /// Create an empty native max-heap.
  MaxHeap() : _h = ffi.NativeHeapHandle(ffi.maxVtable);

  /// Insert [value].
  void push(int value) => _h.push(value);

  /// Remove and return the largest element, or `null` if empty.
  int? pop() => _h.pop();

  /// The largest element without removing it, or `null` if empty.
  int? peek() => _h.peek();

  /// The number of elements.
  int get length => _h.length;

  /// True when the heap has no elements.
  bool get isEmpty => _h.isEmpty;

  /// Free the native handle eagerly. Idempotent.
  void dispose() => _h.dispose();
}

/// Sort [data] ascending (heapsort, executed in Rust).
List<int> heapSort(List<int> data) => ffi.nativeHeapSort(data);

/// Return the [n] largest elements of [data], descending (executed in Rust).
List<int> nLargest(List<int> data, int n) => ffi.nativeNLargest(data, n);

/// Return the [n] smallest elements of [data], ascending (executed in Rust).
List<int> nSmallest(List<int> data, int n) => ffi.nativeNSmallest(data, n);
