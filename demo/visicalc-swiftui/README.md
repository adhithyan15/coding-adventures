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
`sc_get_window` / `sc_used_range` / `sc_changed_since` (the native sibling of the
web demo's `infinite.html`). The sheet is u32 × u32 and sparse; a two-axis
`ScrollView` sized from `used_range` drives the scrollbars, and only the visible
window of cells is built into the view via `SpreadsheetSession.window(...)`.

Headless proof: `Tests/VisiCalcTests/WindowedModelTests.swift` asserts the
window is engine-computed and dense, a formula 1000 rows down (`Z1000` = 39) is
reachable, the gaps are empty (sparse), column letters run AA/BA/BB, and editing
`A1` dirties the far dependent `Z1000` via `changedSince`. Run with `swift test`.

## Notes

- The grid and formula bar are now both pipeline-generated (the SwiftUI Grid
  emitter and the FormulaBar `commit` arity, both noted as gaps in earlier
  versions of this README, have since landed).
- Known engine gap (tracked separately): `SUM` over a range containing an error
  cell does not propagate the error (Excel does); binary operators do.
- Known emitter gap: the SwiftUI views are display-only (constant bindings, no
  tap handlers), so this demo supplies interactivity in the host layer rather
  than through the generated views.
