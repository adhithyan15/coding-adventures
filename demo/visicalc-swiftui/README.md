# VisiCalc — SwiftUI demo (live, on the Rust engine)

The SwiftUI VisiCalc demo (macOS / iOS), now **computing on the shared Rust
`spreadsheet-core` engine** through its C ABI — the same engine the HTML and
WebComponent demos run as WebAssembly. This is the first *native* backend wired
to the engine: it proves the one-engine-everywhere architecture reaches Swift.

## What it shows

- An auto-generated `FormulaBarView` and `GridView`
  (`Sources/VisiCalc/Generated/`, produced by `mosaic-compile --backend
  swiftui` from the shared `demo/visicalc/mosaic/*` sources).
- The grid renders **engine-computed** values: a cross-footing budget where
  column E totals each row, row 5 totals each column, and E5 is the grand total
  (169) — all formulas evaluated by the Rust engine, not hard-coded.
- A host control row (arrow keys to move the selection + an editable field)
  drives `SpreadsheetModel.setSelected`, which writes to the engine and
  recomputes. (The generated views are currently *display-only* — the SwiftUI
  emitter lowers their inputs to constant bindings and emits no tap handlers —
  so interactivity lives in the host layer.)

## How it's wired to the engine

```
SwiftUI views (generated)  ──  SpreadsheetModel / SpreadsheetSession (Engine.swift)
                                       │  sc_set_cell / sc_get_value … (C strings)
                                       ▼
   CSpreadsheetEngine (module map over spreadsheet.h)
                                       │  links
   libspreadsheet_capi.a  ←  spreadsheet-capi (Rust C ABI)  ←  spreadsheet-core
```

`scripts/build.sh` regenerates the views, then builds the `spreadsheet-capi`
crate to a static library and vendors it into `Vendor/` (git-ignored). The
`CSpreadsheetEngine` target exposes the C header to Swift; `Package.swift` links
the static library.

## Build, test, run

```bash
bash scripts/build.sh   # regenerate views + build & vendor the engine (macOS + iOS slices)
swift test              # HEADLESS: proves the grid is engine-computed + recomputes
swift run               # launch the SwiftUI app (macOS)
bash scripts/run-ios.sh # build + launch on the iOS Simulator (same code + engine)
```

`swift test` (`Tests/VisiCalcTests`) asserts the grid values come from the
engine (E1 = 38, A5 = 39, E5 = 169), that editing A1 15 → 115 recomputes the
totals (E5 → 269), and that a formula entry computes with binary-op error
propagation (`=1/0` → `#DIV/0!`, and `=A1+1` over it → `#DIV/0!`). Requires
Swift 5.9+ / Xcode 15+.

**iOS**: the same SwiftUI code and the same Rust engine run on iPhone — the
engine is cross-compiled for `aarch64-apple-ios-sim` (so the iOS slice links
its own `libspreadsheet_capi.a`) and `Package.swift` links the right slice per
platform. `scripts/run-ios.sh` builds it, wraps the executable in a `.app`, and
installs + launches it on a booted iOS Simulator. Verified: the grid computes
on iOS and edits recompute.

**Per-platform layout.** The formula bar uses a *different Mosaic layout* per
platform — the whole point of the `.mll` layer. `scripts/build.sh` generates
`FormulaBar.swift` from `FormulaBar.desktop.mll` (an `HStack`: address label and
field side-by-side) guarded `#if os(macOS)`, and `FormulaBar.touch.swift` from
`FormulaBar.touch.mll` (a `VStack`: field stacked under the label for full width
on a phone) guarded `#if os(iOS)`. Both declare `FormulaBarView`, so the guards
pick the right one and `ContentView` is unchanged. `ContentView` also fills the
full width on iOS (vs the 720pt desktop window) so nothing scrolls off the
narrow screen. (Grid.touch is identical to Grid.desktop, so the grid needs no
variant.)

## Infinite virtualized sheet

A **"Infinite sheet ›"** toggle (top-right) switches the demo to a virtualized,
effectively-infinite grid (`InfiniteGridView` + `WindowedSheetModel`), rendered
on the same engine through its **viewport primitive** — the C ABI's
`sc_get_display_window` / `sc_used_range` / `sc_changed_since` (the native
sibling of the web demo's `infinite.html`). `sc_get_display_window` returns each
cell already rendered through its Excel-style format code (the seed formats the
cross-foot totals as `#,##0.00` and the far-flung `Z1000` total as a percent),
so the host paints the display strings directly and never re-derives number
formatting. The sheet is u32 × u32 and sparse; a two-axis
`ScrollView` sized from `used_range` drives the scrollbars, and only the visible
window of cells is built into the view via `SpreadsheetSession.window(...)`.

The **"Fill ↓ 10"** button next to the formula bar calls
`WindowedSheetModel.fillDown(10)` (over the C ABI's `sc_fill`) to replicate the
selected cell into the 10 rows below it — the engine shifts each copy's relative
references (`=A1`→`=A2`, …), pins absolute (`$`) refs, and carries the format.
The **Copy / Cut / Paste** buttons drive the engine's clipboard
(`WindowedSheetModel.copyCell`/`cutCell`/`pasteCell` over the C ABI's
`sc_copy`/`sc_cut`/`sc_paste`): copy the selected cell, then paste it elsewhere
with its relative references shifted by the destination's offset (absolute `$`
refs pinned, format carried); a cut clears the source on paste, and `pasteCell`
returns `false` (a no-op) for an empty clipboard.
The **Save / Load** buttons serialize the whole workbook
(`WindowedSheetModel.saveBook` over the C ABI's `sc_serialize`) to a JSON
document held in memory and restore it (`loadBook` / `sc_deserialize`): the
document captures only the source (formula text + typed literals) and per-cell
formats — not the computed values, which the engine recomputes on load, so a
loaded formula stays live.
The **Undo / Redo** buttons walk the engine's snapshot history
(`WindowedSheetModel.undoEdit`/`redoEdit` over the C ABI's `sc_undo`/`sc_redo`);
they disable at the history ends via `canUndo`/`canRedo` (re-evaluated whenever
`revision`, a `@Published`, bumps). Every edit is reversible and a restored
formula recomputes live.
The **+ Row / − Row / + Col / − Col** buttons are **structural edits**
(`WindowedSheetModel.insertRow`/`deleteRow`/`insertCol`/`deleteCol` over the C
ABI's `sc_insert_rows`/`sc_delete_rows`/`sc_insert_cols`/`sc_delete_cols`): insert
or delete the selected cell's row/column, and the engine shifts every formula
reference at or after the band so dependents keep pointing at their precedents
(`=A1+A2` with a row inserted above becomes `=A1+A3`); a reference whose whole band
is deleted becomes `#REF!`.
The **Format** buttons (`.00` · `%` · `$` · `Gen`) apply an Excel-style
number-format code to the selected cell's *display only*
(`WindowedSheetModel.applyFormat` over the C ABI's `sc_set_format`): `.00` →
`#,##0.00` (`1234` → `1,234.00`), `%` → `0.0%`, `$` → `$#,##0.00`, and `Gen` →
`""` (clears, back to General). The stored value is untouched — `getRaw` still
returns the source and dependent formulas keep computing on the real number.
The **find / replace** group (a `find` box + a `replace` box + **Find** /
**Replace** buttons) searches and rewrites cell SOURCES: `WindowedSheetModel.findAll`
(over the C ABI's `sc_find_all`) returns the A1 addresses whose formula text contains
the query (case-insensitive) and **Find** jumps the selection to the first hit
(`selectA1` parses column letters past Z); `WindowedSheetModel.replaceAll` (over
`sc_replace_all`) rewrites the query → replacement in every cell's source and
recomputes, with the footer echoing the match / replace count. Because the engine
re-parses each rewrite through its centralised coerce (`set_raw`), a rewritten formula
stays live (`H1`→`H2` turns `=H1+5` into a recomputed `=H2+5`) and a rewritten literal
stays typed (`15`→`99` re-totals every dependent).

### Visual design

`InfiniteGridView` mirrors the **reference visual language** defined by the web
demo (`demo/visicalc-html/infinite.html`) so every VisiCalc backend reads as one
considered dark, modern-spreadsheet surface — the same token set the Qt, Flutter,
Compose, and XAML ports use. The palette lives in a small set of `Color` design
tokens at the top of the view (`cBg`/`cPanel`/`cSurface`/`line`/`ink`/`muted`/
`accent`…), echoing the web demo's CSS custom properties. From those it builds: a
panel-wrapped **toolbar** with an address **pill**, an italic `fx` marker, then a
grown formula field with an accent **focus ring** (a `@FocusState`-driven 2-px
overlay stroke); the actions are **segmented button groups** (drag-fill ·
clipboard · file · history · find/replace) — a reusable `ChipButtonStyle` with
hover/pressed/disabled states, plus compact find/replace text fields — separated
by thin rules. The grid gets subtle **zebra** row
banding, a 2-px **accent selection ring**, and the selected cell's **row + column
headers tint to the accent**; a hairline-separated **status footer** echoes the
live virtual-grid size and revision.

Headless proof: `Tests/VisiCalcTests/WindowedModelTests.swift` asserts the
window is engine-computed and dense, a formula 1000 rows down (`Z1000` = 39) is
reachable, the gaps are empty (sparse), column letters run AA/BA/BB, editing
`A1` dirties the far dependent `Z1000` via `changedSince`, `fillDown`
replicates a relative formula down a column (`I1 = =H1*10` filled down ⇒ I2 = 30,
I3 = 40, source I1 = 20 untouched), and the clipboard (copy `I1 = =H1*2` → paste
at I2 ⇒ I2 = H2*2 = 14; cut A1 → move to C1, A1 clears, a second paste is a
no-op), and a save/load round trip (`saveBook` → mutate A1 ⇒ E1 523.00 →
`loadBook` restores A1 15 / E1 38.00, the loaded formula stays live with A1=5 ⇒
E1 28.00, and malformed input is rejected), and an undo/redo walk (two edits →
undo both → redo both with the formula recomputing live → a fresh edit forks
history), and **find / replace** (`findAll("15")` locates the one literal `A1`, a
case-insensitive `findAll("sum")` finds the total formulas, empty / no-match queries
return nothing, `selectA1("Z1000")` parses a far address, and `replaceAll` rewrites
both a literal `15`→`99` ⇒ E1 122.00 and a formula reference `H1`→`H2` ⇒ `=H1+5`
recomputes to 25, keeping each live). Run with `swift test`.

## Notes

- The grid and formula bar are now both pipeline-generated (the SwiftUI Grid
  emitter and the FormulaBar `commit` arity, both noted as gaps in earlier
  versions of this README, have since landed).
- Known engine gap (tracked separately): `SUM` over a range containing an error
  cell does not propagate the error (Excel does); binary operators do.
- Known emitter gap: the SwiftUI views are display-only (constant bindings, no
  tap handlers), so this demo supplies interactivity in the host layer rather
  than through the generated views.
