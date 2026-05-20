# mosaic-package-artifact-builder

Per-backend package-artifact build mode for Mosaic packages, implementing
**UI29 §4.3** ("Compiling a package").

## What it does

Given a Mosaic package on disk (a directory with `mosaic-package.toml`
plus a `src/` tree of `.mil` / `.mll` / `.msl` triples), this crate
compiles every exported component to the requested backend and writes a
backend-shaped artifact tree:

```
<output_root>/
└── react/             # or swiftui/, qt/, ...
    ├── Grid.tsx
    ├── Cell.tsx
    ├── Column.tsx
    └── index.ts       # or index.swift, qmldir
```

It is the library underneath the `mosaic-compile pkg <root> --backend
<name> --output <dir>` CLI subcommand.

## What it is not

- **Not a resolver.** It compiles every component in isolation against
  its own triple. Cross-package `<Grid />` resolution belongs to
  `mosaic-package-resolver`.
- **Not a modifier of emitter crates.** It consumes their public
  `from_pipeline(interface, layout, style)` entry points as opaque
  IR-to-string lowerings.

## Wired backends

| Backend       | Extension | Status                                    |
|---------------|-----------|-------------------------------------------|
| React         | `.tsx`    | wired                                     |
| SwiftUI       | `.swift`  | wired                                     |
| Qt (QML)      | `.qml`    | wired                                     |
| WebComponent  | —         | returns `UnsupportedBackend` (pending UI) |
| HTML          | —         | returns `UnsupportedBackend` (pending UI) |

## Usage

```rust
use std::path::PathBuf;
use mosaic_package_artifact_builder::{build_package, BuildOptions, Backend};

let opts = BuildOptions {
    package_root: PathBuf::from("code/packages/mosaic-pkg-grid"),
    output_root:  PathBuf::from("/tmp/dist"),
    backend:      Backend::React,
};
let result = build_package(&opts)?;
for path in &result.artifacts {
    println!("wrote {}", path.display());
}
# Ok::<(), mosaic_package_artifact_builder::BuildError>(())
```

## Error surface

```
build_package(...)
    │
    ├── Manifest(_)            ← <package_root>/mosaic-package.toml broken
    ├── UnsupportedBackend(_)  ← WebComponent / Html (not yet wired)
    ├── MissingComponent       ← reserved for cross-package checks
    ├── SourceNotFound         ← .mil/.mll missing under src/
    ├── PipelineError          ← mosmodel / moslayout / mosstyle / emitter failed
    └── Io(_)                  ← read / write / mkdir failed
```

## Layout per backend

| Backend  | Files written                                    |
|----------|---------------------------------------------------|
| React    | `react/<Component>.tsx`, `react/index.ts`         |
| SwiftUI  | `swiftui/<Component>.swift`, `swiftui/index.swift`|
| Qt       | `qt/<Component>.qml`, `qt/qmldir`                 |
