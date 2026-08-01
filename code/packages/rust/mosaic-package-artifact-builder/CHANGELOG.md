# Changelog

## [Unreleased] - runnable host-surface shell acceptance

- Compose Desktop project shells now resolve `node` slots from an optional
  in-process `MosaicHost` props map as composable lambdas.
- Venture's exhaustive `Backend::ALL` gate now verifies both the generated
  component mount and the runnable project-shell host-injection path for all
  nine backends.

## [Unreleased] - exhaustive Venture backend acceptance

- Added `Backend::ALL` as the MIL/MLL/MSL package pipeline's backend source of
  truth.
- Venture's shared browser package now builds a project shell and proves a real
  `HostSurface` mount across every listed backend, including Qt.

## [Unreleased] - preserve XAML emitter support files

XAML package builds now write emitter-owned C# support files, report them in
`BuildResult.artifacts`, and include them in `MosaicPackage.props`. Generated
ViewModels and value converters referenced by component XAML therefore travel
through the same package pipeline as the component triple.

## [Unreleased] - theme axis for style resolution

`BuildOptions` gains a `theme: Option<String>` field — the style (`.msl`)
analogue of the UI30 layout `variant` axis. When `Some("light")`, each
component's style resolves from `<Component>.light.msl` (falling back to the
bare `<Component>.msl`, then the alphabetically-first stylesheet). `None`
preserves the historical theme-agnostic resolution (bare, else
alphabetically-first — the implicit dark default).

Before this, `resolve_style_path` was theme-blind: it picked the bare `.msl`
or the alphabetically-first `<Component>.*.msl`, so `<Component>.dark.msl`
always beat `<Component>.light.msl` and any authored light stylesheet was
**dead code, never emitted**. The theme flows through `compile_one_component`,
`emit_project_shell`, and the dependency-style collection chain, so app styles,
component styles, and nested package-dependency styles all honour the selected
theme. `mosaic-compile pkg` exposes it as `--theme <name>`.

`build_package` validates `opts.theme` as a safe path segment (non-empty ASCII
alphanumeric / `_` / `-`) before any I/O, since the theme flows into a
stylesheet filename joined onto `src/`. The check lives in the library (not just
the `mosaic-compile` CLI) so programmatic callers can't traverse out of `src/`.

**Breaking:** `BuildOptions` now has a required `theme` field. All in-tree
constructors are updated (`None` = prior behaviour).

## [Unreleased] - package reference aware artifact builds

`build_package` now resolves `pkg::P::C` layout references before style
compilation and backend emission, using the shared `mosaic-package-resolver`
layout inliner. This lets app packages compose reusable component packages and
still emit backend artifacts from one app source tree.

`build_package` now installs backend-matching host assets declared in
`mosaic-package.toml` under `[host_assets]`, copying source files from the
package root into the emitted backend project after project-shell generation.
Manifest asset paths are validated to stay relative to the package/output root.
Generated HTML project shells automatically load copied JavaScript module host
assets before `main.js`, and generated React project shells automatically import
copied source-module host assets from `src/main.tsx`.

Package builds now write non-empty merged Mosaic styles as `<Component>.lattice`
sidecars beside each emitted component artifact and include those sidecars in
`BuildResult.artifacts`.

Dependency package styles are now compiled and merged into the consuming
component artifact before backend emission. Dependency styles are applied first
and the consuming component's own style is applied last, so parent/app packages
can intentionally override a named part while keeping default package styling.

The builder also now honors themed style fallbacks such as
`<Component>.dark.msl` when `<Component>.msl` is absent.

Electron project shells now delegate `mosaic:get-props` and
`mosaic:handle-event` IPC calls to an optional host module (`electron/host.ts`
compiled to `dist-electron/host.js`, source-side `electron/host.js` or
`electron/host.mjs`, or `MOSAIC_ELECTRON_HOST_MODULE`) instead of hardcoding
no-op responses. Their generated `npm run dev` script now compiles the Electron
main/preload TypeScript before launching Electron, so a fresh emitted project is
runnable without a separate build.

`BuildOptions::emit_project` now writes XAML project shells through
`mosaic-emit-xaml` as well, producing `<Component>.csproj`, `App.xaml`,
`MainWindow.xaml`, `app.manifest`, `build.ps1`, and README side files beside
the package's component XAML triple and `MosaicPackage.props` fragment.

`Backend::Compose` is now wired through package builds, emitting per-component
`.kt` files, a lightweight `index.kt`, and a README for adding the generated
sources to Android, Desktop, or Compose Multiplatform source sets.

All notable changes to `mosaic-package-artifact-builder` will be documented
in this file.

## [Unreleased] — UI32-M — multi-backend project-shell emission

L8 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2-L7: #4297, #4309, #4315, #4319, #4325, #4326). Adds `BuildOptions::emit_project: bool` so `build_package` produces a per-backend runnable project shell alongside the per-component artifacts.

`mosaic-compile pkg --backend <X> --emit-project --output dist <package>` now writes the same shell side-files the single-component `mosaic-compile --backend <X> --emit-project` path produces (L2-L7 PRs), with the package's first component mounted as the shell root.

New API:

- `pub struct BuildOptions { ..existing.., emit_project: bool }`
- `fn emit_project_shell(component, src_dir, backend_dir, backend) -> Result<Vec<PathBuf>, BuildError>` — re-parses the first component's `.mil`/`.mll`/`.msl` triple and routes through the matching emitter's `from_pipeline_with_options(emit_project: true)`. Writes the resulting `ProjectFiles` into `backend_dir` at the fixed §2.2 paths.

Per-backend dispatch covers React, HTML, WebComponent, Flutter, Qt, SwiftUI, and XAML. XAML shells reuse `mosaic-emit-xaml`'s `EmitOptions::emit_project` path and are written by the artifact-builder beside the component triple.

**v1 scope (documented deviation):** only the FIRST component in `[components].exports` is mounted as the shell root. Per UI32 spec §5 open question 1's first-export-default policy. Multi-component routing/tabs UI (TabView on SwiftUI, MaterialApp routes on Flutter, etc.) is deferred to UI32-M.1.

5 new tests cover:

- `ui32_m_emit_project_false_does_not_emit_shell_side_files` (§3.4 back-compat)
- `ui32_m_emit_project_true_writes_react_vite_shell` (positive: full L2 shell present, banner intact, artifacts list includes shell files)
- `ui32_m_emit_project_true_produces_expected_shell_per_backend` (cross-backend: 7 backends × expected file enumeration)
- `ui32_m_emit_project_true_xaml_writes_project_shell` (positive: full WinUI host shell present)
- `ui32_m_emit_project_shell_is_byte_deterministic` (§3.1 across two tmpdir runs)

All 33 existing tests + 1 doctest pass unchanged. Total tests: 38 (was 33, +5).

The existing 25+ `BuildOptions { ... }` construction sites in tests + the module doctest were updated to add `emit_project: false`.

## [Unreleased] — UI31-M Phase 3 multi-component HTML shell

`build_package` for `Backend::Html` now writes a second index file
alongside the existing bare `index.html`:

- **`html/index-shell.html`** — a complete `<!DOCTYPE html>` document
  that inlines every component's emitted `.html` fragment inside a
  `<section data-component="X">` block. Opening it in a browser
  shows the whole package laid out top-to-bottom; no demo-side
  boilerplate required.

This eats the shell that today's VC2-html demo hand-writes (the
demo's `index.html` currently inlines a hand-written `<table>` for
Grid because the Mosaic pipeline didn't produce a mountable HTML
shell). With this change the demo's wrapper can be replaced by
the auto-generated `index-shell.html`.

Back-compat: the bare `index.html` (a comment-only manifest of
components) is unchanged. Any tool already consuming it sees no
diff. The new file is additive.

Scope note: this PR ships only the HTML shell. The matching
WebComponent shell (`webcomponent/index.html` that loads the
existing `index.js` and instantiates `<mosaic-{name}>` per
component) and the XAML `MainWindow.xaml` shell are queued for a
follow-up PR — same pattern, different per-backend output shape.

1 new test (`html_backend_writes_multi_component_index_shell_in_addition_to_bare_index`).
Total tests: 33 (was 32).

## [Unreleased] — UI30 multi-layout variant enumeration (ML2)

`build_package` now emits one artifact per (component, variant, backend)
tuple. Implementation follows UI30 spec §5: filesystem is the source
of truth. A new `discover_variants()` helper scans the package's
`src/` for `<Component>.<variant>.mll` files and the builder loops
over the discovered variants.

### Filename convention

- **Default variant** (bare `<Component>.mll` exists): output is the
  unsuffixed `<Component>.<ext>` — same name as pre-UI30 builds.
- **Named variants** (`<Component>.touch.mll` etc.): output is
  `<Component>.<variant>.<ext>`. The variant infix lands between the
  component name and the file extension so multiple variants coexist
  in one output directory without collision.

For XAML this means a single component can emit:
```
Grid.xaml          Grid.touch.xaml          (default + variant XAML)
Grid.xaml.cs       Grid.touch.xaml.cs       (matching code-behinds)
Grid.Event.cs      Grid.touch.Event.cs      (matching event unions)
```

### Back-compat clause

Every existing package — toolkit, dialog, the ones with one
`.mll` per component — builds byte-for-byte identically.
`discover_variants` returns `[None]` for a component with only a
bare default, the loop runs once, and the artifact filename is
unsuffixed exactly as before. Eight new tests cover this back-compat
path explicitly.

### Out of scope for this PR

The UI30 spec's `[variants]` manifest section (with `all` /
`overrides` / `fallback` keys) is **not** parsed here. Filesystem
discovery is sufficient for the "ship everything you authored"
default policy; manifest declarations are only needed when a
package wants to *constrain* which variants get built. Follow-up
PR will extend `mosaic-package-manifest` to parse the section and
wire it into the builder.

The variant-aware index file (mounting `Grid.desktop` and
`Grid.touch` as separate exports in the React/HTML/qmldir index)
is also deferred — the index continues to list each component
once, which is correct for the most common runtime-picks-variant
model (host imports either the default or the variant, never both).

### Tests

- `discover_variants_bare_default_only_returns_single_none` —
  back-compat for single-variant packages.
- `discover_variants_default_plus_named_returns_both_in_order`
  — default first, named variants alphabetical.
- `discover_variants_only_named_variants_no_default` — "strict
  mode" packages that omit the bare default.
- `discover_variants_no_mll_files_returns_single_none` —
  degenerate case still triggers the existing SourceNotFound
  error.
- `discover_variants_does_not_cross_pollute_components` — `Grid`
  doesn't pick up `Sidebar.touch.mll`.
- `discover_variants_skips_ambiguous_dotted_middles` —
  `Grid.dark.theme.mll` is rejected (dotted middle can't be a
  clean variant name).
- `build_package_emits_both_default_and_variant_artifacts` —
  end-to-end React build emits both `Grid.tsx` + `Grid.touch.tsx`.
- `build_package_without_variants_is_unchanged_from_pre_ui30` —
  explicit regression guard for the back-compat invariant.

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
