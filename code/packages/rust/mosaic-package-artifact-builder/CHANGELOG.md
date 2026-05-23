# Changelog

All notable changes to `mosaic-package-artifact-builder` will be documented
in this file.

## [Unreleased] — Flutter backend wired

Adds `Backend::Flutter` so userland packages now compile to seven
backends total (the new Flutter target alongside the existing six).

### Added

- `Backend::Flutter` enum variant.
- Dispatch arm in `compile_one_component` calls
  `mosaic_emit_flutter::pipeline::from_pipeline`.
- `index.dart` aggregator that re-exports each component file
  (`export 'X.dart';` per component).
- Minimal `pubspec.yaml` so `flutter pub get` recognises the
  generated directory as a Flutter package. Package name is the
  kebab-case manifest name with `-` rewritten to `_` (Dart's
  package-name convention).
- `flutter_backend_writes_dart_per_component_with_pubspec` test.
- `multi_component_builds_on_all_newer_backends` (renamed from
  `_on_html_webcomponent_xaml`) now also exercises Flutter so a
  regression in any of the four newer backends fails fast.
- New `mosaic-emit-flutter` Cargo dep.

Test count: 23 → 24 passing.

## [Unreleased] — full backend coverage (HTML, WebComponent, XAML)

The first cut shipped React / SwiftUI / Qt and returned
`UnsupportedBackend` for the other three UI29 §4.3 backends. This
update wires HTML, WebComponent, and XAML so userland packages
compile to **all six backends** without per-backend code.

### Added

- New `Backend::Xaml` enum variant. WinUI 3 target; each component
  emits a three-file triple: `{Component}.xaml` (markup),
  `{Component}.xaml.cs` (code-behind partial), and
  `{Component}.Event.cs` (discriminated event union).
- `Backend::Html` now writes `{Component}.html` per component plus
  `index.html` (fragment-shaped aggregator with `<!-- Component:
  X -->` markers).
- `Backend::WebComponent` now writes `{Component}.js` per component
  plus `index.js` (re-imports each component's self-registration via
  `import "./X.js"`).
- `Backend::Xaml` writes a `MosaicPackage.props` MSBuild fragment
  that a host's `.csproj` can `<Import Project="..."/>` to wire
  every component's `.xaml` + `.xaml.cs` + `.Event.cs` into the
  build in one line. Gets the `<DependentUpon>` linkage right so
  Visual Studio nests the partials under the markup file in the
  Solution Explorer.

### Changed

- `Backend::component_extension` now returns `Some(...)` for every
  variant (was `None` for `Html` / `WebComponent`). The `Option`
  shape is preserved so a future "manifest-only" backend can still
  slot in.
- The early-validation guard in `build_package` is kept as a
  future-proof check against adding a new `Backend::Foo` variant
  without wiring `compile_one_component`.
- Cargo deps grew by three: `mosaic-emit-html`,
  `mosaic-emit-webcomponent`, `mosaic-emit-xaml`.

### Removed

- The `webcomponent_backend_is_unsupported` and
  `html_backend_is_unsupported` regression tests (they pinned the
  rejection path that no longer exists). Replaced by the positive
  tests below.

### Security

Added explicit `validate_component_name` and `validate_package_name`
helpers that run at the top of `build_package`, before any I/O.
Component names from the manifest flow into:

- File paths via `out_dir.join(format!("{component}.{ext}"))` — a
  malicious manifest entry like `../../etc/passwd` would escape
  the dist directory.
- The generated `index.html`, `index.js`, and
  `MosaicPackage.props` files — a name like `Grid"; alert(1)//`
  or `Grid --><script>` would inject into the aggregated
  comment/import/XML.
- The XAML branch writes THREE files per component, tripling the
  blast radius.

The TOML parser catches some malformed names (`"` characters) at
manifest-parse time, but quote-free traversal/injection (`../foo`,
`Grid<script>`) sneaks past. The validators are the second line of
defence — they enforce strict `[A-Za-z][A-Za-z0-9_]*` for
components (matches the PascalCase convention every existing
package uses) and `[a-z][a-z0-9-]*` for packages (matches the
`mosaic-pkg-*` kebab convention). New `BuildError::UnsafeName`
variant carries `kind` (`"component"` / `"package"`), the
offending name, and a one-line reason.

The vector was caught during the U29-2 follow-up security review;
the existing React/SwiftUI/Qt paths were affected too — the
validators close the vector for all six backends at once.

### Tests

- 4 new positive tests:
  - `html_backend_writes_html_fragment_per_component`
  - `webcomponent_backend_writes_js_per_component`
  - `xaml_backend_writes_triple_per_component_and_props_fragment`
  - `multi_component_builds_on_html_webcomponent_xaml`
- 6 new security-boundary tests:
  - `component_name_with_path_traversal_is_rejected`
  - `component_name_with_slash_is_rejected`
  - `component_name_with_injection_characters_is_rejected`
  - `package_name_validation_rejects_unsafe_shapes`
  - `standard_component_names_pass_validation`
  - `standard_package_names_pass_validation`
- Test count: 13 → 23 passing.

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
