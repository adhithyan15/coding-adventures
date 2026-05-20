# Changelog

All notable changes to `mosaic-package-artifact-builder` will be documented
in this file.

## [0.1.0] - 2026-05-19

### Added

- Initial release implementing **UI29 §4.3** (per-backend package-artifact
  build mode).
- `Backend` enum: `React`, `SwiftUI`, `Qt`, `WebComponent`, `Html`.
- `BuildOptions` (input), `BuildResult` (output), `BuildError` (failure
  modes).
- `build_package(opts)` entry point: parses
  `<package_root>/mosaic-package.toml`, compiles every exported
  component's three-file triple through `mosmodel-compiler` +
  `moslayout-compiler` + `mosstyle-compiler`, and hands the IRs to the
  chosen backend's `from_pipeline` function.
- Per-backend index emitters: `index.ts` for React, `index.swift` for
  SwiftUI, `qmldir` for Qt.
- Defensive fallback for missing `.msl`: synthesise an empty
  `style <Component> { }` source so the pipeline still produces a valid
  artifact.
- `WebComponent` and `Html` backends return `BuildError::UnsupportedBackend`
  pending their respective kernel-completion PRs.
- 14 unit tests covering empty packages, single/multi-component builds for
  all three wired backends, missing-source and malformed-source error
  paths, output-directory auto-creation, optional `.msl` fallback, and
  index/qmldir generation.
