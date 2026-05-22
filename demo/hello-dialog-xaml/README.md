# hello-dialog-xaml — minimum end-to-end Mosaic → XAML → on-screen dialog

The first time we made a Mosaic component lower to XAML and render on
screen as a real WinUI 3 dialog. This directory is the working
reference for what the toolchain SHOULD produce, with the
generator-gap fixes applied by hand.

## What's here

```
mosaic/
  HelloDialog.mil       — interface (slots: title, message, open; emit: onClose)
  HelloDialog.mll       — layout (HostDialog → Column → Box[message]/Box[actions])
  HelloDialog.dark.msl  — style (mostly padding)
winui/
  HelloDialog.xaml      — hand-patched: ContentDialog root, x:Bind, no `mos:`
  HelloDialog.xaml.cs   — hand-patched: : ContentDialog, DialogTitle DP alias
  HelloDialog.Event.cs  — UNTOUCHED (matches what mosaic-emit-xaml produces)
  App.xaml(.cs)         — host application shell (hand-written)
  MainWindow.xaml(.cs)  — host window with "Open the dialog again" button
  HelloDialog.csproj    — WindowsAppSDK 1.7 + self-contained + PRI mitigations
  app.manifest          — DPI awareness + supported OS
ISSUES.md               — chronological catalog of every gap we hit, with fix
                          locations. Read this for the to-do list.
dialog-rendered.png — proof the end-state renders correctly
```

The `mosaic/` sources are unedited author input. The `winui/` files
are what `mosaic-compile --backend xaml --emit-project` *should*
produce — they're hand-patched in this snapshot to work around the
generator gaps documented in `ISSUES.md`.

## What you need

1. **.NET SDK 9.0 or 8.0** — `dotnet --list-sdks` should show one.
2. **Windows App Runtime 1.7** installed system-wide
   (`winget install Microsoft.WindowsAppRuntime.1.7`). Alternatively
   the `.csproj` is already configured for
   `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` so
   you don't strictly need this — the runtime DLLs bundle into the
   build output.
3. **Build tools** — Visual Studio Build Tools 2022 with the
   "Universal Windows Platform build tools" workload eliminates the
   MSBuild errors detailed below. Without it the build still produces
   a working `.exe`; it just complains.

## How to build and run

```powershell
# From repo root:
cd demo\hello-dialog-xaml\winui

# Optional: regenerate the Mosaic-derived files via the compiler.
# (Hand-patches will be lost — see ISSUES.md before regenerating.)
$compiler = "..\..\..\code\packages\rust\target\release\mosaic-compile.exe"
& $compiler `
    --interface ..\mosaic\HelloDialog.mil `
    --layout    ..\mosaic\HelloDialog.mll `
    --style     ..\mosaic\HelloDialog.dark.msl `
    --backend   xaml `
    -o          HelloDialog

# Build. Will print one MSBuild error about
# `Microsoft.Build.AppxPackage.RemovePayloadDuplicates` — ignore it.
# The .exe and all dependencies ARE produced before the failing
# packaging cleanup target runs.
dotnet build HelloDialog.csproj -c Debug

# Manually flatten native DLLs (dotnet build doesn't do this; only
# dotnet publish does). Without this step the .exe crashes on launch
# because Microsoft.WindowsAppRuntime.Bootstrap.dll isn't found.
Copy-Item bin\Debug\net9.0-windows10.0.19041.0\runtimes\win-x64\native\*.dll `
          bin\Debug\net9.0-windows10.0.19041.0\

# Run.
.\bin\Debug\net9.0-windows10.0.19041.0\HelloDialog.exe
```

When the window appears, click the **Open the dialog again** button in
the bottom-right and the modal `<ContentDialog>` should pop up with
the Title "Hello from Mosaic" and the bound message. Click **Close**
or press **Esc** to dismiss it.

The status bar at the bottom-left echoes the `HelloDialogEvent.Close`
record as it dispatches back to the host — proof that the UI24
dispatch contract round-trips through the generated wiring.

## Known frictions (every one is in ISSUES.md with a fix location)

- The generated XAML is hand-patched in five places (A1–A5 in ISSUES.md).
- The `.csproj` has to be hand-written; `--emit-project` doesn't work yet (B1).
- Native DLLs require manual copying after build (B2).
- `dotnet build` emits one MSBuild error that doesn't affect the output (C1).

After the follow-up PRs land, this directory will get regenerated
from scratch by the toolchain and these workarounds disappear.
