## 0.5.0 — real allocation measurement (LH-003a)

### Changed (breaking)

- `bench::AllocCounter` no longer counts what a closure *records*; it reads the
  calling thread's tally from a counting global allocator. `record_alloc`,
  `total_bytes` and `total_count` are removed. The old counter made every
  `max_alloc_bytes = 0` budget pass vacuously (ct-compare's zero-allocation
  evidence was meaningless).
- `AllocCounter::sample` returns `NotLive` when the counting allocator is not
  installed, so the harness keeps `alloc_bytes` in `unchecked_hard_budgets`
  instead of reporting a fabricated zero.

### Added

- `counting_alloc` module: `CountingAllocator` (a `GlobalAlloc` that only
  delegates to `System` and tallies allocations, frees and bytes in a
  `const`-initialised thread-local, so no measurement lock is needed),
  `AllocStats`, `thread_stats`, `is_installed` (a probe that leaves the tally
  untouched) and `measure`.
- `install_counting_allocator!()` macro for test/bench binaries.
- `AllocCounter::stats()` exposing allocation and free counts.
- `tests/alloc_budget.rs`: sabotage (`Vec::with_capacity(1)` fails a zero
  budget; an allocation hidden in a callee is seen; exact boundary at 8/9
  bytes) and controls (a non-allocating closure passes with 0.0 B/iter).
- `bench_statistics.rs` now proves the uninstalled case is `NotLive`/unchecked.

### Spec

- VER07 §8.2 rewritten: the "cooperative" limit is replaced by the real-heap
  contract and its remaining limits.
