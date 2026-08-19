# Changelog

## Unreleased — 2026-08-18

Brought under CI: the crate had no `BUILD` file, so nothing ever ran its tests
or linted it.

### Added

- **`BUILD` file — this crate is now built, tested and linted in CI.**

  This crate is a member of the `code/packages/rust` workspace, so it compiled
  whenever a sibling with a `BUILD` file pulled it in as a path dependency. But
  the build tool discovers work by scanning for `BUILD` files, so with none of
  its own it was never a package in its own right: its **test targets were never
  compiled, its assertions never ran, and `cargo clippy --all-targets -- -D
  warnings` never linted it**, on any platform. Adding `BUILD` puts it under the
  same per-package clippy gate and test run as every other watched Rust crate.

  The BUILD is the repo-standard one-liner, `cargo test -p spreadsheet-core -- --nocapture`,
  kept on a single line: the build tool runs each BUILD line as its own
  `sh -c`, so a backslash continuation would silently truncate the command.
  It was verified green locally first — clippy `-D warnings` clean and a full
  unfiltered `cargo test --no-fail-fast` passing — per the "expect to find
  existing breakage when you start watching a long-unwatched package" rule in
  `lessons.md`.

## 0.17.0

**Sparse read accessors for serializers.** Two additive, non-breaking `Workbook`
methods added for the `spreadsheet-io` adapter (SSIO01) that unifies `.xlsx`/
`.xls` load & save onto this engine:

- `cell_is_formula(sheet, addr) -> bool` — whether a cell holds a formula (vs a
  literal value or empty). A serializer needs it to choose between writing a
  formula and a plain value; `cell_source_text` alone can't tell them apart (a
  formula's text and a literal's canonical string are both just strings) and the
  `=` prefix is unreliable.
- `populated_cells(sheet) -> Vec<CellAddress>` — the **sparse**, sorted list of
  non-empty cell addresses. The counterpart to `used_range` (a bounding box): a
  serializer must walk only the cells that exist, never the dense rectangle
  between them. A sheet with cells at `A1` and `XFD1048576` has a ~17-billion-
  position used range but two populated cells — iterating the box would hang.
  This closes a DoS in any code that expanded `used_range` into a dense walk.

## 0.16.0

**Column widths & row heights — engine-homed, persisted layout.** The `Workbook`
now stores per-sheet column widths and row heights, the engine side of resizable
columns/rows in the demos. The engine treats the value as an **opaque `f64` in host
units it never interprets** — it only stores, key-shifts, and serializes it; the host
owns the unit, the default, and any min/max clamp.

- `Sheet` gains `col_widths: HashMap<u32, f64>` + `row_heights: HashMap<u32, f64>`,
  keyed by the 1-based column / row index. A column / row absent from the map uses the
  host default, so a fresh sheet (both maps empty) is byte-identical to before.
- API: `column_width(sheet, col) -> Option<f64>` / `row_height(sheet, row)`;
  `column_widths_in(sheet, c0, c1)` / `row_heights_in(sheet, r0, r1)` bulk reads (only
  the customized indices in range, sorted) so a host fetches a viewport's overrides in
  one call; `set_column_width` / `set_row_height` (return `bool`; reject `NaN` / `±∞` /
  `≤ 0` / index 0 so a bad host value can't poison the map or the file; setting the
  current value is a no-op, no revision bump); `clear_column_width` / `clear_row_height`.
- **Structural edits shift the keys.** `apply_structural_edit` slides column-width keys
  on InsertCols / DeleteCols and row-height keys on InsertRows / DeleteRows (reusing the
  same `insert_coord` / `delete_coord` helpers that shift cell addresses and references,
  now `pub(crate)`), dropping a key in a deleted band — so widen column C, insert a
  column at B, and the widened column (now D) keeps its width. The other axis is untouched.
- **Sort-immune:** column widths are columnar and row heights positional, so a range
  sort (which reorders row *records*) leaves them where they are (matches Excel —
  resize is chrome, not cell data).
- **Persisted:** `serialize` emits optional per-sheet `colWidths` / `rowHeights` arrays
  (sorted, **only when non-empty** — the document stays `version: 1` and a workbook with
  no custom sizes serializes byte-identically to before, exactly as `formats` was added).
  `deserialize` reads them **tolerantly** — a missing array, a non-finite / ≤ 0 value, or
  a 0 / out-of-`u32` index is skipped, never aborting the load.
- Sheet management is free: the maps live inside `Sheet`, so `delete_sheet` drops them
  and `move_sheet` carries them with no reindex (they're keyed by col/row, not `SheetId`).
- 8 new tests (set/get/clear, reject bad values + indices, bulk-in-range, insert/delete
  shift both axes independently, sort-immunity, two-sheet round-trip, tolerant load,
  empty-omits-arrays). 203 tests pass; `#![forbid(unsafe_code)]` unchanged.

## 0.15.0

**Sheet management + multi-sheet load fix (multi-sheet PR-4 — the last engine
slice).** Rename / delete / reorder sheets, list them for a tab bar, and a fix so a
loaded workbook's cross-sheet dependencies are live.

- `Workbook::sheet_names() -> Vec<&str>` (tab order), `rename_sheet(id, new)`,
  `delete_sheet(id)`, `move_sheet(id, to_index)`. Dense `SheetId`s are the sheet
  `Vec`'s indices, so delete/move rebuild the `name → SheetId` index and the
  dependency graph (`rebuild_sheet_index` helper); names — and therefore formula
  qualifiers and computed values — are unaffected by a reorder.
- **Rename** rewrites the qualifier in every formula that named the sheet
  (`=Old!A1` → `=New!A1`, via `FormulaAst::rename_qualifier`); the `SheetId` and all
  values are unchanged. Rejects an empty or duplicate name.
- **Delete** refuses to remove the last sheet, reindexes the survivors, and rewrites
  every inbound reference to the gone sheet to the `#REF!` literal
  (`FormulaAst::sheet_refs_to_error`) — permanent, so re-adding a same-named sheet
  doesn't resurrect it (Excel behaviour).
- **Fix (`deserialize`)**: a workbook is loaded sheet-by-sheet in file order, so a
  cross-sheet formula loaded *before* its target sheet couldn't resolve its
  qualifier at `set_formula` time and its dependency edge was skipped — values were
  right (a full `recalc_all` resolves names) but a later edit of the precedent didn't
  recompute the dependent. `deserialize` now rebuilds the dependency graph after all
  sheets exist, so a loaded cross-sheet formula is fully live.
- Tests: rename rewrites qualifiers + keeps values + recomputes; delete reindexes,
  inbound → `#REF!`, can't-delete-last; move reorders + preserves cross-sheet values;
  multi-sheet serialize round-trip with a live cross-sheet formula.

## 0.14.0

**Cross-sheet references — structural-edit / fill / sort propagation (multi-sheet
PR-3).** Cross-sheet references now survive the operations that move cells around:
a structural edit on one sheet ripples into *inbound* references from other sheets,
fill replicates a qualified relative ref correctly, and sort leaves cross-sheet refs
pinned.

- `FormulaAst::adjust_for_sheet_edit(edit, edited_is_host, edited_name)` (`edit.rs`):
  a reference shifts (or → `#REF!` on a deleted band) **only if it points into the
  edited sheet** — an unqualified ref when the formula's own sheet is edited, or a
  qualified ref whose name matches the edited sheet. `apply_structural_edit` now
  relocates the edited sheet's own cells with `edited_is_host = true`, then walks
  **every other sheet** and rewrites only their `EditedSheet!…` references
  (`edited_is_host = false`) — so inserting/deleting rows or columns on `Summary`
  shifts a `=Summary!A5` reference living on `Sheet1` to `=Summary!A6` (and a
  reference to a deleted band becomes `#REF!`).
- `FormulaAst::shift_local(d_row, d_col)` (`ast.rs`): like `shift`, but **only
  same-sheet refs move** — a qualified ref names a fixed cell on another sheet.
  `sort_range` now uses `shift_local`, so sorting rows within a sheet no longer
  mis-shifts a row's `=Summary!A1` reference (whereas *drag-fill* still uses `shift`,
  which correctly shifts a qualified *relative* ref — `=Summary!A1` filled down is
  `=Summary!A2`).
- Tests: inbound structural shift + deleted-band → `#REF!`, an own-sheet edit leaving
  the formula's outbound cross-sheet refs alone, fill shifting a qualified relative
  ref (keeping the qualifier), and sort pinning cross-sheet refs while reordering rows.

## 0.13.0

**Cross-sheet references — evaluation + dependencies (multi-sheet PR-2).** A
cross-sheet reference (`=Summary!A1`) now **resolves and reads the target sheet**,
and editing a cell on one sheet recomputes a formula on another through the
already-cross-sheet dependency graph. (PR-1 added parse/represent/re-emit; this is
the slice where a qualified reference computes a real value instead of `#REF!`.)

- `evaluate` / `collect_refs` (`recalc.rs`) take a `resolve: Fn(&str) ->
  Option<SheetId>` callback (threaded through the lazy `IF`/`AND`/`OR`/`IFERROR`
  helpers). A qualified ref resolves its sheet name to a `SheetId` and reads/depends
  on that sheet; an **unknown** sheet name resolves to `None` → `#REF!` and registers
  no precedent. Unqualified refs are unchanged (resolve to the current sheet).
- The workbook wires the resolver as `|name| self.sheet_by_name.get(name).copied()`
  at every eval and dependency-collection site (`set_formula`, the dependency-graph
  rebuild, and `evaluate_cell`). So a cross-sheet edge is registered when the formula
  is set, and `set_value` on the target sheet recomputes the cross-sheet dependent.
- Tests: cross-sheet read + recompute across two sheets (`Summary!A1` edit ⇒ a
  `Sheet1` formula updates), `SUM(Summary!A1:A3)` over a range on another sheet, an
  unknown sheet → `#REF!`, and cross-sheet precedents registered against the *target*
  sheet in `collect_refs`.

## 0.12.0

**Cross-sheet references — formula layer (multi-sheet workbooks, PR-1 of the arc).**
The workbook container and the dependency graph were already multi-sheet
(`Workbook.sheets` / `sheet_by_name`, `dag::Node = (SheetId, CellAddress)`); this
release teaches the **formula layer** to *represent, parse, and re-emit* a
cross-sheet reference (`=Summary!A1`). Evaluation of a qualified reference is
deferred to the next release and yields `#REF!` in the meantime (a clean "not
wired" signal, never a wrong value). The single-sheet path is byte-identical.

- `FormulaAst::Ref` / `Range` gain an `Option<String>` **sheet qualifier**
  (`ast.rs`): `None` = the formula's own sheet (the unchanged common case),
  `Some(name)` = a cross-sheet reference, holding the sheet name *as written*
  (resolved to a `SheetId` later, by a workbook). New ergonomic constructors
  `FormulaAst::cell` / `sheet_cell` / `cell_range` / `sheet_range`.
- Parser (`parser.rs`): `Name!A1`, `Name!A1:B2`, and single-quoted
  `'Q1 Budget'!A1` / `'O''Brien'!A1` (doubled `''` → a literal apostrophe). A `!`
  makes the preceding token a **sheet name**, never a cell, so `'A1'!B2` is
  unambiguous. An unknown sheet is not a parse error — it becomes `#REF!` at
  evaluation, so formulas can load in any order.
- Re-emit (`to_formula_string`): a qualified reference prints `Name!A1`,
  single-quoting the name only when it isn't a bare token (spaces, punctuation,
  a leading digit, or a name that itself spells a cell address).
- `shift` (fill/copy) and `adjust` (structural edits) **preserve the qualifier**:
  a cross-sheet ref shifts its address but keeps its sheet, and a structural edit
  on a formula's own sheet leaves its cross-sheet refs untouched (their target
  lives elsewhere — inbound cross-sheet propagation is a later slice).
- Evaluation + dependency extraction (`recalc.rs`): unqualified refs are
  unchanged; a qualified ref evaluates to `#REF!` and registers no precedent
  (pending the resolver slice).

## 0.11.1

- Make `Workbook::cell_source_text` **public** so a facade can resync its
  raw-source echo map after `replace_all` rewrites cells (the engine had no other
  public accessor for a cell's source text). No behavior change.

## 0.11.0

**Find / replace + `set_raw` (Edit ▸ Find / Replace).** Locate and bulk-edit cells
by text, and a single raw-string entry point that centralizes cell-entry policy.

- `Workbook::set_raw(sheet, addr, raw)` (`workbook.rs`): the one place that decides
  "what a typed string means" — trims, routes empty → `clear_cell`, a `=`-prefix →
  `set_formula` (a string that won't parse degrades to a `#VALUE!` literal), and
  anything else through literal coercion (`"TRUE"`/`"FALSE"` → boolean, finite
  number → number, else text) → `set_value`. The facades previously each
  re-implemented this; the replace path and any host can now reach the engine's
  full cell-entry behaviour through one call.
- `Workbook::find_all(sheet, query, in_formulas, match_case) -> Vec<CellAddress>`:
  every non-empty cell whose text contains `query`, in (row, col) order.
  `in_formulas` picks the haystack — the cell's **source** (formula text / literal
  canonical string) when true, its **computed display** value when false.
  `match_case = false` folds ASCII case. Empty query → no matches. Sparse (scans
  only populated cells).
- `Workbook::replace_all(sheet, query, replacement, match_case) -> usize`: rewrites
  the matched substring(s) in each matching cell's **source** and re-applies via
  `set_raw` (so the result re-parses — a still-`=` result as a formula, a literal
  re-coerced); returns the count of cells changed. Empty query is a no-op. Like a
  spreadsheet, a replace can break a formula or edit its *text* not its references
  — the caller chooses the query.
- New private helpers `coerce_literal` / `contains` / `replace_substring`
  (case-insensitive replace splices over original spans, UTF-8-boundary-safe) +
  `cell_source_text`. 4 unit tests (set_raw routing incl. invalid-formula→#VALUE!;
  find by value vs source, case-insensitive, ordered, empty-query; replace in
  literals + formulas with recompute, no-match/empty → 0; case-insensitive
  replace). Spec §4 "Find / replace". No new public types; `spreadsheet-core` →
  0.11.0. Facades + the 6 demos follow in later PRs.

## 0.10.0

**Range sort — `Workbook::sort_range` (Data ▸ Sort).** Reorders the **rows** of a
rectangular range by the computed values in one **key column** — the third member
of the range-operation family (after `fill` and the clipboard), built on the same
`FormulaAst::shift` machinery.

- `Workbook::sort_range(sheet, range, key_col, ascending) -> bool` (`workbook.rs`):
  each row of `range` is a record spanning the range's columns; the rows are
  permuted into key order while every record's cells stay together. The sort key
  is the cell's **computed value** at `(row, key_col)` (a formula sorts by what it
  evaluates to, not its text).
- **Total order** over values, so any mix of types is deterministic: blanks always
  sort last (both directions, Excel's rule); otherwise by type — Number < Text <
  Boolean < Error — then within a type (numeric, **case-insensitive** text with a
  case-sensitive tiebreak, `FALSE`<`TRUE`, fixed error order). `ascending = false`
  reverses only the non-empty comparison. The sort is **stable** (equal keys keep
  their original relative order).
- Because the rows physically move, a moved cell's formula has its references
  **shifted by that row's displacement** (`Δrow`, `Δcol = 0`) via `FormulaAst::shift`
  — relative refs track, absolute (`$`) refs pin, an off-grid ref collapses to
  `#REF!` — exactly as if each row were cut and pasted. Display **formats** ride
  with their cells. Cells in the sorted rows but outside the column band, and all
  cells outside the range, are untouched.
- Returns the **permutation** it applied — `Some(order)` where
  `order[new_row_offset] = old_row_offset` (0-based from `range.start.row`), so a
  caller that keeps its own per-cell side-table (the wasm facade's raw-source echo
  map) can replay the exact row move with `rewrite_raw_for_fill` instead of
  re-deriving the comparator. `None` is the no-op rejection (unknown sheet,
  out-of-range `key_col`, empty/inverted/single-row range, or a range over
  `MAX_RANGE_CELLS` — the shared DoS guard); an already-sorted range returns
  `Some(identity)` and is left untouched (no revision bump). One recalc
  transaction; every cell in the range is logged for `changed_since`.
- *Divergence from Excel* (documented, same class as `cut`): a sort shifts each
  moved formula's own refs by its row displacement and does not rewrite refs that
  pointed into the range from outside. Plain-data columns — the common case — sort
  exactly as expected.
- 13 unit tests (numbers asc/desc, text case-insensitive, stable equal keys, blanks
  last both directions, cross-type order, record+format carry, relative-ref shift,
  outside-band untouched, bad-args rejection, already-sorted no-op, change logging,
  returned-permutation). Spec §4 "Range sort". No new public types;
  `spreadsheet-core` → 0.10.0.

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
