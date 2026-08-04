# Changelog — FenwickTree (Swift)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Swift port of the `fenwick-tree` reference package — the
  fourth language (after Dart, Java, Kotlin) in the pure-port + native campaign.
- `FenwickTree` over `Double` with `update`, `prefixSum`, `rangeSum`,
  `pointQuery`, and `findKth` (order-statistic search), plus `count`, `isEmpty`,
  `bitArray`, and a `CustomStringConvertible` description.
- Two initialisers: `init(size:)` (all-zero) and `init(values:)` (O(n) build).
- Fallible operations `throw FenwickError` (indexOutOfRange / invalidRange /
  emptyTree / nonPositiveTarget / targetExceedsTotal) — the idiomatic Swift
  equivalent of the reference's `Result`.
- 11 XCTest cases: reference prefix/range/point examples, updates (incl.
  negative delta), find_kth order statistics and its error cases, invalid-index
  and invalid-range errors, introspection, and a brute-force prefix/range
  cross-check.
