# VisiCalc — XAML (WinUI 3) demo

Sixth and final per-backend cross-backend visual demo (Phase 2 /
VC2-xaml), running on WinUI 3 / .NET 9 (Windows-only).

## What it shows

A `Window` containing:

- An auto-generated `FormulaBar` UserControl (from
  `Generated/FormulaBar.{xaml,xaml.cs,Event.cs}`, produced by
  `mosaic-compile --backend xaml`).
- A hand-written 5×5 spreadsheet grid built programmatically in
  `MainWindow.xaml.cs`'s `BuildSampleGrid()`. Tap a cell to select
  it; the formula bar updates with its value.

Same hard-coded 5×5 sample data as the other VC2-* demos so the
WinUI render matches React/HTML/WebComp/Flutter/Qt/SwiftUI.

## How to build the generated FormulaBar

```bash
bash scripts/build.sh
```

Runs `mosaic-compile --backend xaml` against the Mosaic sources and
writes `Generated/FormulaBar.xaml`, `.xaml.cs`, and `.Event.cs`.

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
  `<gen:FormulaBar/>` and builds the sample grid programmatically.
- `app.manifest` — DPI-aware + Win10/11 compatibility shim.

Plan item [M] (Phase 3 — multi-component artifact-builder shells)
will extend `mosaic-package-artifact-builder` to generate the
multi-component MainWindow.xaml automatically. Until then, this
demo's hand-written `MainWindow.xaml.cs` is the reference for what
that emitter should produce.

## The Grid gap

Like the other VC2-* demos: the `mosaic-emit-xaml` pipeline doesn't
yet support the `Grid` built-in primitive. The 5×5 grid lives in
`MainWindow.xaml.cs`'s `BuildSampleGrid()`. When the XAML Grid
emitter lands, that hand-written code gets replaced with a
`<gen:Grid>` tag in `MainWindow.xaml`.

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
