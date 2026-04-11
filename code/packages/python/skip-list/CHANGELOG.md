# Changelog

All notable changes to `coding-adventures-skip-list` will be documented here.

## [0.1.0] - 2026-04-08

### Added

- Initial implementation of `SkipList` probabilistic sorted data structure
- Multi-level linked list with sentinel head (+inf key) and tail (-inf key)
- Span-augmented forward pointers for O(log n) rank queries (Redis ZRANK style)
- `insert(key, value)` — O(log n) expected; updates value if key exists
- `delete(key)` — O(log n) expected; returns bool indicating presence
- `search(key)` — O(log n) expected; returns associated value or None
- `contains(key)` — O(log n) expected; handles None-valued keys correctly
- `rank(key)` — O(log n) 0-based rank using accumulated span traversal
- `by_rank(rank)` — O(log n) key lookup by 0-based rank
- `range_query(lo, hi, inclusive)` — O(log n + k) range scan at level 1
- `__len__`, `__contains__`, `__iter__`, `__repr__` Python protocols
- Configurable `max_level` (default 16) and `p` (default 0.5) parameters
- `_NegInf` / `_PosInf` sentinel objects with full comparison operator coverage
- 82 unit tests covering empty list, single element, bulk operations, stress
  tests with 10,000 elements, rank/by_rank round-trips, and sentinel internals
- 99.45% line coverage (only unreachable defensive fallback excluded)
- Ruff linting passes cleanly

### Notes

- Bug found and fixed during development: `_find_predecessors` was updating
  `rank_so_far[level]` instead of the shared `cumulative_rank` variable inside
  the while loop, causing all rank computations to return 0 regardless of
  actual position. The fix ensures `cumulative_rank` is incremented on each hop
  so that the value carried to lower levels correctly represents all hops taken
  so far.
