# Changelog — coding_adventures_heap_native

## 0.1.0 — 2026-07-11

### Added

- Initial release: native-through-Rust Dart bindings for i64 binary heaps,
  introducing a new native shape — an opaque handle whose pop/peek *return
  values* via a bool-return + out-parameter convention.
- Rust `cdylib` (`src/lib.rs`): macro-generated C ABI for `MinHeap<i64>` and
  `MaxHeap<i64>` (new/push/pop/peek/len/is_empty/free) plus `heap_sort_i64`,
  `heap_nlargest_i64`, `heap_nsmallest_i64` over caller-owned i64 buffers.
- Dart FFI layer with `HEAP_NATIVE_PATH` loading (absolute-path validated), a
  per-flavour vtable, and a NativeFinalizer-backed handle wrapper with eager
  `dispose()`.
- Public API: `MinHeap` / `MaxHeap` (push/pop/peek/length/isEmpty/dispose) and
  `heapSort` / `nLargest` / `nSmallest` over `List<int>`.
- `tools/run-tests.sh` builds the release cdylib and runs the suite. 5 Rust ABI
  unit tests + 10 Dart tests through FFI (ascending/descending pops, peek/empty,
  negatives/duplicates, disposed-handle safety, array-algorithm parity).
- Windows CI skipped (cdylib cross-compile out of scope); Linux and macOS build.
