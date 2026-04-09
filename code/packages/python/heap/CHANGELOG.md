# Changelog

## 0.1.0 — 2026-04-08

### Added
- `MinHeap` and `MaxHeap` classes backed by a flat array
- `push`, `pop`, `peek`, `is_empty`, `to_array`, `__len__`, `__bool__`
- `MinHeap.from_iterable` / `MaxHeap.from_iterable` — O(n) build via Floyd's algorithm
- Pure functions: `heapify`, `heap_sort`, `nlargest`, `nsmallest`
- Full test suite with heap-property verification after every mutation
