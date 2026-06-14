# VisiCalc — XAML (WinUI 3) demo

Sixth and final per-backend cross-backend visual demo (Phase 2 /
VC2-xaml), running on WinUI 3 / .NET 9 (Windows-only).

## What it shows

A `Window` containing:

- An auto-generated `FormulaBar` UserControl (from
  `Generated/FormulaBar.{xaml,xaml.cs,Event.cs}`, produced by
  `mosaic-compile --backend xaml`).
- An auto-generated `Grid` UserControl (from
  `Generated/Grid.{xaml,xaml.cs,Event.cs}` + `Grid_*Vm.cs` +
  `BoolToVisibilityConverter.cs`), produced by the SAME
  `mosaic-compile --backend xaml` pipeline from `mosaic-pkg-grid`
  (`HostTable` + nested `For` + `Cell`). There is no hand-written
  grid anymore — `MainWindow.xaml.cs` feeds the generated control's
  dependency properties and handles its `Dispatch` event.

Same hard-coded 5×5 sample data as the other VC2-* demos so the
WinUI render matches React/HTML/WebComp/Flutter/Qt/SwiftUI.

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
| 4 | demo/visicalc-all/ | TODO |

**Phase 2 is complete.** All six backends have a runnable VC2 demo
(or, for Windows-only platforms, the buildable scaffolding).
