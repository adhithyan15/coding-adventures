# Changelog — coding_adventures_heap

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Dart port of the `heap` reference package.
- `MinHeap<T>` / `MaxHeap<T>` — array-backed binary heaps with `push`, `pop`,
  `peek`, `isEmpty`, `length`, `toList`, and an O(n) `fromIterable` constructor.
- Companion algorithms: `heapify`, `heapSort`, `nLargest`, `nSmallest`
  (the last two use a bounded heap of size n, O(m log n)).
- Ordering defaults to natural `Comparable` order; an optional `Comparator<T>`
  orders by any key.
- 22 unit tests: ascending/descending pop order, peek/empty behaviour,
  duplicates, heap-property validation of `fromIterable`, string and
  custom-comparator ordering, and companion-algorithm correctness against manual
  sorts.
