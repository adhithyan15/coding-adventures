# toolkit-multi-demo — Bootstrap-shape components in WinUI 3

A runnable WinUI 3 application hosting four `mosaic-pkg-toolkit`
components — **Button**, **Alert** (dismissible), **Badge**, and
**Spinner** — every one of them auto-generated from
`.mil`/`.mll`/`.msl` triples via `mosaic-emit-xaml`.

This is the cross-platform proof that the v0.1–v0.11 Bootstrap-shape
toolkit lowers cleanly to real native UI — not just abstract markup.

## What's here

```
mosaic/                    — toolkit sources (copies of code/packages/mosaic-pkg-toolkit/src/)
  Button.mil, .mll, .light.msl
  Alert.mil,  .mll, .light.msl
  Badge.mil,  .mll, .light.msl
  Spinner.mil,.mll, .light.msl
winui/                     — auto-emitted XAML + hand-written shell
  Button.xaml(.cs)         — auto-emitted from mosaic/Button.{mil,mll,light.msl}
  Alert.xaml(.cs)          — same, with hand patches (see ISSUES.md)
  Badge.xaml(.cs)          — same, with hand patches
  Spinner.xaml(.cs)        — same, with hand patches
  *.Event.cs               — auto-emitted event unions
  BoolToVisibilityConverter.cs — auto-emitted converter
  App.xaml(.cs)            — WinUI 3 application shell (auto-emitted)
  MainWindow.xaml(.cs)     — hand-written multi-component host
  Button.csproj            — WindowsAppSDK 1.7 project
  app.manifest, build.ps1  — generated build infrastructure
ISSUES.md                  — the three mosaic-emit-xaml gaps caught here
```

## Prerequisites

1. **.NET SDK 9.0** — `dotnet --list-sdks` shows a `9.0.*` entry.
2. **Windows App Runtime 1.7** installed system-wide:
   `winget install Microsoft.WindowsAppRuntime.1.7`.
3. Optional: Visual Studio Build Tools 2022 with the WinUI workload.
   Without it the build emits one cosmetic `MSB4062` (missing
   AppxPackage MSBuild tasks); the `.exe` still produces correctly
   before the failing target runs.

## How to build and run

```powershell
cd demo\toolkit-multi-demo\winui
.\build.ps1
.\build.ps1 -Run
```

The window opens with the four components stacked vertically. A
status bar at the bottom echoes Dispatch events — click the Button
or the Alert's close `x` to see them fire.

## How the XAML files got here

For each toolkit component:

```sh
mosaic-compile --backend xaml \
  --interface mosaic/Component.mil \
  --layout    mosaic/Component.mll \
  --style     mosaic/Component.light.msl \
  --output    winui/Component.xaml
```

Plus an initial `--emit-project` invocation for Button to scaffold
the WinUI 3 project files (`Button.csproj`, `App.xaml`, etc.).

The `MainWindow.xaml(.cs)` files in this demo are **hand-written**
(replacing the `--emit-project` stub that hosts a single
component) to demonstrate multiple toolkit components living in the
same window.

## Hand patches required (for now)

Three mosaic-emit-xaml gaps surfaced when compiling the toolkit:

1. `BorderRadius` → `CornerRadius` (WinUI 3's actual property name)
2. `x:Name="Button"` collision with class `Button` → `x:Name="ButtonElement"`
3. `Foreground`/`FontSize`/`FontWeight` on `<Border>` (not valid;
   should cascade to inner content)

See [ISSUES.md](ISSUES.md) for the full breakdown and proposed fixes
in `code/packages/rust/mosaic-emit-xaml/src/pipeline.rs`. Once those
land, the patches in `winui/*.xaml` go away and the directory
regenerates cleanly.

## Why this is interesting

The same `Component.mil`/`.mll`/`.msl` triple compiles unchanged to
every backend Mosaic supports: React, SwiftUI, Qt, WebComponent,
HTML, **and** WinUI 3 via XAML. One Bootstrap-shape userland, every
target platform.
