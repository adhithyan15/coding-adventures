# hello-dialog-xaml — end-to-end Mosaic → XAML → on-screen dialog

A minimal Mosaic component lowered to XAML and rendered as a real WinUI 3
dialog. The `winui/` directory is **fully auto-generated** from the
`mosaic/` sources via `mosaic-compile --backend xaml --emit-project`.
No hand-patches anywhere.

When this directory first landed (#3906), the `winui/` files were
hand-patched around five generator gaps and three missing pieces of
infrastructure. Those eleven issues are catalogued in `ISSUES.md` and
were all closed by the follow-up PRs listed at the bottom of this
file. This directory was regenerated cleanly via the fixed toolchain.

## What's here

```
mosaic/
  HelloDialog.mil       — interface (slots: title, message, open; emit: onClose)
  HelloDialog.mll       — layout (HostDialog → Column → Box[message]/Box[actions])
  HelloDialog.dark.msl  — style (mostly padding)
winui/                  — ALL auto-generated; do not hand-edit
  HelloDialog.xaml      — <ContentDialog> root from mosaic-emit-xaml
  HelloDialog.xaml.cs   — partial class : ContentDialog
  HelloDialog.Event.cs  — discriminated event union (UI24)
  App.xaml(.cs)         — application shell (from --emit-project)
  MainWindow.xaml(.cs)  — host window with "Open the dialog" button
                          and dispatch-event status echo
  HelloDialog.csproj    — WindowsAppSDK 1.7 + framework-dependent
  app.manifest          — DPI awareness + supported OS
  build.ps1             — Clean / Build / Run driver
  README.md             — emitted README for the generated project
ISSUES.md               — historical catalog (now all resolved) of every
                          gap we hit while making this work the first time
dialog-rendered.png     — proof screenshot
```

## Prerequisites

1. **.NET SDK 9.0** — `dotnet --list-sdks` shows one matching `9.0.*`.
2. **Windows App Runtime 1.7** installed:
   `winget install Microsoft.WindowsAppRuntime.1.7`.
3. Optional: Visual Studio Build Tools 2022 with the WinUI workload.
   Without it `dotnet build` emits one cosmetic `MSB4062` error from
   missing AppxPackage MSBuild tasks; the `.exe` and dependencies still
   build correctly and the post-build `FlattenNativeRuntimeDlls` target
   runs (see the generated `.csproj`).

## How to build and run

```powershell
cd demo\hello-dialog-xaml\winui
.\build.ps1            # build
.\build.ps1 -Run       # build + launch the .exe
.\build.ps1 -Clean     # delete bin/obj
```

When the window appears, click **Open the dialog** in the bottom-right.
The modal `<ContentDialog>` pops up with the bound `Message` slot value
and a `Close` button. Pressing `Close` (or `Esc`) dismisses it and
fires `HelloDialogEvent.Close` to the host's `OnComponentDispatch`
handler in `MainWindow.xaml.cs`. The status bar at the bottom-left
updates to `Dispatch: Close` as proof the event round-tripped.

## Regenerating

```powershell
# From repo root:
$compiler = ".\code\packages\rust\target\release\mosaic-compile.exe"
& $compiler `
    --interface demo\hello-dialog-xaml\mosaic\HelloDialog.mil `
    --layout    demo\hello-dialog-xaml\mosaic\HelloDialog.mll `
    --style     demo\hello-dialog-xaml\mosaic\HelloDialog.dark.msl `
    --backend   xaml `
    --emit-project `
    -o          demo\hello-dialog-xaml\winui\HelloDialog
```

All 11 files in `winui/` are produced by that single command. Edit the
`mosaic/` sources to change the component's interface, layout, or
style; rerun to regenerate.

## The journey

`ISSUES.md` documents each of the eleven gaps we hit and closed:

| # | Layer | Status |
|---|---|---|
| A1 | Generator: HostDialog → ContentDialog root | ✅ #3910 |
| A2 | Generator: drop undeclared `mos:` namespace | ✅ #3910 |
| A3 | Generator: `{x:Bind}` consistency for Title | ✅ #3910 |
| A4 | Generator: slot/base-class collision alias | ✅ #3910 |
| A5 | Generator: auto-emit BoolToVisibilityConverter.cs | ✅ #3910 |
| B1 | Generator: `--emit-project` flag | ✅ #3917 |
| B2 | Generator: auto-copy native DLLs (MSBuild target) | ✅ #3917 |
| B3 | Generator: build driver script | ✅ #3917 |
| C1 | WinUI SDK: missing AppxPackage tasks | Doc-only (lessons.md, README) |
| C2 | WinUI runtime: system install requirement | Doc-only (winget instruction) |
| D1 | Host code: ContentDialog XamlRoot from button | ✅ #3917 (template) |

`#3906` was the catalog. `#3910` shipped the five generator fixes.
`#3917` shipped `--emit-project` + the build script + the native-DLL
copy MSBuild target. This PR (the regen) confirms the toolchain
produces a working project end-to-end with no hand-patches.
