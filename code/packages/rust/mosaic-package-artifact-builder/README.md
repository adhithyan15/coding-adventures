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

The inventory identifies the passive drag/drop lowering on XAML,
native table lowerings without table semantics (excluding canonical UI31/Grid
shapes on Flutter, Compose, Qt, SwiftUI, and XAML), Flutter's
dialog placeholder and missing URL effect
host, ignored tri-state checkbox and radio-group properties, XAML dialog state
that still requires code-behind, ignored XAML/SwiftUI dialog lifecycle events,
ignored XAML/SwiftUI external-link activation events, and generated native
project shells that can fall back to sample props. Compose, Flutter, Qt, SwiftUI,
and XAML now have closed shells: their strict profiles
require Mosaic's standard Rust runtime, wait for the first props envelope,
reject missing required props, and omit sample-data and optional-host fallbacks.
The overall native-complete milestone remains open while ignored properties,
events, styles, effects, and
accessibility metadata are added to the inventory.

Dropped *style* properties (issue #12022) are a separate, non-gating list:
`DegradationReport::style_degradations`, written to the same JSON report as
`styleDegradations`. XAML's stylesheet lowering (`mosaic-emit-xaml::pipeline
::dropped_style_properties`) reports every mosstyle property it silently
discarded — currently over a hundred distinct properties across the
package-expanded TaskApp, from `box-shadow` (30 uses, no WinUI CSS-shaped
shadow) down to absolute positioning and per-side border shorthands. These
are deliberately kept OUT of `degradations`/`nativeComplete`: several are
real, already-accepted gaps (elevation tokens, dashed borders) that the
currently-green `native-complete` builds for TaskApp and
`mosaic-pkg-rating-controls` already ship with, and folding them into the
gating list today would break those builds rather than fix anything. Only
XAML is wired so far — SwiftUI/Compose/Qt/Flutter's own style lowering
hasn't been audited and gains no new degradations from this.

Flutter, Compose, Qt, SwiftUI, and XAML drag primitives are no longer reported as
inert: those emitters use native pointer/touch drag targets plus the UI35
keyboard, accepted-drop, component-scoping, and announcement contracts.

The package-expanded TaskApp is the full strict-profile proof point for Flutter,
Compose, Qt, SwiftUI on macOS, and XAML/WinUI. SwiftUI emits `Table` with dynamic columns on
macOS 14.4 / iOS 17.4 and a native `List` compatibility path on the package's
older deployment targets. Each strict desktop output has no known degradations,
bundles the standard Rust runtime, verifies that installed runtime against the
selected artifact, and launches without an injected library path. SwiftUI's iOS
16 build remains a separate source-portability gate rather than packaging the
macOS dylib. XAML also bundles the concrete `task-mosaic-app` adapter, verifies
the DLL installed beside `TaskApp.exe`, and drives the generated .NET binding with
that app-local engine. The complete TaskApp now passes XAML's strict
`native-complete` profile with zero degradations: its Sheet carries native UIA
table/grid semantics and its board/calendar use native WinUI drag, drop, keyboard,
acceptance, and accessibility behavior. Hosted Windows CI still does not claim a
visible interactive launch.

Property degradations carry the exact package-expanded node and property index.
For example, Compose/Flutter/SwiftUI report an authored, non-false
`HostCheckbox.indeterminate`, while Compose/Flutter/Qt/SwiftUI report
`HostRadio.group` until those emitters provide native mutual exclusion. An
explicit `indeterminate: false` is a semantic no-op and does not fail a strict
build.

Dialog/link degradations are likewise value-sensitive. SwiftUI's modal
`HostDialog.onClose` and XAML/SwiftUI internal-link `onActivate` dispatch are
supported, while SwiftUI's non-modal close event and external-link activation
events are rejected until those emitter paths preserve the authored behavior.

`compose_component` is the canonical in-memory entry point shared by package
builds and standalone three-file compilation. It returns the compiled model,
resolved layout, and merged style definition without selecting a backend.

`build_package_with_tokens` and
`build_package_with_profile_runtime_and_tokens` accept one resolved token map
for the full composition. The map is used for both the root package and every
recursive dependency style, so reusable controls inherit app branding without
copying or editing their MSL. Existing entry points retain the built-in Mosaic
palette for source compatibility.

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
binding and runtime-provided props before mounting the component. Use
`build_package_with_profile_and_runtime` with a target `.dylib`, `.so`, or `.dll`
to copy the selected Rust engine into Compose's platform-specific application
resources, Flutter's bundled Dart code assets, Qt's CMake install tree, SwiftUI's
SwiftPM resource bundle, or XAML's WinUI output directory under its conventional
`mosaic_app` filename. Flutter emits `hook/build.dart` so the Flutter toolchain
packages and resolves the selected target library without application-specific
runner edits. The Compose and SwiftUI bindings resolve their installed
resources, while the Qt and XAML bindings resolve the engine beside the
installed executable before global lookup. A strict project build on any of the
five native backends without that selection reports
`runtime.library-not-bundled`. Selecting a runtime also makes the generated
shell require that engine even under the permissive reporting profile. This
allows an app with unrelated, explicitly reported UI degradations to ship a
fail-closed engine boundary without retaining preview/sample props.
Every exported Compose component is mirrored into the generated Gradle source
set, so `gradle compileKotlin` type-checks the complete package even though the
shell mounts the manifest's first export as its entry component.
SwiftUI package shells likewise install Mosaic's standard Foundation host plus
a tiny C dynamic-loader target. A selected `.dylib` is copied into SwiftPM's
`Runtime` resource bundle and its `Bundle.module` path is passed to the generated
host; `MOSAIC_APP_LIBRARY` remains a development override. The generated host
owns the Rust handle, buffers, sequence, snapshots, and updates. Permissive builds
can fall back to the legacy reflection hook; `native-complete` builds require
the standard binding and runtime-provided props before mounting the view.
Every exported SwiftUI view is mirrored into the generated SwiftPM application
target, so `swift build` type-checks the complete package while `App.swift`
continues to mount the manifest's first export.
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
Every exported Flutter widget is mirrored into the generated application's
`lib/` source set, so `dart analyze lib` type-checks the complete package while
`main.dart` continues to mount the manifest's first export. Native CI also
bootstraps the documented Linux runner and builds the toolkit desktop app.
Qt project shells install a standard QObject host backed by `QLibrary` and
Qt JSON/variant APIs. Explicit package host assets can still replace
`MosaicHost.h/.cpp` for specialized native surfaces and effects. Permissive
builds retain the optional-host seam; `native-complete` builds compile the
standard binding unconditionally, validate Rust-provided required MIL props
before QML construction, and never mount an inert or sample-backed component.
Every exported QML component is listed in the generated
`qt_add_qml_module`, so CMake and Qt's QML cache compiler validate the complete
package while the application continues to mount the manifest's first export.

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
