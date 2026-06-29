# VisiCalc column width & row height

Status: **draft** (spec-first; no implementation yet)
Scope: the next cross-cutting feature after multi-sheet, rolled out the same way
— engine first, then the 3 facades, then all 6 VisiCalc demos
(web → Qt → Flutter → Compose → XAML → SwiftUI), one PR per stage/backend, each
security-reviewed, headless-proven, and babysat to merge.

## 1. Why this, and the layering decision

A real spreadsheet lets you resize columns and rows: drag a column header's right
edge to widen it, drag a row's bottom edge to make it taller. Today every VisiCalc
demo paints a **fixed** geometry — the web/Qt/Flutter/Compose/XAML/SwiftUI infinite
grids all hardcode `colW = 92`, `rowH = 26`. This feature makes those per-column /
per-row, user-adjustable, and **persisted**.

**The layering question.** A column's width is pure presentation — the engine never
*computes* with it (unlike a cell value or a format code, which changes the displayed
string). So two designs are possible:

1. **Demo-only view state** — each demo keeps its own `Map<col, width>`. Simple, but
   the size is **lost on save/load** and every demo reinvents the same map, the same
   insert/delete-column shifting, and the same serialization. Six divergent copies.
2. **Engine-homed, persisted layout** *(chosen)* — the `Workbook` stores per-sheet
   `col_widths` / `row_heights`, `serialize`/`deserialize` carry them, and a
   **structural edit shifts the keys** (insert a column → every later column's width
   slides right; delete a column → its width is dropped). The engine treats the value
   as an **opaque number it never interprets** — it just persists it, keyed by column
   or row, per sheet.

Design 2 matches how *formats* were added (per-cell display codes the engine stores
but the host paints) and how *multi-sheet* layout state lives per sheet. It keeps all
six demos consistent and makes layout survive a save/load round-trip — the same
"one shared engine, every backend identical" philosophy as the prior six rollouts.
The **host owns the units, the default, and the min/max clamp**; the engine owns
**storage, key-shifting, and persistence**.

This is a *smaller* campaign than multi-sheet: **one** engine PR (the container already
has the per-sheet `Sheet` struct, `serialize`, and `insert_coord`/`delete_coord` shift
helpers), then facades, then six demos.

## 2. Engine design (`spreadsheet-core`)

### 2.1 Storage — two maps per sheet

`Sheet` (today `{ name, cells, formats }`) gains two maps:

```rust
struct Sheet {
    name: String,
    cells: HashMap<CellAddress, Cell>,
    formats: HashMap<CellAddress, String>,
    col_widths: HashMap<u32, f64>,   // 1-based column index → width (host units)
    row_heights: HashMap<u32, f64>,  // 1-based row index    → height (host units)
}
```

A column/row **absent** from the map uses the host's default size. The value is an
**opaque `f64`** — the engine stores and persists it but never reads it for any
computation. Keyed by the same 1-based column/row index the rest of the API uses.

### 2.2 Workbook API

```rust
// Reads — None means "no custom size; the host uses its default".
pub fn column_width(&self, sheet: SheetId, col: u32) -> Option<f64>;
pub fn row_height(&self, sheet: SheetId, row: u32) -> Option<f64>;

// Bulk reads for a viewport strip — only the columns/rows in the inclusive range
// that HAVE a custom size, sorted ascending. Lets a host fetch a window's overrides
// in one call instead of one-per-column. (col0/row0 ≥ 1; empty if none.)
pub fn column_widths_in(&self, sheet: SheetId, col0: u32, col1: u32) -> Vec<(u32, f64)>;
pub fn row_heights_in(&self, sheet: SheetId, row0: u32, row1: u32) -> Vec<(u32, f64)>;

// Writes — return true if applied, false if rejected. A width must be FINITE and
// > 0 (NaN / ±∞ / ≤ 0 rejected, so a bad host value can't poison the map or the
// serialized file). col/row must be ≥ 1. Bumps the revision on a real change.
pub fn set_column_width(&mut self, sheet: SheetId, col: u32, width: f64) -> bool;
pub fn set_row_height(&mut self, sheet: SheetId, row: u32, height: f64) -> bool;

// Clear — back to the host default. Returns true if an entry was removed.
pub fn clear_column_width(&mut self, sheet: SheetId, col: u32) -> bool;
pub fn clear_row_height(&mut self, sheet: SheetId, row: u32) -> bool;
```

`set_*` with the same value already present is a no-op (no revision bump), matching
the engine's existing diff-gating convention (so an undo/redo snapshot isn't created
for a non-change).

### 2.3 Structural edits shift the keys

This is the part that *earns* the engine home. When rows/columns are inserted or
deleted, the width/height keys must move with their columns/rows — reusing the exact
`insert_coord(v, at, count)` / `delete_coord(v, at, count) -> Option<u32>` helpers that
already shift cell addresses and formula references in `edit.rs`:

- **InsertCols { at, count }** — every `col_widths` key `≥ at` slides up by `count`
  (`insert_coord`). Heights untouched.
- **DeleteCols { at, count }** — keys in the deleted band `[at, at+count)` are
  **dropped** (`delete_coord` → `None`); keys past the band slide down. Heights untouched.
- **InsertRows / DeleteRows** — the same, on `row_heights`. Widths untouched.

Applied inside `apply_structural_edit` (same site that already relocates cells +
formats + rewrites references), so insert/delete keeps the visual layout aligned with
the data: widen column C, insert a column at B, and the widened column is now D — its
width travels with it.

### 2.4 Sheet management is free

Because the maps live **inside `Sheet`**, `delete_sheet` drops them with the sheet and
`move_sheet` carries them — no extra reindex (the maps are keyed by col/row, not by
`SheetId`). `rename_sheet` touches neither. Nothing to add there.

### 2.5 Serialize / deserialize round-trip

`serialize` gains two **optional** per-sheet arrays (additive — **no version bump**;
the document stays `version: 1`, and an old file without them loads as "no custom
sizes", exactly as `formats` was added):

```json
{"version":1,"sheets":[{"name":"Sheet1",
  "cells":[...], "formats":[...],
  "colWidths":[{"col":3,"w":140.0}],
  "rowHeights":[{"row":2,"h":40.0}]}]}
```

Sorted by index for stable output. `deserialize` reads them **tolerantly** — a missing
array, a non-finite or ≤ 0 value, or a 0 index is skipped (never aborts the load), so a
hand-edited or future file can't crash a load. Empty maps emit empty arrays (or are
omitted — either is fine as long as the round-trip is lossless for real data).

### 2.6 Tests (engine PR)

`#![forbid(unsafe_code)]` stays. New tests:

- set/get/clear a width and a height; absent → `None`; same-value set is a no-op
  (revision unchanged); a 2nd different value overwrites.
- reject `NaN`, `+∞`, `-∞`, `0.0`, `-5.0`; reject col/row `0`.
- `column_widths_in` / `row_heights_in` return only in-range customized indices, sorted.
- **InsertCols** shifts a width's key up; **DeleteCols** drops the band's width and
  slides the rest down; rows analogous; the *other* axis is untouched.
- a width set on a column whose cells then get sorted (`sort_range`) does **not** move
  (sort reorders row *records*, not columns — widths are columnar) — i.e. width keys are
  immune to sort; height keys likewise stay put (sort moves values between rows but the
  row *heights* are positional chrome, not record data — **document this choice**).
- `serialize` → `deserialize` round-trips widths + heights across **two** sheets; a file
  with no width/height arrays still loads; a non-finite stored value is skipped on load.
- single-sheet / no-resize path stays **byte-identical**: every existing test passes
  unchanged (the maps default empty; `serialize` of a workbook with no custom sizes is
  unchanged if empty arrays are omitted — prefer omission to keep old golden strings).

> **Sort interaction (decided):** column widths are **columnar** and row heights are
> **positional** — neither participates in a range sort (which reorders the *values* of
> row records). Widening column C and sorting A1:E4 leaves C wide; row 2 being tall and
> sorting leaves row 2 tall. This matches Excel (resize is chrome, not cell data).

## 3. Facade design (`spreadsheet-facades`)

All three facades wrap `core-wasm`'s `SpreadsheetSession`, so the logic is added **once**
in `core-wasm`, on the **active sheet** (bare ops address it, like every other
session method):

- `core-wasm` `SpreadsheetSession`:
  `column_width(col) -> f64` (0.0 = unset/default), `row_height(row) -> f64`,
  `set_column_width(col, w) -> bool`, `set_row_height(row, h) -> bool`,
  `clear_column_width(col) -> bool`, `clear_row_height(row) -> bool`, and bulk
  `column_widths(col0, col1) -> JSON [{ "col": N, "w": F }, …]` /
  `row_heights(row0, row1) -> JSON [{ "row": N, "h": F }, …]`. The mutators go through
  the existing `mutate` gate (so a resize is **undoable** and snapshot-tracked, like any
  edit). 0.0 is an unambiguous "unset" sentinel because a valid width is always `> 0`.
- `capi`: `sc_column_width(s, col) -> double`, `sc_row_height(s, row) -> double`,
  `sc_set_column_width(s, col, w) -> int` (1/0), `sc_set_row_height(s, row, h) -> int`,
  `sc_clear_column_width(s, col) -> int`, `sc_clear_row_height(s, row) -> int`,
  `sc_column_widths(s, col0, col1) -> char*` (JSON, free with `sc_string_free`),
  `sc_row_heights(s, row0, row1) -> char*` + declarations in `include/spreadsheet.h`.
  `col`/`row` are `uint32_t`; the `double` is the host unit.
- `wasm`: linear-mem exports (`f64` width in/out) + JS loader wrappers
  (`columnWidth/rowHeight/setColumnWidth/setRowHeight/clearColumnWidth/clearRowHeight/
  columnWidths/rowHeights`) + rebuilt `pkg/spreadsheet_engine.wasm` + re-bundle.
- bump all 3 facades; `verify-infinite.mjs` gains a width/height proof (set a width,
  read it back, save → load round-trips it, insert a column shifts it).

## 4. Demo design (all six)

Each demo's infinite-grid view, one PR, same shape as the prior six rollouts:

- **Model passthrough**: `columnWidth(col)` / `rowHeight(row)` (fall back to the demo's
  default `colW`/`rowH` when unset), `setColumnWidth`/`setRowHeight`/`clearColumnWidth`/
  `clearRowHeight`, and a bulk read for the visible strip.
- **Render** each column at its width and each row at its height (the gutter / header /
  body must agree, since they already share the geometry constants).
- **Resize affordance**: a thin draggable handle on the **right edge of each column
  header** (drag to set that column's width) and the **bottom edge of each row's
  gutter cell** (drag to set its height), clamped to a sensible `[min, max]` (e.g.
  `[40, 600]` px for width, `[16, 240]` for height). On drag-commit, call
  `setColumnWidth`/`setRowHeight`; the grid re-reads. Double-click the handle to clear
  (auto-size back to default) where the toolkit makes that easy.
- **Seed**: widen one column and tallen one row in the seed so the demo opens showing a
  non-uniform grid (e.g. column C = 140, row 2 = 40), proving the engine path on launch.
- **Headless proof**: set a width → reads back; survives `saveBook` → mutate → `loadBook`
  (the loaded width is restored); `insertCol` before the widened column shifts its width
  with it; `deleteCol` of the widened column drops it. Plus the seed's widened
  column/tall row read back at their seeded sizes.
- **Backend proof harness** per backend: `verify-infinite.mjs` (web), `tst_window`
  (Qt, qmake), `flutter test` (Flutter), `scripts/verify.sh` (Compose kotlinc+FFM /
  XAML dotnet), `swift test` (SwiftUI). `/security-review` with the diff inline before
  every push; babysit each PR to green.

Per-backend binding notes (from the prior rollouts):
- **Flutter** `dart:ffi`: `double` ⇄ C `double` is a direct `Double` typedef; clamp
  col/row into u32 via the existing `_u32` helper before marshalling.
- **Compose** Java FFM: `FunctionDescriptor.of(JAVA_DOUBLE, ptr, JAVA_INT)` for the
  getter — **include the session ptr** (the recurring WrongMethodType trap); `JAVA_DOUBLE`
  for the width arg.
- **XAML** P/Invoke: `[DllImport] double sc_column_width(IntPtr, uint)`; the WinUI view
  is Windows-only, so `Engine.cs` + `test/Program.cs` carry the proof.
- **SwiftUI**: module-map C funcs (no `DllImport`); re-sync the **tracked** header
  `Sources/CSpreadsheetEngine/include/spreadsheet.h` from the canonical capi header;
  links the static `.a` from `Vendor/macos`.
- **Qt** `Q_INVOKABLE` `double columnWidth(int)` etc.; name the QByteArray locals so
  UTF-8 outlives any C call (only the bulk-JSON getter needs strings).
- All native demos **re-vendor** the freshly built capi (`.a`/`.dylib` + canonical
  `spreadsheet.h`) from the **worktree** rust dir (the primary checkout is stale);
  `nm`-verify the new symbols after re-vendor. The libs are gitignored (CI rebuilds);
  the SwiftUI header is tracked.

## 5. Out of scope

- Auto-fit ("size column to widest cell") — needs text measurement the engine can't do
  (no font metrics); a host could add it later as a demo-only convenience.
- Hidden rows/columns (width/height = 0) — `set_*` rejects ≤ 0; hiding is a separate
  feature with its own semantics (skip in layout, still computes).
- Freeze panes, named ranges, 3-D refs — separate future campaigns.

## 6. Rollout order (hard dependency chain)

```
spec (this doc)
  → engine PR        (storage + get/set/clear + bulk + structural shift + serialize + tests)
  → facades PR       (core-wasm + capi + wasm + JS + verify-infinite proof; bump 3)
  → 6 demo PRs       web → Qt → Flutter → Compose → XAML → SwiftUI
```

Build each stage only once its dependency is **merged to main**. When all eight PRs
(spec + engine + facades + 6 demos) are merged, column-width / row-height is complete
across all six backends — report and check in with the user before the next feature.
