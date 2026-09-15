### Added — SQL Auto-Index: Composite Multi-Column Index (IX-8)
- **`IndexScan.columns: tuple[str, ...]`** in `sql-planner` — replaces
  `column: str`; single-column scans produce a 1-tuple, composite scans an
  n-tuple matching the leading prefix of the index used.
- **Multi-column bounds** — `IndexScan.lo` / `IndexScan.hi` widened to
  `tuple[object, ...] | None`; `OpenIndexScan` in `sql-codegen` and the VM
  decode them with `list(ins.lo)` for prefix-key comparison in the backend.
- **`_extract_multi_column_bounds`** planner helper — chains `_extract_index_bounds`
  across consecutive index columns; EQ extends the chain, range terminates it.
- **Best-match index selection** — `_try_index_scan` evaluates all indexes and
  picks the one covering the most predicate columns.
- **`IndexAdvisor` pair tracking** — `_pair_hits` accumulates `(table, col_a, col_b)`
  pairs from full-table scans; `_maybe_create_composite_index` creates a
  two-column index when the policy threshold is reached, skipping redundant
  creation when a leading-column single index already exists.
- **`_auto_index_meta`** — maps auto-index names → `(table, columns_tuple)` for
  correct drop-loop bookkeeping without name parsing.
- **21 new tests** in `mini-sqlite/tests/test_tier3_composite.py` covering
  advisor pair logic, planner composite selection, and end-to-end integration.

