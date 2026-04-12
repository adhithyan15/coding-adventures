# Changelog

All notable changes to `go/b-tree` are documented here.

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of a generic B-tree (`BTree[K, V]`) — DT11.
- Minimum degree `t` is configurable at construction time; any `t ≥ 2` is valid.
- Key ordering is user-supplied via a `less func(K, K) bool`, making the tree
  work with any totally-ordered type (int, string, custom structs, etc.).
- **Operations**: `Insert`, `Delete`, `Search`, `Contains`, `MinKey`, `MaxKey`,
  `Inorder`, `RangeQuery`, `Len`, `Height`, `IsValid`.
- Proactive top-down splitting during insertion — single downward pass, no
  backtracking.
- Complete deletion with all sub-cases:
  - Case 1: key in leaf — direct removal.
  - Case 2a: key in internal node, left child rich — predecessor replacement.
  - Case 2b: key in internal node, right child rich — successor replacement.
  - Case 2c: both children at minimum — merge then recurse.
  - Case 3: deficient child filled via left-rotate, right-rotate, or merge.
- `IsValid()` validates all B-tree structural invariants (key count bounds,
  sorted keys, uniform leaf depth, correct child counts, size consistency).
- Literate programming style: ASCII diagrams, invariant proofs, and step-by-step
  explanations in every function.
- Test suite with 95%+ coverage:
  - All delete sub-cases tested individually.
  - Random insert/delete stress test with `t ∈ {2, 3, 5}`.
  - Bulk test with 10,000+ keys.
  - `RangeQuery` correctness verified against linear scan.
