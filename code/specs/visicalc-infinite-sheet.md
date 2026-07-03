# VisiCalc — Infinite Virtualized Sheet

Status: **proposed** · Layer: spreadsheet engine + facades + render hosts ·
Supersedes the fixed 5×5 demo grid with an unbounded, virtualized sheet.

## 1. Motivation

The cross-backend VisiCalc demos prove one Rust `spreadsheet-core` engine
computes live on every backend (web via WASM; native via the C ABI). But each
demo renders a **fixed 5×5** grid. A real spreadsheet is an *unbounded* sheet you
scroll through, where only the visible cells are materialised on screen.

This spec adds the missing abstraction: a **viewport primitive in the shared
engine** plus a **windowed-render contract** the hosts implement. The sheet is
already infinite at the storage layer — the work is to *read a window of it*
efficiently and *render only that window*.

### What already exists (no change needed)

- **Coordinate space is effectively infinite.** `CellAddress { row: u32, col:
  u32 }` → 4.29 billion rows × 4.29 billion columns. No human or generated
  dataset approaches this; we keep `u32` (going wider adds cost to every address
  and the dep graph for a ceiling nobody reaches).
- **Storage is sparse.** `Sheet.cells: HashMap<CellAddress, Cell>` — only cells
  you touch exist. An empty sheet is empty; a cell at `Z100000` costs one entry.
- **Recalc is dependency-ordered** (`dag.rs`, topological) and stamped by a
  global `Workbook.epoch: u64` bumped on every recalc.
- **Column letters exist**: `address::column_index_to_letters(u32) -> String`
  and `column_letters_to_index(&str) -> Result<u32>` (round-trip tested).

### What is missing (this spec)

1. A **windowed read** — give me just the rectangle the user can see.
2. An **extent query** — how far does the data go, so scrollbars size correctly.
3. A **changed-cells diff** — after an edit, which cells changed, so a host
   re-fetches only the dirtied *visible* cells instead of the whole window.
4. Hosts that **virtualize**: render only the visible window and recycle cells
   as the user scrolls, with a frozen header row (letters) and gutter column
   (row numbers).

## 2. Design principles

- **The abstraction lives in the shared engine, not per host.** Every backend
  asks the same primitive "what's in rectangle `(r0,c0)–(r1,c1)`?" and renders
  it. Hosts own *windowing/recycling* (platform-specific); the engine owns
  *what the window contains*. This keeps the 7 backends from each reinventing
  virtualization, exactly as they share recalc today.
- **1-based public coordinates** (A1 = row 1, col 1), matching the existing A1
  surface. Internal `CellAddress` stays as-is.
- **Bounded work per call.** A window read is capped so a host (or a hostile
  caller) can't request a billion-cell rectangle. The cap is screen-scale, not
  data-scale.
- **Same string-in / JSON-out facade contract** the existing calls use, so the
  WASM and C ABI surfaces stay byte-identical across hosts.

## 3. Engine API (`spreadsheet-core`)

All methods are on `Workbook`, operate on the active `SheetId`, and use 1-based
inclusive coordinates.

### 3.1 `get_window` — the windowed read

```rust
/// Dense computed values for the inclusive rectangle (row0..=row1, col0..=col1),
/// row-major. Empty cells yield CellValue::Empty (not omitted) so the host can
/// index the result directly by (r - row0, c - col0). Errors if the rectangle
/// is inverted or exceeds MAX_WINDOW_CELLS.
pub fn get_window(&self, sheet: SheetId,
                  row0: u32, col0: u32, row1: u32, col1: u32)
    -> Result<Window, SpreadsheetError>;

pub struct Window {
    pub row0: u32, pub col0: u32,        // echoed origin (1-based)
    pub rows: u32, pub cols: u32,        // dimensions
    pub values: Vec<CellValue>,          // rows*cols, row-major
}
```

- **Cap**: `MAX_WINDOW_CELLS = 1 << 16` (65 536). A 4K screen shows ~a few
  thousand cells; 64 K is generous headroom incl. overscan. Over-cap → a `Ref`
  error (same family the range cap already uses). The host clamps before
  asking; the cap is a safety net, not the budget.
- **Density**: returns blanks for empty cells so the host renders a solid grid.
  The implementation iterates the requested rect and looks each address up in
  the sparse map — `O(window)` not `O(sheet)`.

### 3.2 `used_range` — the extent

```rust
/// Bounding box of all materialised, non-empty cells on the sheet, or None if
/// the sheet is empty. 1-based inclusive. Lets a host size its scrollable area
/// to the data (plus a comfortable margin so you can scroll into blank space).
pub fn used_range(&self, sheet: SheetId) -> Option<UsedRange>;

pub struct UsedRange { pub min_row: u32, pub min_col: u32,
                       pub max_row: u32, pub max_col: u32 }
```

- v1 computes this by scanning the sparse map (`O(materialised cells)`). For the
  demo and any human-scale sheet this is trivially cheap. A follow-up may
  maintain running min/max on insert/remove if profiling warrants; the API does
  not change.

### 3.3 `changed_since` — the diff

The engine stamps each cell with the epoch at which its **value last changed**,
and keeps a bounded **change log** of `(epoch, address)` so a host can ask "what
changed since the epoch I last rendered?".

```rust
/// Addresses whose computed value changed strictly after `since_epoch`, on this
/// sheet, plus the current epoch. If `since_epoch` is older than the retained
/// window, returns `Stale` and the host must re-read its whole window (the
/// safe fallback — never silently miss a change).
pub fn changed_since(&self, sheet: SheetId, since_epoch: u64) -> ChangeSet;

pub enum ChangeSet {
    Delta { current_epoch: u64, changed: Vec<CellAddress> },
    Stale { current_epoch: u64 },   // gap too large; re-read the window
}
pub fn current_epoch(&self) -> u64;   // already exists as epoch()
```

Mechanics:
- `Cell` gains `changed_epoch: u64`. `set_cell` and recalc set it to the new
  epoch **only when the value actually differs** from the prior value (a
  no-op edit changes nothing → no spurious diff entry).
- The `Workbook` keeps `changes: VecDeque<(u64, SheetId, CellAddress)>`, pushed
  during `set_cell`/recalc, pruned to the last `CHANGELOG_RETAIN = 4096`
  entries. `changed_since(e)` walks the log; if the oldest retained epoch `> e`,
  it can't prove completeness → `Stale`.
- **Why a log + fallback rather than a per-cell scan**: a scan is `O(sheet)` and
  defeats the purpose of a diff. The log makes the common case (a handful of
  cells changed since the last frame) `O(changes)`, and the `Stale` fallback
  keeps correctness when a host has been away for many edits.

### 3.4 Column letters (already present — re-exported)

`column_index_to_letters` / `column_letters_to_index` move into the documented
public surface so hosts render identical headers (A, B … Z, AA, AB …) without
re-implementing the base-26-bijective math.

## 4. Facade surface (WASM + C ABI)

Each facade adds the same calls, string-in / JSON-out, alongside the existing
`set_cell` / `get_value` / `get_raw` / `get_values`:

| Call | Args | Returns (JSON) |
|---|---|---|
| `get_window` | `row0,col0,row1,col1` | `{"row0":1,"col0":1,"rows":R,"cols":C,"values":[[<value>,…],…]}` |
| `used_range` | — | `{"minRow":…,"minCol":…,"maxRow":…,"maxCol":…}` or `null` |
| `column_letters` | `index` (1-based) | `"AA"` |
| `current_epoch` | — | `{"epoch":N}` |
| `changed_since` | `since` | `{"epoch":N,"changed":["B2",…]}` or `{"epoch":N,"stale":true}` |

`<value>` is the existing value-JSON shape (`{"kind":"number","value":46}` …),
so a host reuses its current value→display mapping verbatim. The 2-D `values`
array is row-major, dimensions `rows × cols`.

- **WASM** (`spreadsheet-wasm`): hand-written `#[no_mangle]` entry points using
  the existing `[len][utf8]` linear-memory protocol; integer args passed
  directly. Panic-safe via `catch_unwind` like the current calls.
- **C ABI** (`spreadsheet-capi`): `sc_get_window(s, r0, c0, r1, c1) -> char*`,
  `sc_used_range(s) -> char*`, `sc_column_letters(s, idx) -> char*`,
  `sc_current_epoch(s) -> uint64_t`, `sc_changed_since(s, since) -> char*`.
  Each `char*` freed with `sc_string_free`, per the existing memory contract.

## 5. Windowed-render contract (hosts)

A virtualized host (web first; native later) implements:

1. **Geometry**: fixed row height `H` and per-column width. The scrollable
   content size is `(extent + margin)` from `used_range` — large enough to feel
   unbounded (e.g. data extent + 1000 rows / 50 cols of blank runway).
2. **Visible window**: from scroll offset `(top,left)` and viewport size,
   compute first/last visible row & col (+ a small **overscan** of 2–3 lines
   each side to avoid edge flicker). Clamp to `MAX_WINDOW_CELLS`.
3. **Fetch**: `get_window(firstRow,firstCol,lastRow,lastCol)` → render that
   rectangle, recycling cell elements/widgets (DOM nodes on web; a recycler /
   lazy grid on native). Absolute-position each cell at
   `((r-1)*H, Σ widths)` so scrolling is just a transform.
4. **Frozen chrome**: a sticky header row of column **letters** (from
   `column_letters`) and a sticky gutter column of **row numbers**, both
   following the scroll on the cross axis only.
5. **Edit → diff refetch**: on commit, `set_cell`, then `changed_since(lastEpoch)`;
   re-render only the changed cells that fall in the visible window (or re-read
   the window on `stale`). Update `lastEpoch`.

The host owns recycling and scroll plumbing; the engine owns values, extent, and
the change set. No host hard-codes data or grid size.

## 6. Verification

- **Engine**: Rust unit tests — `get_window` density + cap + inverted-rect
  error; `used_range` on empty/one-cell/scattered sheets; `changed_since`
  delta-then-stale, no-op-edit-no-diff, value-actually-changed stamping.
- **Facades**: WASM (Node smoke) + C ABI (C/Swift/… smokes) round-trip each new
  call and assert the JSON shape.
- **Web (first render proof)**: a virtualized HTML demo over a sheet seeded with
  scattered far-flung cells (e.g. a value at `A1`, a `=SUM` at `Z1000`, a block
  near `BA50`). Verified live with the preview tools: scroll thousands of rows
  and confirm (a) only visible cells are in the DOM (node count stays bounded),
  (b) headers read A…Z, AA, AB…, (c) jumping to a far cell shows its
  engine-computed value, (d) editing a cell recomputes dependents via the diff
  path. Screenshot + DOM-node-count assertion are the evidence.

## 7. Out of scope (tracked separately)

In-cell editing across the whole sheet, insert/delete rows-cols (which *shift*
references), copy/replicate with relative-ref adjustment, cell formatting,
save/load, and native-backend virtualization (web is the first proof; natives
follow the same contract in subsequent PRs).

## 8. PR sequence

1. **This spec** (committed first, per repo specs-first rule).
2. **Engine**: §3 primitive in `spreadsheet-core` + unit tests.
3. **Facades**: §4 in `spreadsheet-core-wasm` + `spreadsheet-wasm` +
   `spreadsheet-capi` + smokes.
4. **Web**: §5 virtualized HTML demo + live preview-tool verification (§6).
5. **Natives** (follow-ups): SwiftUI / Qt / Flutter / Compose / XAML windowed
   grids over the same primitive, one PR each.
