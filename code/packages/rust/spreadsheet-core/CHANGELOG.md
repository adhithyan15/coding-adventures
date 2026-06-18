# Changelog

## 0.9.0

**Persistence — serialize / deserialize (save / load).** `Workbook::serialize`
captures the whole workbook as a portable JSON string; `Workbook::deserialize`
rebuilds a workbook from one. The engine side of save/load — no I/O happens here,
the caller supplies/keeps the bytes.

- `Workbook::serialize() -> String` (`workbook.rs`): per sheet, every **source**
  cell (a formula's text, or a literal's typed value) + every **format**
  (including formats on otherwise-empty cells, which outlive content). Computed
  values are deliberately NOT stored — `deserialize` recomputes them, so the file
  is small and can't disagree with the engine. Cells and formats are sorted by
  (row, col), so output is stable (byte-identical for equal workbooks — handy for
  diffing and tests). Shape, version 1:
  `{"version":1,"sheets":[{"name":..,"cells":[{"a1":"A1","value":{"number":15.0}},
  {"a1":"E1","formula":"=SUM(A1:D1)"}],"formats":[{"a1":"E1","code":"#,##0.00"}]}]}`.
  A literal value is one of `{"number":n}` / `{"text":s}` / `{"bool":b}` /
  `{"error":"#REF!"}`; a non-finite number degrades to `#NUM!` (JSON can't hold
  NaN/∞), matching how it reads in the grid.
- `Workbook::deserialize(&str) -> Result<(), String>`: validates the JSON +
  `version` + `sheets` array BEFORE mutating (a bad file leaves the workbook
  untouched), then clears sheets/graph/clipboard/changelog and rebuilds in file
  order (so a single-sheet host keeps `SheetId(0)`), and `recalc_all` repopulates
  caches (revision bumps once). A stored formula that no longer parses is kept as
  its literal text rather than dropped — no user input is silently lost. Errs on
  malformed JSON, unsupported version, missing `sheets`, or a bad cell address.
- New dependency: `serde_json` (the engine had none; serialization is pure
  String↔state, no I/O, `forbid(unsafe_code)` intact). 4 round-trip tests
  (values+formulas+formats incl. empty-cell format + stable re-serialize;
  replace-not-merge; reject bad JSON/version/missing-sheets; keep-bad-formula-as-text).
  Spec §16 (Persistence) rewritten from "out of scope" to this JSON format.

## 0.8.0

**Clipboard — cut / copy / paste.** A stateful clipboard on `Workbook`, layered
over the same `FormulaAst::shift` reference arithmetic that powers fill. Where
fill replicates one source cell across a range (each target shifts by its own
offset), copy/paste captures a whole **rectangle** and shifts it as a unit on
paste — the block's internal structure is preserved (`=A1` copied two columns
right pastes as `=C1`).

- `Workbook::copy(sheet, range)` / `cut(sheet, range)` (`workbook.rs`): snapshot
  the non-blank cells of `range` (content **and** format) as offsets from the
  range's top-left anchor. A copy survives any number of pastes; a cut is a
  one-shot move (the buffer is consumed on the paste that places it). The source
  is captured but not cleared until paste — a cut with no paste is a no-op
  (spreadsheet "marching-ants" semantics). A `range` over `MAX_RANGE_CELLS` is
  rejected (the same DoS guard fill/formula ranges use); an unknown sheet too.
- `Workbook::paste(sheet, dst_anchor) -> bool`: place the block so its top-left
  lands at `dst_anchor`. The whole block's references shift by
  `dst_anchor − anchor` via `FormulaAst::shift` (relative refs track, absolute
  `$` refs pin, off-grid → `#REF!`); content + format ride along. Every cell of
  the destination rectangle is written, so blanks in the source **erase** their
  targets. A cut then clears the source cells it didn't overwrite and consumes
  the buffer. Returns `false` (a no-op) for an empty clipboard, unknown sheet, or
  a destination rectangle that would run past the u32 grid edge — never silently
  truncates or wraps. The block-shift delta is computed in `i64` then clamped
  into `shift`'s `i32` contract, so a paste anchored at a high coordinate can't
  overflow. `has_clipboard()` reports whether a block is held.
- Known divergence (documented in code + spec): a **cut** shifts the moved
  formulas' own references like a copy, rather than preserving them as Excel's
  move does (Excel additionally rewrites outside references that pointed into the
  moved range). This keeps cut a thin layer over the copy machinery.
- 8 new tests: whole-block ref-shift, format-carry + absolute-pin, blank-cell
  erase, cut-move-and-clear (+ one-shot), off-grid reject (buffer kept),
  oversized-range reject, empty-clipboard no-op, unknown-sheet no-op.

## 0.7.0

**Fill / replicate (drag-fill)** — `Workbook::fill` plus the `FormulaAst::shift`
copy/paste reference arithmetic it rides on. Replicating a cell across a target
range now shifts each copy's **relative** references by its offset while leaving
**absolute** (`$`) references pinned — the classic spreadsheet fill.

- `FormulaAst::shift(d_row, d_col) -> FormulaAst` (`ast.rs`): pure recursive
  copy/paste rewrite of every `Ref`/`Range`, the sibling of `adjust`
  (structural edits). The two differ on absolutes: `adjust` moves both relative
  and absolute refs (a cell physically relocates); `shift` tracks relatives and
  pins absolutes (`=A1`→`=A2` on fill-down, `$A$1` stays). A reference shifted
  off the top/left edge collapses to `#REF!`. 6 new tests.
- `Workbook::fill(sheet, src, dst)` (`workbook.rs`): replicate the `src` cell
  across every cell of the `dst` range — formulas shifted per-target, literals
  copied unchanged, an empty source clears each target, and the source's display
  **format rides along**. One recalc transaction; unknown sheet is a no-op; a
  `dst` over `MAX_RANGE_CELLS` is rejected wholesale (the same DoS guard formula
  ranges use, so a hostile caller can't ask the engine to write billions of
  cells). 7 new tests.
- **Fix: absolute references now resolve.** A cell is keyed by *position* only,
  but the evaluator (and `collect_refs`) used a reference's full address —
  including its `$` flags — as the cell-store / dependency-graph key, so `=$A$1`
  missed the relatively-stored `A1` cell and read as **0**, and editing `A1`
  did not recompute a dependent that referenced it absolutely. New
  `CellAddress::without_absolute()` normalises the address at those two
  boundaries. (Pre-existing latent bug — no test had ever evaluated an absolute
  reference; surfaced by the fill work, which depends on correct `$` handling.)
  New regression test.

## 0.6.0

**`get_display_window`** — a windowed read returning each cell's **formatted
display string** (its value rendered through its format code), the format-aware
sibling of `get_window`. This is the one read a virtualized grid needs per
frame: a dense, ready-to-draw rectangle, so a host renders engine-formatted text
directly instead of re-deriving number formatting itself.

- `Workbook::get_display_window(sheet, row0, col0, row1, col1) -> Result<DisplayWindow, _>`;
  new `DisplayWindow` type (row-major `cells: Vec<String>`, empty cells `""`).
- The 1-based-coords / inverted / `MAX_WINDOW_CELLS` / u64-span-overflow guards
  are now a shared `window_dims` helper used by both `get_window` and
  `get_display_window`, so the (security-critical) bounds checks can't drift
  apart. `get_window`'s behavior is unchanged (its tests still pass).
- 1 new test (formatted/percent/text/empty cells row-major + the bounds guards).

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
