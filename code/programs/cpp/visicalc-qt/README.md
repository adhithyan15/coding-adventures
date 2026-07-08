# VisiCalc — Qt demo (live, on the Rust engine)

The Qt/QML VisiCalc demo (C++ host), now **computing on the shared Rust
`spreadsheet-core` engine** through its C ABI (`spreadsheet-capi`) — the same
engine the SwiftUI demo links natively and the HTML / WebComponent demos run as
WebAssembly. This is the second native backend wired to the engine, and the
proof that the one-engine-everywhere architecture reaches C++.

## What it shows

- An auto-generated `FormulaBar.qml` and `Grid.qml`
  (`build/`, produced by `mosaic-compile --backend qt` from the shared
  `code/programs/mosaic/visicalc/*` sources — the same triples the React, HTML,
  WebComponent, and SwiftUI demos consume).
- The grid renders **engine-computed** values: the classic cross-footing budget
  where column E totals each row, row 5 totals each column, and E5 is the grand
  total (169) — all formulas evaluated by the Rust engine, not hard-coded.
- Editing the formula bar (e.g. `100` or `=SUM(A1:A4)`) writes through to the
  engine via `SpreadsheetModel.setSelected`, which recomputes every dependent
  cell. Clicking a cell selects it and pulls its source into the bar.

## How it's wired to the engine

```
QML views (generated)  ──  SpreadsheetModel  (src/SpreadsheetModel.{h,cpp})
   main.qml binds              │  sc_set_cell / sc_get_value … (C strings → QString)
   model.viewportRows          ▼
                       spreadsheet.h  (C ABI header, vendored)
                               │  links
   libspreadsheet_capi.a  ←  spreadsheet-capi (Rust C ABI)  ←  spreadsheet-core
```

`SpreadsheetModel` is a `QObject` that owns the engine session and exposes the
computed display matrix (`viewportRows`), the selection, and `setSelected(...)`
to QML. `main.cpp` registers it as the `model` context property before loading
`main.qml`; the generated `Grid` binds its `viewportRows` to `model.viewportRows`.
The model is QtCore-only (no GUI types), so the headless test exercises it
without a display.

## Build, test, run

All paths need the **Qt 6 SDK** (https://www.qt.io/download). CMake is optional —
qmake (which ships with Qt) is enough.

```bash
bash scripts/build.sh   # regenerate QML + build & vendor the engine static lib
```

### Run the engine-backed GUI

```bash
qmake && make && ./visicalc_qt_app          # qmake — no CMake needed
# or, if you have CMake:
cmake -B build-cmake && cmake --build build-cmake && ./build-cmake/visicalc_qt_app
```

### Headless proof (the Qt equivalent of `swift test`)

```bash
cd test && qmake && make && ./tst_model
```

`test/tst_model.cpp` (QtTest) links the vendored engine and asserts the grid
values are engine-computed (E1 = 38, A5 = 39, E5 = 169), that editing A1
15 → 115 recomputes the totals (E5 → 269), and that a formula entry computes
with binary-op error propagation (`=1/0` → `#DIV/0!`, and `=A1+1` over it →
`#DIV/0!`). A green run means the C++ ↔ C ABI ↔ Rust path is sound end-to-end.

> Note: `qml main.qml` still opens the layout for QML iteration, but the bare
> runner can't expose the C++ `model` or link the engine, so its grid is empty.
> Build and run the binary to see the live spreadsheet.

## Infinite virtualized sheet

`SpreadsheetModel` also exposes the engine's **viewport primitive** —
`window(r0,c0,r1,c1)` (a dense `QVariantList` rectangle of display strings),
`usedRange()`, `columnLetters()`, `currentRevision()`, and `changedSince()` —
over the C ABI's `sc_get_display_window` / `sc_used_range` / `sc_changed_since`.
`window()` reads `sc_get_display_window`, so each cell arrives already rendered
through its Excel-style format code (the seed formats the cross-foot totals as
`#,##0.00` and the far-flung `Z1000` total as a percent) — the Qt host paints
the strings directly and never re-derives number formatting. These
are `Q_INVOKABLE`, so a windowed QML grid (rendering only the visible rectangle
of an unbounded sheet, the Qt sibling of the web/SwiftUI infinite views) binds
to them directly.

### The touch FormulaBar layout (`FormulaBarTouch.qml`)

The **Touch bar** button (shown in classic-grid mode) swaps the formula bar
between two layouts generated from the *same* `FormulaBar.mil` interface:

- **Desktop** (`FormulaBar.desktop.mll` → `FormulaBar.qml`): a `RowLayout` —
  the cell-address label sits to the **left** of the input.
- **Touch** (`FormulaBar.touch.mll` → `FormulaBarTouch.qml`): a `ColumnLayout` —
  the address label stacks **above** a full-width input, the arrangement that
  fits a phone-width viewport.

`scripts/build.sh` emits both QML files (the touch one as `FormulaBarTouch.qml`
because a QML type name is its file basename), `qml/qmldir` registers both, and
`main.qml` mounts both bound to the same `model` + shared `commitFormula()` /
`cancelFormula()` handlers — only one is `visible` at a time. This is the UI30
"one component, many layouts, identical host contract" invariant made a runtime
toggle: the native analogue of the web demo's `touch.html`, which pivots the
same `FormulaBar.touch.mll` to `flex-direction: column`. Editing behaves
identically in both — the layout is the only thing that changes.

### The scrollable infinite GUI (`InfiniteSheet.qml`)

The **Infinite sheet** button in the running app toggles from the classic 5×5
grid to `InfiniteSheet.qml` — a virtualized, effectively-infinite (u32 × u32,
sparse) sheet rendered on the same engine. The body is a QtQuick `ListView`,
which natively virtualizes: it instantiates a row delegate only while that row
is on screen, so a 1000-row-tall sheet costs the handful of rows you can
actually see. Each visible row calls `model.rowCells(row)` **once** — a single
`get_display_window` over that row's `1×totalCols` strip — so per-frame engine
work is proportional to *visible* rows, never to the sheet's height.

Two-axis scroll with frozen chrome, kept in sync by binding offsets: the
column-letter header tracks the body's horizontal pan (`header.contentX ←
bodyFlick.contentX`) and the row-number gutter (its own non-interactive
`ListView`) tracks the body's vertical scroll (`gutter.contentY ←
body.contentY`). Tapping a cell calls `model.selectInf(row, col)` (pulling its
source into the formula bar); pressing Enter calls `model.commitInf(text)`,
which writes through to the engine, recomputes every dependent, regrows the
extent, and bumps `model.revision` so the visible rows re-fetch. The **"Fill ↓
10"** button next to the formula bar calls `model.fill(src, dstStart, dstEnd)`
(over the C ABI's `sc_fill`) to replicate the selected cell into the 10 rows
below it — the engine shifts each copy's relative references (`=A1`→`=A2`, …),
pins absolute (`$`) refs, and carries the format. The **Copy / Cut / Paste**
buttons drive the engine's clipboard (`model.copy`/`cut`/`paste` over the C ABI's
`sc_copy`/`sc_cut`/`sc_paste`): copy the selected cell, then paste it elsewhere
with its relative references shifted by the destination's offset (absolute `$`
refs pinned, format carried); a cut clears the source on paste. `paste` returns a
`bool` — false (a no-op) for an empty clipboard, malformed address, or off-grid.
The **Save / Load** buttons serialize the whole workbook (`model.serialize` over
the C ABI's `sc_serialize`) to a JSON document held in memory and restore it
(`model.deserialize` / `sc_deserialize`): the document captures only the source
(formula text + typed literals) and per-cell formats — not the computed values,
which the engine recomputes on load, so a loaded formula stays live.
The **Save… / Open…** buttons open and save a **real spreadsheet file** through a
proper OS dialog (Qt Quick `FileDialog`): `model.saveFile(path, format)` /
`openFile(path, format)` wrap `model.exportBytes(format)` / `importBytes(format,
bytes)`, which bind the C ABI's `sc_save_<fmt>` / `sc_load_<fmt>`
(`spreadsheet-capi`). File bytes ride in a **`QByteArray`** — an `.xlsx` (a ZIP,
with live formulas preserved) or an `.xls`/`.csv`/`.tsv`/`.json` crosses the C ABI
as raw bytes, never a `QString` that a `0x00` inside a ZIP would truncate; a save
copies the engine's `(ptr, len)` buffer out and frees it with `sc_bytes_free` (the
engine's own allocator). Unlike the plugin-free hosts, Qt's `FileDialog` gives a
genuine path, so there's no fixed demo file. The headless `test/tst_model`
round-trips the bytes: a saved `.xlsx` is asserted to begin with the ZIP magic
`PK\x03\x04` (and its formula still recomputes after a reopen), an `.xls` with the
OLE2 magic `0xD0CF`, and unknown/empty inputs are safe no-ops.
The **Undo / Redo** buttons walk the engine's snapshot history (`model.undo`/
`redo` over the C ABI's `sc_undo`/`sc_redo`); they enable off the `canUndo`/
`canRedo` Q_PROPERTYs (which notify on `revisionChanged`, so they re-evaluate
after every edit). Every edit is reversible and a restored formula recomputes
live.

The model seeds far-flung sparse cells (`Z1000`, `BA50`, `BB50`) on top of the
budget so there's something to scroll to; the extent (`totalRows`/`totalCols`)
is derived from `usedRange()` plus a margin.

#### Visual design

`InfiniteSheet.qml` mirrors the **reference visual language** defined by the web
demo (`code/programs/typescript/visicalc-html/infinite.html`) so every VisiCalc backend reads as one
considered dark, modern-spreadsheet surface. The palette lives in a small set of
`readonly property color` design tokens at the top of the file (`cBg`/`cPanel`/
`cSurface`/`cLine`/`cInk`/`cMuted`/`cAccent`…), echoing the web demo's CSS custom
properties rather than scattering hex values. From those tokens it builds: a
panel-wrapped **toolbar** with an address **pill**, an `fx` marker, then a grown
formula field with an accent **focus ring**; the actions are **segmented button
groups** (drag-fill · clipboard · file · history) — a reusable
`component ToolButton: Button` with hover/down/disabled chrome — separated by
thin rules. The grid gets subtle **zebra** row banding, a 2-px **accent
selection ring**, and the selected cell's **row + column headers tint to the
accent**; a hairline-separated **status footer** echoes the live virtual-grid
size and revision. Cells and the formula field use a real monospace face
(`Menlo` on macOS, `Consolas` on Windows, `monospace` elsewhere) so digits
column-align. `src/main.cpp` calls `QQuickStyle::setStyle("Basic")` before
loading any QML, because the macOS *native* Controls style ignores `Button`
`background`/`contentItem` customization (the Basic style honors it).

### Headless proof

`test/tst_window.cpp` (qmake) seeds far-flung sparse cells and asserts the
window is engine-computed + dense, a formula 1000 rows down (`Z1000` = 39) is
reachable, the gaps are empty (sparse), column letters run AA/BA, and editing
`A1` dirties the far dependent `Z1000` via `changedSince`. A fourth case drives
the infinite-view binding layer directly: `rowCells` returns one engine-read
row, `selectInf` selects + clamps and loads the source, and `commitInf` edits
`A2` 8 → 108 with every dependent recomputing (E2 → 151, A5 → 139, E5 → 269).
Further cases cover drag-fill, clipboard copy/cut/paste, a save/load round
trip (serialize → mutate → restore, with the loaded formula staying live and
malformed input rejected), and an undo/redo walk (two edits → undo both → redo
both with the formula recomputing live → a fresh edit forks history):

```bash
cd test && qmake tst_window.pro && make && ./tst_window
```

## Where this fits in the cross-backend demo plan

| Backend | Engine | Status |
|---|---|---|
| HTML (web) | WASM | ✅ live |
| WebComponent (web) | WASM | ✅ live |
| SwiftUI (macOS / iOS) | C ABI | ✅ live |
| Qt / C++ (this one) | C ABI | ✅ live |
| Flutter (Dart) | C ABI (dart:ffi) | in progress |
| Compose / Android (Kotlin) | C ABI (FFM / JNI) | in progress |
| XAML (.NET, Windows) | C ABI (P/Invoke) | in progress |
