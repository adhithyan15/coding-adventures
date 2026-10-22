### Added

- Opt-in per-component projection counters for reads, created and retained rows,
  and removals. Nested projections accumulate on their owner. The read-only
  snapshot is UI-thread diagnostic data; it is not a native realization count.
