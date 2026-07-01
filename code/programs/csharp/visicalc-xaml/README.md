# VisiCalc — XAML (WinUI 3) demo (live, on the Rust engine)

The WinUI 3 / .NET 9 VisiCalc demo (Windows-only GUI), now **computing on the
shared Rust `spreadsheet-core` engine** through its C ABI (`spreadsheet-capi`),
reached via **P/Invoke** — the path WinUI / XAML use. The same engine the
SwiftUI and Qt demos link natively, the Flutter demo loads via dart:ffi, the
Compose demo reaches via Java FFM, and the web demos run as WebAssembly.

## What it shows

A `Window` containing the auto-generated `FormulaBar` and `Grid` UserControls
(`Generated/`, produced by `mosaic-compile --backend xaml`).

- The grid is fed **engine-computed** values: the classic cross-footing budget
  where column E totals each row, row 5 totals each column, and E5 is the grand
  total (169) — all formulas evaluated by the Rust engine, not hard-coded.
- `MainWindow.xaml.cs` feeds the generated control's dependency properties from
  `SpreadsheetModel` (`Engine.cs`); committing an inline cell edit calls
  `model.SetCell`, which writes to the engine and recomputes every dependent.

## How it's wired to the engine

```
WinUI controls (generated)  ──  SpreadsheetModel / SpreadsheetSession (Engine.cs)
   Grid.ViewportRows = …         │  sc_set_cell / sc_get_value … (P/Invoke, string↔char*)
                                 ▼
   native/spreadsheet_capi.dll   ←  spreadsheet-capi (Rust C ABI)  ←  spreadsheet-core
```

`Engine.cs` is deliberately free of any WinUI dependency (just
`System.Runtime.InteropServices` + `System.Text.Json`), so the same file
compiles into the WinUI app on Windows AND into the cross-platform console test
(`test/`) that proves the engine path on macOS/Linux.

## Verify the engine path (cross-platform)

The WinUI GUI is Windows-only, but the engine path is verifiable anywhere .NET 9
runs:

```bash
bash scripts/build.sh    # regenerate controls + build & vendor the engine (cdylib)
bash scripts/verify.sh   # runs test/ — P/Invokes the engine, asserts the grid is
                         #   engine-computed (E1=38, A5=39, E5=169), recomputes on
                         #   edit (E5 -> 269), and propagates =1/0 -> #DIV/0!
```

This is the .NET analog of the SwiftUI demo's `swift test`, the Qt demo's
`tst_model`, the Flutter demo's `flutter test`, and the Compose demo's
`verify.sh`. Verified green on macOS (12/12 checks).

## How to build the generated controls

```bash
bash scripts/build.sh
```

Runs `mosaic-compile --backend xaml` twice — once for the FormulaBar
and once for the Grid (the Grid invocation adds
`--package-search-path code/packages` so the compiler can resolve
`pkg::mosaic-pkg-grid::Grid`). It writes the FormulaBar triple and
the Grid triple + view-model records into `Generated/`.

## How to run the app (Windows only)

```cmd
winget install Microsoft.WindowsAppRuntime.1.7
dotnet build
dotnet run
```

Requires:
- Windows 10/11
- .NET 9 SDK
- WindowsAppRuntime 1.7 (installable via winget)

## Why not `--emit-project`?

`mosaic-emit-xaml --emit-project` produces a complete WinUI 3 shell
— but only for a SINGLE component. VisiCalc needs to host both
FormulaBar AND a Grid, so the shell here is hand-authored:

- `VisiCalc.csproj` — project file mirroring the `--emit-project`
  output, but with the FormulaBar's `<Page>`/`<Compile>` entries
  pointing at `Generated/`.
- `App.xaml` + `App.xaml.cs` — application bootstrap.
- `MainWindow.xaml` + `MainWindow.xaml.cs` — host shell that mounts
  `<gen:FormulaBar/>` and `<gen:Grid/>` and feeds them host state.
- `app.manifest` — DPI-aware + Win10/11 compatibility shim.

Plan item [M] (Phase 3 — multi-component artifact-builder shells)
will extend `mosaic-package-artifact-builder` to generate the
multi-component MainWindow.xaml automatically. Until then, this
demo's hand-written `MainWindow.xaml.cs` is the reference for what
that emitter should produce.

## The Grid (now generated)

The `mosaic-emit-xaml` pipeline now lowers the Grid from
`mosaic-pkg-grid` into valid WinUI 3 markup: CSS `px` units are
stripped from length setters, CSS-only properties (`border-collapse`,
`border-style`, `outline`, …) are dropped, `text-align` becomes
WinUI `TextAlignment` with PascalCase enum values, `font-weight`
becomes the `FontWeights` constant, and each column gets a fixed
pixel width via a `Width="{x:Bind Width}"` binding on the cell.

`MainWindow.xaml.cs` feeds the generated control:
`ColumnHeaders = ["", "A", "B", "C", "D", "E"]`, `ViewportRows`
(each row prefixed with its "1".."5" label), `ColumnWidths =
[48, 96, 96, 96, 96, 96]`, plus `Selected*/Edit*` and a `Dispatch`
handler for `Navigate / FormulaChange / EditCommit / EditCancel`.

**Remaining for a Windows dev** (this checkout is macOS, so it
cannot `dotnet build`): the per-cell VM projection that turns
`ViewportRows` into the `Grid_RowVm` / `Grid_VVm` instances the
nested `ItemsRepeater` templates bind (zipping each cell value with
its column index → `ColumnWidths[col]`). See the WINDOWS-DEV TODO
block in `MainWindow.xaml.cs` and the `<remarks>` on
`Generated/Grid_VVm.cs`. The selected/editing cell-background
highlight (state blocks) is also still React-only in the emitter.

## Infinite virtualized sheet

`SpreadsheetSession` (`Engine.cs`) also binds the engine's **viewport
primitive** over P/Invoke — `Window(r0,c0,r1,c1)` (a dense
`IReadOnlyList<IReadOnlyList<string>>` rectangle), `UsedRange()`,
`ColumnLetters()`, `CurrentRevision()`, and `ChangedSince()` — so a windowed,
virtualizing WinUI grid (an `ItemsRepeater` / `ListView`) can render only the
visible rectangle of an unbounded sheet (the .NET sibling of the
web/SwiftUI/Qt/Flutter/Compose infinite views), parsed with `System.Text.Json`.

### The scrollable infinite GUI (`InfiniteSheet.xaml` / `.xaml.cs`)

The **Infinite sheet** button in `MainWindow` toggles from the classic 5×5 grid
to `InfiniteSheet` — a virtualized, effectively-infinite (u32 × u32, sparse)
view rendered on the same engine. The body is a `ListView` whose `ItemsSource`
is just the row numbers (`1..TotalRows`); its `ItemsStackPanel` realizes a
container only for on-screen rows, and `ContainerContentChanging` fills each
realized row's cells from **one** engine `get_display_window` over its
`1×TotalCols` strip (`InfiniteSheetModel.RowCells`) — display strings, each
already rendered through its Excel-style format code (the seed formats the
cross-foot totals as `#,##0.00` and the far-flung `Z1000` total as a percent),
so the host paints them directly. Building the UI costs only the visible rows,
never the whole (millions-tall) sheet. Frozen chrome by scroll-sync: the
row-number gutter is a second virtualized `ListView` slaved to the body's
vertical scroll, and the column-letter header follows the body's horizontal pan.
Tap a cell → `SelectInf` (clamps, loads the source into the formula bar); press
Enter → `CommitInf` (writes through, recomputes dependents, regrows the extent).
The **"Fill ↓ 10"** button next to the formula bar calls
`InfiniteSheetModel.FillDown(10)` (over the C ABI's `sc_fill`) to replicate the
selected cell into the 10 rows below it — the engine shifts each copy's relative
references (`=A1`→`=A2`, …), pins absolute (`$`) refs, and carries the format.
The **Copy / Cut / Paste** buttons drive the engine's clipboard
(`InfiniteSheetModel.CopyCell`/`CutCell`/`PasteCell` over the C ABI's
`sc_copy`/`sc_cut`/`sc_paste`): copy the selected cell, then paste it elsewhere
with its relative references shifted by the destination's offset (absolute `$`
refs pinned, format carried); a cut clears the source on paste, and `PasteCell`
returns `false` (a no-op) for an empty clipboard.
The **Save / Load** buttons serialize the whole workbook
(`InfiniteSheetModel.SaveBook` over the C ABI's `sc_serialize`) to a JSON
document held in memory and restore it (`LoadBook` / `sc_deserialize`): the
document captures only the source (formula text + typed literals) and per-cell
formats — not the computed values, which the engine recomputes on load, so a
loaded formula stays live.
The **Undo / Redo** buttons walk the engine's snapshot history
(`InfiniteSheetModel.UndoEdit`/`RedoEdit` over the C ABI's `sc_undo`/`sc_redo`);
they enable/disable off `CanUndo`/`CanRedo` (refreshed after every edit). Every
edit is reversible and a restored formula recomputes live.
The **+ Row / − Row / + Col / − Col** buttons are **structural edits**
(`InfiniteSheetModel.InsertRow`/`DeleteRow`/`InsertCol`/`DeleteCol` over the C
ABI's `sc_insert_rows`/`sc_delete_rows`/`sc_insert_cols`/`sc_delete_cols`): insert
or delete the selected cell's row/column, and the engine shifts every formula
reference at or after the band so dependents keep pointing at their precedents
(`=A1+A2` with a row inserted above becomes `=A1+A3`); a reference whose whole band
is deleted becomes `#REF!`.
The **.00 / % / $ / Gen** buttons apply a number **format** to the selected cell
(`InfiniteSheetModel.ApplyFormat` over the C ABI's `sc_set_format`, with the code
`#,##0.00`, `0.0%`, `$#,##0.00`, or `""` to clear). The format is display-only —
the engine renders the stored value through the code, so the underlying number is
unchanged.
The **find / replace** group (a `find` box + a `replace` box + **Find** /
**Replace** buttons) searches and rewrites cell SOURCES: `InfiniteSheetModel.FindAll`
(over the C ABI's `sc_find_all`) returns the A1 addresses whose formula text contains
the query (case-insensitive) and **Find** jumps the selection to the first hit
(`SelectA1` parses column letters past Z); `InfiniteSheetModel.ReplaceAll` (over
`sc_replace_all`) rewrites the query → replacement in every cell's source and
recomputes, with the footer echoing the match / replace count. Because the engine
re-parses each rewrite through its centralised coerce (`set_raw`), a rewritten formula
stays live (`H1`→`H2` turns `=H1+5` into a recomputed `=H2+5`) and a rewritten literal
stays typed (`15`→`99` re-totals every dependent).

`InfiniteSheetModel` (in `Engine.cs`, WinUI-free) seeds far-flung sparse cells
(`Z1000`, `BA50`, `BB50`) and derives the extent from `UsedRange()` + a margin
(saturated in `long` then clamped to `int`, guarding the u32-overflow case).
The bottom **sheet tab bar** drives a multi-sheet **workbook**: the workbook
holds several sheets and bare-`A1` ops address the *active* one, while a formula
reaches **across** with a qualifier (`=Summary!B3`). Each tab is a chip (the
active one tints to the accent); **click** to switch, the active tab carries
inline **✎ rename** (a `ContentDialog`) and **✕ delete** affordances, and
**+ Sheet** adds one (`InfiniteSheetModel.SelectSheet`/`AddSheet`/`RenameSheet`/
`DeleteSheet` over the C ABI's `sc_set_active_sheet`/`sc_add_sheet`/
`sc_rename_sheet`/`sc_delete_sheet`, with `sc_sheet_names`/`sc_active_sheet`
reading the tab list). The seed adds a second sheet, **Summary** (`B3 = A1+A2 =
300`), and `Sheet1!G1 = =Summary!B3` pulls that value across; editing a Summary
input recomputes the cross-sheet dependent live. Renaming `Summary` rewrites
every referencing qualifier; deleting a referenced sheet turns the dangling
reference into `#REF!`, and the engine keeps at least one sheet (deleting the
last is a no-op). The single-sheet path is byte-identical — an unqualified `A1`
still means the active sheet.

#### Visual design

`InfiniteSheet.xaml` / `.xaml.cs` mirror the **reference visual language** defined
by the web demo (`code/programs/typescript/visicalc-html/infinite.html`) so every VisiCalc backend
reads as one considered dark, modern-spreadsheet surface — the same token set the
Qt, Flutter, and Compose ports use. The palette lives in a small set of
`SolidColorBrush` design tokens at the top of the code-behind (`Bg`/`Panel`/
`Line`/`LineStrong`/`Head`/`HeadSel`/`Ink`/`Muted`/`Accent`/`Sel`), echoing the
web demo's CSS custom properties. From those it builds: a panel-wrapped **toolbar**
with an address **pill**, an italic `fx` marker, then a grown formula field with
an accent **focus ring** (the wrapping `Border` thickens + tints to the accent on
`GotFocus`); the actions are **segmented button groups** (drag-fill · clipboard ·
file · history · find/replace) — a rounded `ToolChip` button style whose hover/pressed/disabled
chrome is recolored by overriding the stock WinUI `Button*` theme brushes in
`UserControl.Resources` (no custom `ControlTemplate`) — separated by thin rules.
The grid gets subtle **zebra** row banding (`rowNum % 2`), a 2-px **accent
selection ring**, and the selected cell's **row + column headers tint to the
accent** (the header is rebuilt and the realized gutter rows repainted on
selection); a hairline-separated **status footer** echoes the live virtual-grid
size and revision.

### Verification

The WinUI view itself is Windows-only — this macOS checkout cannot `dotnet
build` it (same boundary the classic Grid's VM-projection notes). The
engine-backed logic it drives is proven cross-platform by the headless console
harness:

`scripts/verify.sh` (the `test/` console harness, runs on macOS/Linux/Windows)
seeds far-flung sparse cells and asserts the window is engine-computed + dense
(A1=15, E1=38, E5=169), a formula 1000 rows down (`Z1000` = 39) is reachable, the
gaps are empty (sparse), column letters run AA/BA, and editing `A1` dirties the
far dependent `Z1000` via `ChangedSince`. It also drives `InfiniteSheetModel`
directly: `RowCells` one-read rows, `SelectInf` clamping + source load,
`CommitInf` recompute (A2 `8`→`108` ⇒ E2 151, A5 139, E5 269), drag-fill,
clipboard copy/cut/paste, and a save/load round trip (`SaveBook` → mutate A1 ⇒
E1 523.00 → `LoadBook` restores A1 15 / E1 38.00, the loaded formula stays live
with A1=5 ⇒ E1 28.00, and malformed input is rejected), and an undo/redo walk on
a fresh session (two edits → undo both → redo both with the formula recomputing
live → a fresh edit forks history). And it drives **find / replace**: `FindAll("15")`
locates the one literal (`A1`), a case-insensitive `FindAll("sum")` finds the total
formulas, empty / no-match queries return nothing, `SelectA1("Z1000")` parses a far
address, and `ReplaceAll` rewrites both a literal (`15`→`99` ⇒ E1 122.00) and a
formula reference (`H1`→`H2` ⇒ `=H1+5` recomputes to 25) keeping each live.
Finally a **multi-sheet** section proves the workbook over the C ABI: a
cross-sheet `=Summary!B3` computes and stays live as its precedent changes (300
→ 350), a rename rewrites the referencing qualifier (`Summary` → `Totals`),
deleting a referenced sheet yields `#REF!` (and the last sheet can't be deleted),
and the `InfiniteSheetModel` seed exposes `Sheet1`/`Summary` with a live cross-ref.

## Where this fits in the cross-backend demo plan

| Phase | Demo | Status |
|---|---|---|
| 2 | VC2-html | ✅ |
| 2 | VC2-webcomp | ✅ |
| 2 | VC2-flutter | ✅ |
| 2 | VC2-qt | ✅ |
| 2 | VC2-swiftui | ✅ |
| 2 | VC2-xaml (this one) | ✅ |
| 3 | multi-component artifact-builder shells | TODO ([M]) |
| 4 | code/programs/typescript/visicalc-all/ | TODO |

**Phase 2 is complete.** All six backends have a runnable VC2 demo
(or, for Windows-only platforms, the buildable scaffolding).
