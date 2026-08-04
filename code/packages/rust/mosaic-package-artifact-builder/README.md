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
Compose Desktop shells also subscribe to the optional host's prop-change
callback and include pinned JNA/JSON runtime support, so package-owned native
adapters can publish live content-surface state back into generated controls.

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
  coordinates that resolver during artifact builds.
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
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

let opts = BuildOptions {
    package_root: PathBuf::from("code/packages/mosaic/mosaic-pkg-grid"),
    output_root: PathBuf::from("/tmp/dist"),
    backend: Backend::React,
    emit_project: false,
};
let result = build_package(&opts)?;
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
