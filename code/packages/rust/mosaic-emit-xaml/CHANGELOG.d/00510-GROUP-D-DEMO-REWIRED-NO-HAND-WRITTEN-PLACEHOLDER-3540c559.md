### Group D — demo rewired (no hand-written placeholder)

`code/programs/csharp/visicalc-xaml/`: `scripts/build.sh` now runs a second
`mosaic-compile --backend xaml` for the Grid (with
`--package-search-path code/packages`); `MainWindow.xaml` mounts
`<gen:Grid>`; `MainWindow.xaml.cs` feeds the generated control's
dependency properties + a `Dispatch` handler; `VisiCalc.csproj`
compiles the generated Grid files. The per-cell VM projection and
the selected/editing background highlight remain for a Windows dev
(see the demo README + `MainWindow.xaml.cs` TODO).

## [Unreleased] — #4548 toolkit-demo regressions — three emitter gaps closed

Three mosaic-emit-xaml code-gen bugs surfaced when compiling
components from `mosaic-pkg-toolkit` (Button / Alert / Badge / Spinner
demo, PR #4548) through the XAML backend. None of the existing
demos (hello-dialog, mosaic-pkg-grid) exercised the affected style
or naming surface. Each fix is a localised change with regression
tests; the toolkit Button + Alert + Badge XAML now regenerates
cleanly and builds without hand-patches.

