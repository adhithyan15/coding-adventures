# mosaic-package-artifact-builder

Per-backend package-artifact build mode for Mosaic packages, implementing
UI29 "Compiling a package".

## What It Does

Given a Mosaic package on disk, this crate compiles every exported component to
the requested backend and writes a backend-shaped artifact tree:

```text
<output_root>/
`-- react/             # or swiftui/, qt/, html/, xaml/, flutter/
    |-- Grid.tsx
    |-- Grid.lattice
    |-- Cell.tsx
    |-- Cell.lattice
    |-- Column.tsx
    |-- Column.lattice
    `-- index.ts
```

It is the library underneath the `mosaic-compile pkg <root> --backend <name>
--output <dir>` CLI subcommand.

## Native-completeness profiles

`build_package_with_profile` adds an explicit completion policy to package
builds. `BuildProfile::Permissive` emits the normal artifacts and a deterministic
`<backend>/mosaic-degradations.json`. `BuildProfile::NativeComplete` writes the
same report but rejects the build before application artifacts are emitted when
the selected backend has a known degradation.

The inventory identifies passive drag/drop lowerings, native table lowerings
without table semantics, Flutter's dialog placeholder and missing URL effect
host, ignored tri-state checkbox and radio-group properties, and generated
native project shells that can fall back to sample props. Compose, Flutter, Qt,
SwiftUI, and XAML now have closed shells: their strict profiles
require Mosaic's standard Rust runtime, wait for the first props envelope,
reject missing required props, and omit sample-data and optional-host fallbacks.
The overall native-complete milestone remains open while ignored properties,
events, styles, effects, and
accessibility metadata are added to the inventory.

Property degradations carry the exact package-expanded node and property index.
For example, Compose/Flutter/SwiftUI report an authored, non-false
`HostCheckbox.indeterminate`, while Compose/Flutter/Qt/SwiftUI report
`HostRadio.group` until those emitters provide native mutual exclusion. An
explicit `indeterminate: false` is a semantic no-op and does not fail a strict
build.

`compose_component` is the canonical in-memory entry point shared by package
builds and standalone three-file compilation. It returns the compiled model,
resolved layout, and merged style definition without selecting a backend.

Package references such as `pkg::mosaic-pkg-card::Card` are inlined before
backend emission. Styles from referenced packages are compiled and merged first,
then the consuming package's style is applied, so apps get reusable component
defaults plus local override points. Each non-empty resolved style map is also
written as a backend-agnostic `<Component>.lattice` sidecar beside the emitted
component artifact.

Set `BuildOptions::emit_project` to `true` to write backend project shells
beside the component artifacts. React, Electron, HTML, WebComponent, Flutter,
Compose, Qt, SwiftUI, and XAML all produce their shell side files from the same
package source tree. Typed `node` slots remain in-process host objects rather
than serialized scalars: the generated shell resolves the matching native
view, element, widget, composable, or QML/WinUI object through its optional
`MosaicHost` contract.
Compose Desktop shells install Mosaic's standard JNA runtime binding. The binding
owns the Rust application handle, buffer lifecycle, startup context, event
sequence, and JSON updates; applications no longer need to rebuild that FFI
adapter. Permissive builds try the binding before the legacy optional host hook
and retain sample props for previews. `native-complete` builds require the
binding and runtime-provided props before mounting the component.
SwiftUI package shells likewise install Mosaic's standard Foundation host plus
a tiny C dynamic-loader target. Set `MOSAIC_APP_LIBRARY` to the application
`cdylib` path (or package it as `libmosaic_app.dylib`); the generated host owns
the Rust handle, buffers, sequence, snapshots, and updates. Permissive builds
can fall back to the legacy reflection hook; `native-complete` builds require
the standard binding and runtime-provided props before mounting the view.
XAML project shells install a standard .NET host that uses built-in native
loading and JSON APIs. The generated window prefers that runtime when its DLL is
available, then retains the app-owned reflection host only as a permissive
compatibility fallback. `native-complete` builds instead load Rust before WinUI
activation, validate required MIL props before showing the component, and omit
reflection, sample-prop, and app-owned dispatch paths.
Flutter project shells replace the no-op host stub with the standard Dart FFI
runtime while preserving `MosaicApp(mosaicHost: ...)` injection. Set
`MOSAIC_APP_LIBRARY` to the Rust application library, or package it under the
target platform's conventional `mosaic_app` name. Permissive builds retain the
injectable/optional preview path; `native-complete` builds require the binding
and Rust-provided props before mounting the generated widget.
Qt project shells install a standard QObject host backed by `QLibrary` and
Qt JSON/variant APIs. Explicit package host assets can still replace
`MosaicHost.h/.cpp` for specialized native surfaces and effects. Permissive
builds retain the optional-host seam; `native-complete` builds compile the
standard binding unconditionally, validate Rust-provided required MIL props
before QML construction, and never mount an inert or sample-backed component.

Packages may declare optional `[host_assets]` file copies in
`mosaic-package.toml`. Matching backend assets are copied from package-relative
paths into the backend output directory after shell emission, so app packages
can attach host adapters without moving that file list into bespoke scripts.
For generated project shells, JavaScript module assets are also activated where
the backend has a conventional host hook: HTML module assets are loaded before
`main.js`, and React source modules under `src/` are imported from
`src/main.tsx`.

Electron project shells expose the same renderer-side `window.mosaicHost`
contract as React and route it through context-isolated IPC. The generated main
process can load an optional host module from compiled `dist-electron/host.js`,
source-side `electron/host.js` or `electron/host.mjs`, or from
`MOSAIC_ELECTRON_HOST_MODULE`, so apps can bind generated UI events to shared
business logic without editing renderer artifacts. The generated `npm run dev`
script compiles the Electron main/preload TypeScript before launching Electron
so a fresh emitted project is runnable without a separate build step.

## Boundaries

- Cross-package layout inlining lives in `mosaic-package-resolver`; this crate
  coordinates layout resolution and dependency-style composition for every
  compile entry point.
- Emitters stay backend-owned. This crate consumes their public
  `from_pipeline(interface, layout, style)` entry points.

## Wired Backends

| Backend | Extension | Status |
| --- | --- | --- |
| React | `.tsx` | wired |
| SwiftUI | `.swift` | wired |
| Qt (QML) | `.qml` | wired |
| WebComponent | `.js` | wired |
| HTML | `.html` | wired |
| XAML | `.xaml` | wired |
| Flutter | `.dart` | wired |
| Compose | `.kt` | wired |
| Electron | `.tsx` | wired |

## Usage

```rust
use std::path::PathBuf;
use mosaic_package_artifact_builder::{
    build_package_with_profile, Backend, BuildOptions, BuildProfile,
};

let opts = BuildOptions {
    package_root: PathBuf::from("code/packages/mosaic/mosaic-pkg-grid"),
    output_root: PathBuf::from("/tmp/dist"),
    backend: Backend::React,
    emit_project: false,
    theme: None,
};
let result = build_package_with_profile(&opts, BuildProfile::Permissive)?;
for path in &result.artifacts {
    println!("wrote {}", path.display());
}
# Ok::<(), mosaic_package_artifact_builder::BuildError>(())
```

## Error Surface

```text
build_package(...)
  |-- Manifest(_)            <- mosaic-package.toml broken
  |-- UnsupportedBackend(_)  <- future backend without a compiler path
  |-- MissingComponent       <- reserved for cross-package checks
  |-- SourceNotFound         <- .mil/.mll missing under src/
  |-- PipelineError          <- mosmodel / moslayout / mosstyle / emitter failed
  |-- PackageReferenceError  <- pkg::P::C layout/style dependency failed
  |-- NativeIncomplete       <- strict profile found known degradations
  |-- UnsafeName             <- manifest name unsafe for output paths
  |-- UnsafePath             <- host asset path escaped package/output roots
  `-- Io(_)                  <- read / write / mkdir failed
```

## Layout Per Backend

| Backend | Files written |
| --- | --- |
| React | `react/<Component>.tsx`, `react/<Component>.lattice`, `react/index.ts` |
| SwiftUI | `swiftui/<Component>.swift`, `swiftui/<Component>.lattice`, `swiftui/index.swift` |
| Qt | `qt/<Component>.qml`, `qt/<Component>.lattice`, `qt/qmldir` |
| HTML | `html/<Component>.html`, `html/<Component>.lattice`, `html/index.html` |
| XAML | `xaml/<Component>.xaml` plus code-behind/events and `<Component>.lattice` |
| Flutter | `flutter/<Component>.dart`, `flutter/<Component>.lattice`, `flutter/index.dart` |
