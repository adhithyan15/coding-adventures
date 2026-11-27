# Changelog

## 0.2.0 — Unreleased

### Added
- Generic `BoundedHeap[T]`, `CheckedPriorityQueue[T]` protocol, explicit
  capacity validation, and `HeapFullError`.
- Stateful model, exhaustive-history, deterministic byte-program, comparator,
  ownership, compatibility, and installed-wheel consumer checks.
- Replaced the stateful test dependency with seeded standard-library histories
  that check every model transition and minimize replayable failures.

### Fixed
- Recreate the local virtual environment on every BUILD invocation, pin Python
  3.13 with pip seeded for the installed-wheel check, and run lint, formatting,
  type checking, and tests through that exact environment on Unix and Windows.
- Keep the BUILD test-tool installation aligned with actual test imports; the
  editable install uses `--no-deps` and the model generator uses only the
  standard library.

## 0.1.0 — 2026-04-08

### Added
- `MinHeap` and `MaxHeap` classes backed by a flat array
- `push`, `pop`, `peek`, `is_empty`, `to_array`, `__len__`, `__bool__`
- `MinHeap.from_iterable` / `MaxHeap.from_iterable` — O(n) build via Floyd's algorithm
- Pure functions: `heapify`, `heap_sort`, `nlargest`, `nsmallest`
- Full test suite with heap-property verification after every mutation
