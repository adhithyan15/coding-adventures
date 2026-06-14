# Changelog

All notable changes to `array-core` are documented in this file. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-16

### Added

- Initial Phase 1 implementation per `code/specs/backend-crate-catalog.md`'s
  `array-core` row.
- `Array2D<T>` 2-D row-major wrapper with `new`, `from_vector`, `filled`,
  `get`, `set`, `row`, `col`, `is_empty`, and `is_na` (f64 specialization).
- `ArrayError` enum (`ShapeMismatch`, `EmptyResult`, `BadParameter`,
  `OutOfRange`) with `Display` and `std::error::Error` impls.
- `generate::sequence` — Excel 365 `SEQUENCE(rows, cols, start, step)`.
- `shape::take` / `shape::drop` / `shape::expand` — Excel 365 `TAKE`, `DROP`,
  `EXPAND` with Excel-style negative-count "from the end" semantics and NA
  padding for `EXPAND`.
- `stack::hstack` / `stack::vstack` — Excel 365 `HSTACK` / `VSTACK` with NA
  padding for mismatched shapes.
- `reshape::to_row` / `reshape::to_col` — Excel 365 `TOROW` / `TOCOL` with the
  4-mode `ignore` enum (`KeepAll`, `SkipBlanks`, `SkipErrors`, `SkipBoth`) and
  optional column-major scan order.
- `reshape::wrap_rows` / `reshape::wrap_cols` — Excel 365 `WRAPROWS` /
  `WRAPCOLS` with caller-supplied or NA padding.
- `pick::choose_rows` / `pick::choose_cols` — Excel 365 `CHOOSEROWS` /
  `CHOOSECOLS` with 1-based indices, negative-from-end, repeats allowed.
- `filter::filter` — Excel 365 `FILTER` with 1-D boolean mask, `if_empty`
  fallback, and NA-as-FALSE mask semantics.
- `sort::sort` / `sort::sort_by` — Excel 365 `SORT` / `SORTBY` with stable
  ordering, NA-last-in-ascending convention, optional `by_col` axis, and
  multi-key support (capped at 6 keys for Phase 1; Excel allows 128).
- `unique::unique` — Excel 365 `UNIQUE` with `by_col` and `exactly_once`
  options. NA values dedupe against each other.
- Integration test suite (~50 tests) covering every function, NA propagation,
  Excel edge cases, and error paths.

### Notes / divergences from Excel 365

- `TOROW` / `TOCOL` modes 2 (skip errors) and 3 (skip both) currently behave
  identically to 0 / 1 respectively because `Array2D<f64>` has no error
  encoding distinct from NA. Phase 2 (mixed text/error arrays) will revisit.
- `SORTBY` is capped at 6 keys (Excel: 128). v1 callers should not hit this.
- `RANDARRAY` is intentionally deferred — it routes through a separate RNG
  crate per the catalog.
