# Changelog

## 0.5.0

**Cell display formats.** Cells can now carry an Excel-style format code, and a
new accessor returns the formatted display string — wiring `number-format-core`
into the engine's display path.

- Per-sheet `formats` store (independent of cell content, so a cell can be
  formatted while empty and the format survives content edits).
- `Workbook::set_format(sheet, addr, code)` / `clear_format` / `get_format`.
- `Workbook::get_display(sheet, addr) -> String`: the computed value run through
  its format code (or `General`) — numbers formatted per the code (grouping,
  decimals, percent, dates via the date/time codes), text/booleans/errors render
  naturally, empty → `""`. The one call a renderer needs per visible cell.
- Structural edits relocate the format store alongside cells: a format rides
  with the cell it decorates on insert/delete, is dropped when its cell is
  deleted, and the off-grid-overflow guard now also covers format-only entries.
- New `number-format-core` dependency (a Layer-1 core, like the math cores the
  formula engine already dispatches to). 5 new tests.

## 0.4.0

**Insert/delete rows & columns** (`workbook.rs`) — wires the structural-edit
reference arithmetic from 0.3.0's `edit.rs` into the live `Workbook`, plus a
formula serializer to keep echo text honest.

- `Workbook::insert_rows` / `delete_rows` / `insert_cols` / `delete_cols(sheet,
  at, count)`: relocate every cell (a cell on a deleted line is removed),
  rewrite each formula's references via `FormulaAst::adjust` (a reference to a
  deleted line becomes `#REF!`), rebuild the dependency graph (every address
  moved, so the old edges are stale), and recalculate. One revision transaction;
  surviving cells are logged so a viewport `changed_since` snapshot sees the
  relocation. Unknown `sheet` → no-op.
- `FormulaAst::to_formula_string()`: render an AST back to a formula string,
  fully parenthesising binary operators so it always re-parses to an equivalent
  tree. Used to refresh a cell's stored source after a structural edit rewrites
  its references; independently useful for echo-back / save-load / copy-paste.
  Plus `BinaryOp::symbol()` (the source token for an operator).
- v1 scope: single-sheet (the engine's formula references are sheet-local) and a
  full rebuild + recalc sweep (correct and simple). Cross-sheet reference
  adjustment and incremental recalc are future optimisations.
- 8 new tests (6 `Workbook` edits — relocate + rewrite, survivor shift, deleted
  reference → `#REF!`, column shift, revision bump, unknown-sheet no-op; 2
  serializer round-trip/literal tests).

## 0.3.0

**Structural-edit reference arithmetic** (`edit.rs`) — the pure substrate of the
insert/delete rows & columns feature. Mutates no workbook state; a later layer
wires these transforms into `Workbook` (relocate cells, rewrite each formula's
AST, rebuild the dependency graph, recalc).

- `StructuralEdit`: `InsertRows` / `DeleteRows` / `InsertCols` / `DeleteCols`,
  each `{ at, count }` (1-based).
- `CellAddress::adjust(edit) -> Option<CellAddress>`: where an address moves, or
  `None` if it sat on a deleted line (→ `#REF!`). Structural edits shift **both**
  relative and absolute references (`$A$1` → `$A$2` when a row is inserted
  above) — absolute flags are preserved, not exempted; that's distinct from
  `CellAddress::shift`'s copy/paste semantics.
- `CellRange::adjust(edit) -> Option<CellRange>`: grow on interior insert, move
  on insert-before, shrink/clamp on partial delete, `None` when the whole range
  is deleted. Absolute corner flags preserved.
- `FormulaAst::adjust(edit) -> FormulaAst`: pure recursive rewrite of every
  `Ref`/`Range`; deleted references collapse to the `#REF!` error literal, which
  then propagates through evaluation like any error.
- 18 unit tests covering before/at/in-band/after for rows and columns, absolute
  shifting, range grow/move/shrink/destroy, nested-AST recursion, and the
  zero-count identity edit.

## 0.2.0

**Viewport primitive for the virtualized infinite sheet** (`viewport.rs`,
`workbook.rs`). The sheet was already unbounded (u32 addresses, sparse storage);
these reads let a host render only the visible window of it.

- `Workbook::get_window(sheet, row0, col0, row1, col1) -> Window`: a dense,
  row-major rectangle of computed values (empty cells included as
  `CellValue::Empty`), `O(window)` not `O(sheet)`. Rejects inverted rectangles
  and windows over `MAX_WINDOW_CELLS` (65 536 — a screen-scale safety cap), with
  overflow-safe size checking.
- `Workbook::used_range(sheet) -> Option<UsedRange>`: bounding box of
  materialised non-empty cells, for scrollbar sizing.
- `Workbook::current_revision()` + `Workbook::changed_since(sheet, since) ->
  ChangeSet`: a per-edit revision clock (advances on every `set_value` /
  `set_formula` / `clear_cell`, unlike `epoch` which only advances on
  `recalc_all`) plus a bounded change log, so a host re-fetches only the cells
  dirtied since its last render. Returns `ChangeSet::Stale` (re-read everything)
  when a query reaches back before the retained log window (`CHANGELOG_RETAIN`).
  v1 stamps on write (a no-op recompute may over-report a cell — safe; exact
  old/new diffing is a future optimisation).
- Re-exported `column_index_to_letters` / `column_letters_to_index` for hosts to
  render identical A…Z, AA… headers without re-implementing the math.

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
