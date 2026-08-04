# coding_adventures_heap_native

**Native-through-Rust** Dart bindings for binary heaps and heap algorithms,
backed by the Rust `heap` crate through a C ABI (`dart:ffi`).

Because a C ABI cannot be generic, the element type is fixed to `int` (`i64`) —
the common case for a native priority queue. (The pure-Dart
[`coding_adventures_heap`](../heap) package is generic over any `Comparable`.)

## A new native shape: handles that return values

The hash bindings' opaque handles only ever *wrote a digest*; here the handle's
`pop`/`peek` **return a value**, via a `bool`-return + out-parameter convention
(`false` = empty):

```c
HANDLE heap_min_new(void);
void   heap_min_push(HANDLE, int64_t);
bool   heap_min_pop(HANDLE, int64_t* out);   // false if empty
bool   heap_min_peek(HANDLE, int64_t* out);
size_t heap_min_len(HANDLE);
bool   heap_min_is_empty(HANDLE);
void   heap_min_free(HANDLE);
/* …_max_… mirror … */
void   heap_sort_i64(const int64_t*, size_t, int64_t* out);
size_t heap_nlargest_i64(const int64_t*, size_t, size_t n, int64_t* out);
size_t heap_nsmallest_i64(const int64_t*, size_t, size_t n, int64_t* out);
```

The Dart `MinHeap`/`MaxHeap` wrappers manage the handle with a `NativeFinalizer`
(auto-free on GC) plus an eager `dispose()`.

## Usage

```dart
import 'package:coding_adventures_heap_native/coding_adventures_heap_native.dart';

void main() {
  final h = MinHeap()..push(5)..push(1)..push(3);
  print(h.pop()); // 1  (computed in Rust)
  h.dispose();

  print(heapSort([3, 1, 2]));          // [1, 2, 3]
  print(nLargest([5, 1, 4, 2, 3], 2)); // [5, 4]
  print(nSmallest([5, 1, 4, 2, 3], 2));// [1, 2]
}
```

## Building and testing

```
sh tools/run-tests.sh
```

builds the cdylib, sets `HEAP_NATIVE_PATH`, and runs `dart test`. Windows CI is
skipped (cdylib cross-compile out of scope); Linux and macOS build and test.
