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
    |-- Cell.tsx
    |-- Column.tsx
    `-- index.ts
```

It is the library underneath the `mosaic-compile pkg <root> --backend <name>
--output <dir>` CLI subcommand.

Package references such as `pkg::mosaic-pkg-card::Card` are inlined before
backend emission. Styles from referenced packages are compiled and merged first,
then the consuming package's style is applied, so apps get reusable component
defaults plus local override points.

Set `BuildOptions::emit_project` to `true` to write backend project shells
beside the component artifacts. React, HTML, WebComponent, Flutter, Qt,
SwiftUI, and XAML all produce their shell side files from the same package
source tree.

Electron project shells expose the same renderer-side `window.mosaicHost`
contract as React and route it through context-isolated IPC. The generated main
process can load an optional host module from `electron/host.ts` (compiled to
`dist-electron/host.js`) or from `MOSAIC_ELECTRON_HOST_MODULE`, so apps can
bind generated UI events to shared business logic without editing renderer
artifacts.

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

## Usage

```rust
use std::path::PathBuf;
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

let opts = BuildOptions {
    package_root: PathBuf::from("code/packages/mosaic-pkg-grid"),
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
  `-- Io(_)                  <- read / write / mkdir failed
```

## Layout Per Backend

| Backend | Files written |
| --- | --- |
| React | `react/<Component>.tsx`, `react/index.ts` |
| SwiftUI | `swiftui/<Component>.swift`, `swiftui/index.swift` |
| Qt | `qt/<Component>.qml`, `qt/qmldir` |
| HTML | `html/<Component>.html`, `html/index.html` |
| XAML | `xaml/<Component>.xaml` plus code-behind/events |
| Flutter | `flutter/<Component>.dart`, `flutter/index.dart` |
