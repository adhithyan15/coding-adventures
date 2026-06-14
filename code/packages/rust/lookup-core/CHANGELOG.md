# Changelog

All notable changes to `lookup-core` are recorded here.  Format follows
Keep a Changelog; the project uses semantic versioning starting at `0.1.0`.

## [0.1.0] — Phase 1

### Added
- `LookupValue` frontend-agnostic value enum (`Empty` / `Boolean` /
  `Number` / `Text`) with NA encoded via `r_vector::na_real`.
- `LookupError` enum with `Display` + `std::error::Error` impls.
- `vlookup` and `hlookup` with exact and approximate (binary-search) modes.
- `index_1d` and `index_2d` (1-based; supports whole-row/whole-column
  results when `row=0` or `col=0`).
- `match` (`Exact` / `LessOrEqual` / `GreaterOrEqual`).
- `xlookup` and `xmatch` covering all Excel `match_mode` × `search_mode`
  combinations, including wildcard match and `if_not_found` fallback.
- `offset` for range arithmetic over a 2-D table.
- `choose` variadic pick.
- `row` / `column` / `rows` / `columns` shape-introspection helpers.
- Centralised Excel-compatible equality, ordering, and `?`/`*`/`~`
  wildcard matchers.
- Unit tests in every module plus integration tests in
  `tests/lookup_tests.rs`.
