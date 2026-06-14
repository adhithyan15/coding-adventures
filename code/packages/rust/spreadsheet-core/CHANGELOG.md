# Changelog

## 0.1.0

Initial release — Phase 1 headless spreadsheet engine.

- **Cell model** (`cell.rs`, `ast.rs`): literal values (number / text / boolean
  / blank / error) and formulas parsed to an AST.
- **Address grammar** (`address.rs`): A1-style `CellAddress` and ranges with
  bijective column ⇄ letter conversion.
- **Dependency graph + recalc** (`dag.rs`, `recalc.rs`): cells form a directed
  graph of references; `recalc_all` evaluates them in topological order, with
  cycle detection surfaced as an error and error propagation through
  dependents.
- **Dispatch table** (`dispatch.rs`): ~50 functions. Logical (`IF`, `AND`,
  `OR`, `NOT`, `IFERROR`, `IFNA`) and information (`ISBLANK`, `ISERROR`,
  `ISNA`, `ISNUMBER`, `ISTEXT`, …) are inlined; everything else
  (`SUM`, `AVERAGE`, `MIN`/`MAX`, `COUNT`, `STDEV`/`VAR`, `ROUND`, trig,
  `POWER`, `MOD`, …) is delegated to the Layer-1 cores (`statistics-core`,
  `math-core`, `financial-core`, `lookup-core`, `text-core`,
  `datetime-core`).
- **Workbook API** (`workbook.rs`): multi-sheet `Workbook` with
  `add_sheet` / `set_value` / `set_formula` / `get_value` / `recalc_all`,
  a recalc `epoch`, and bidirectional sheet name ⇄ id lookup
  (`sheet_id` / `sheet_name`).
- `#![forbid(unsafe_code)]`; 90+ unit tests plus a doctest.

### Hardening against adversarial formulas

Because this engine will be fed untrusted, UI-typed formulas (and compiled to
WASM / a C ABI), a security review of the adopted code surfaced four
denial-of-service vectors, all now fixed and regression-tested:

- **Oversized ranges** (`address.rs`, `recalc.rs`): a single formula like
  `=SUM(A1:XFD1048576)` named ~17 billion cells. Range cardinality is now
  computed in `u64` (no `usize` overflow on wasm32) and capped at
  `MAX_RANGE_CELLS` (2²⁰); an oversized range surfaces `#REF!` instead of
  expanding into the dependency graph or argument vector.
- **Parser recursion** (`parser.rs`): deeply nested input (`=((((…))))`,
  stacked unary) could overflow the native stack. The recursive-descent
  parser now enforces `MAX_PARSE_DEPTH` (256), returning `ParseError::TooDeep`.
- **Quadratic recalc** (`workbook.rs`): `evaluate_cell` cloned every cell of
  every sheet on each call (O(N²) to recalc N cells). It now evaluates against
  a read-only borrow of the cell storage — linear, no per-cell allocation.
- **Recursive cycle detection** (`dag.rs`): Tarjan's SCC search was recursive,
  so a long dependency chain (thousands of cells deep) overflowed the stack.
  Rewritten with an explicit heap work-stack.

This crate originated as the stalled Phase 1 PR (#3378), authored before its
Layer-1 dependencies all existed on `main`. Brought current: added to the
Rust workspace, cleaned to zero clippy warnings (removed a redundant closure
and a `let`-and-return; the previously-dead `Sheet::name` field is now read by
the new `sheet_name` accessor), and documented (this changelog + README).
