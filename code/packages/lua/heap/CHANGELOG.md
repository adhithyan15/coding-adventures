# Changelog — coding-adventures-heap (Lua)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-12

### Added

- Initial implementation of `coding_adventures.heap`.
- Generic comparator-based `MinHeap` and `MaxHeap` binary heaps.
- `new(compare)` and `from_iterable(items, compare)` constructors.
- `push`, `pop`, `peek`, `len`, `size`, `is_empty`, and `to_array` operations.
- Busted tests covering min-heap ordering, max-heap ordering, heapify, custom
  comparators, and empty-heap behavior.
