# Changelog

## [Unreleased] - narrow the `Path` degradation to also exclude Compose (#12028 item 3, UI39)

Compose now lowers `Path`'s `circle`/`line`/`curve` kinds to real
Jetpack Compose vector geometry (`mosaic-emit-compose`). Narrowed the
`("Path", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Qt | Backend::Flutter)` to
also exclude `Backend::Compose`, matching `HostSlider`'s per-backend
narrowing pattern. Same primitive-level (not per-kind) caveat as the
XAML/Qt/Flutter narrowings: a real build using `kind: arc` on Compose
still hard-errors from the emitter itself. SwiftUI is now the only
native backend still reporting this degradation.

## [Unreleased] - narrow the `Path` degradation to also exclude Flutter (#12028 item 3, UI39)

Flutter now lowers `Path`'s `circle`/`line`/`curve` kinds to real Dart
widget/`CustomPaint` geometry (`mosaic-emit-flutter`). Narrowed the
`("Path", ...)` arm in `collect_native_degradations` from
`!matches!(backend, Backend::Xaml | Backend::Qt)` to
`!matches!(backend, Backend::Xaml | Backend::Qt | Backend::Flutter)`,
matching `HostSlider`'s per-backend narrowing pattern. Same
primitive-level (not per-kind) caveat as the XAML/Qt narrowings: a real
build using `kind: arc` on Flutter still hard-errors from the emitter
itself.

## [Unreleased] - narrow the `Path` degradation to also exclude Qt (#12028 item 3, UI39)

Qt now lowers `Path`'s `circle`/`line`/`curve` kinds to real QML vector
geometry (`mosaic-emit-qt`). Narrowed the `("Path", ...)` arm in
`collect_native_degradations` from `backend != Backend::Xaml` to
`!matches!(backend, Backend::Xaml | Backend::Qt)`, matching
`HostSlider`'s per-backend narrowing pattern. Same primitive-level
(not per-kind) caveat as the XAML narrowing: a real build using
`kind: arc` on Qt still hard-errors from the emitter itself.

## [Unreleased] - narrow the `Path` degradation to exclude XAML (#12028 item 3, UI39)

XAML now lowers `Path`'s `circle`/`line`/`curve` kinds to real vector
geometry (`mosaic-emit-xaml`). Narrowed the `("Path", ...)` arm in
`collect_native_degradations` with `backend != Backend::Xaml`, matching
`HostSlider`'s per-backend narrowing pattern exactly. This is a
primitive-level flag, not per-kind — a real build using the
not-yet-implemented `arc` kind on XAML still hard-errors from the
emitter itself with a named message, it just isn't reflected as a
separate degradation code (matching how `HostSlider`'s own arm doesn't
distinguish authored prop combinations either).

## [Unreleased] - degradation plumbing for the `Path` kernel drawing primitive (#12028 item 3)

Added the `("Path", ...)` arm to `collect_native_degradations`, following
`HostSwitch`'s lifecycle exactly: unconditionally degraded (code
`primitive.path-unimplemented`) on every native backend the moment the
primitive is registered, since none renders it yet. As each backend lands a
real lowering (XAML first, immediately following this), narrow the arm's
`is_native()` guard with a `!matches!(backend, ...)` exclusion the same way
the existing `HostSlider` arm already does. See
`code/specs/UI39-mosaic-drawing-primitive.md`.

## [Unreleased] - gate the radio-group degradation on actual native support (#13007)

`mosaic-emit-compose`/`-flutter`/`-qt` now apply real mutual-exclusion
wiring (`selectableGroup`/synthesized `groupValue`/`ButtonGroup`) for a
literal `HostRadio.group` value shared by 2+ resolvable siblings.
Changed the `("HostRadio", "group")` match arm in
`ignored_native_property` to check a new `native_radio_groups:
&HashSet<String>` parameter — the set of literal group values that get
real wiring on the current backend — computed once per component (from
the whole layout tree, since the recursive degradation walk only ever
sees one node at a time and can't discover a node's siblings on its
own) via each backend's new `radio_groups_with_native_semantics`, and
threaded through `collect_native_degradations`'s recursion alongside
the existing `backend`/`component`/`variant` parameters. SwiftUI has no
idiomatic ancestor-grouping widget for N independently-bound `Toggle`s
and is deliberately excluded from this gating — it stays unconditionally
degraded (tracked as a follow-up). A `slot:`-bound group, or a literal
value with no qualifying peer, still reports the degradation on every
backend exactly as before.

Added `literal_radio_group_with_two_siblings_is_native_on_compose_flutter_qt_not_swiftui`,
covering the real `mosaic-pkg-deck-options`-shaped fixture (2 sibling
radios, one shared literal group) across all 5 backends.

## [Unreleased] - gate the checkbox-indeterminate degradation on actual native support (#13006)

`mosaic-emit-compose`/`-flutter`/`-swiftui` now lower `HostCheckbox`'s
`indeterminate:` to real native tri-state controls (#13006). Changed
the `("HostCheckbox", "indeterminate")` match arm in
`ignored_native_property` to call each backend's new
`host_checkbox_has_native_semantics` predicate (mirroring the existing
`host_table_has_native_semantics`/`host_dialog_has_native_semantics`
pattern), so `property.checkbox-indeterminate-ignored` is only reported
when the authored value is a shape none of the three emitters actually
act on (in practice, never — the toolkit's `Checkbox` component only
ever authors a `slot:`-bound value) rather than unconditionally for
every non-`false` `indeterminate` on these three backends.

Updated `native_degradation_analysis_reports_ignored_checkbox_and_radio_properties`'s
expected-degradations matrix: Compose/Flutter/SwiftUI now collapse to
just the still-open `property.radio-group-ignored` entry (#13007),
matching Qt's existing shape, and the strict-mode `degradation_count`
assertion for Compose drops from 2 to 1.

## [Unreleased] - gate Flutter's HostDialog degradation on the actual gap (#13010)

`mosaic-emit-flutter` now implements a real native dialog for `HostDialog`'s
default `modal: true` shape (#13010) — only `modal: false` still falls
back to a placeholder. Changed the `("HostDialog", backend == Flutter)`
match arm in `ignored_native_property` to call the new
`mosaic_emit_flutter::pipeline::host_dialog_has_native_semantics`
predicate, mirroring the existing `host_table_has_native_semantics`
pattern, so `interaction.dialog-placeholder` is only reported for the
genuinely-still-degraded `modal: false` case rather than unconditionally
for every `HostDialog` on Flutter.

## [Unreleased] - document HostDialog's XAML open-host-required gap as permanent (#13008)

Added a doc comment above the `("HostDialog", "open")` arm in
`ignored_native_property` recording that this degradation is confirmed
permanent, not an open TODO: WinUI3's `ContentDialog` has no bindable
`IsOpen`-style property the way `Popup`/`Flyout`/`TeachingTip` do, so
there's no declarative show/hide surface for the XAML emitter to bind
`open:` to — unlike SwiftUI/Qt/Compose, whose dialog primitives are all
natively declarative. No behavior change; the degradation code and
message are unchanged. Closes #13008.

## [Unreleased] - report dropped style properties, non-gating (#12022)

- New `DegradationReport::style_degradations` field (`styleDegradations` in
  `mosaic-degradations.json`), populated for the XAML backend by calling the
  new `mosaic_emit_xaml::pipeline::dropped_style_properties` per
  component/variant, right alongside the existing `collect_native_degradations`
  layout walk.
- Deliberately a separate field from `degradations`, not merged in:
  `native_complete`/the `NativeComplete` profile gate are computed from
  `degradations` alone and are completely unaffected. Regenerating the
  package-expanded TaskApp's XAML report locally shows *why* this matters —
  166 style properties are now visible for the first time (`box-shadow`,
  absolute positioning, per-side border shorthands, `transform`, and more),
  and several of them (30 `box-shadow` uses, 2 `border-style: dashed`) are
  real, already-shipped gaps in TaskApp's own stylesheet. Folding them into
  the gating list today would break the currently-green `native-complete`
  CI job for TaskApp and `mosaic-pkg-rating-controls`. This PR ships full
  detection + reporting (the invisible-failure-class problem #12022
  describes is fully fixed — nothing vanishes without a record anymore);
  the hard-fail is deferred until those gaps are addressed. See #12022 for
  the follow-up.
- Scoped to XAML only. SwiftUI/Compose/Qt/Flutter's own style lowering
  hasn't been audited and gains no degradations from this change.
- New tests: a `box-shadow` declaration on XAML is reported in
  `styleDegradations` while `degradations`/`nativeComplete` stay unaffected
  (`xaml_style_drop_is_reported_but_not_gating`); the same style on a
  non-XAML backend produces zero `styleDegradations`
  (`non_xaml_backend_does_not_report_style_drops`).

## [Unreleased] - HostSwitch capability tracking

- Native-complete analysis reports `primitive.switch-unimplemented` on every
  native backend until its real switch lowering ships.
- This keeps newly registered `HostSwitch` packages from being mislabeled as
  native-complete or silently lowered as checkboxes while emitter work proceeds.

## [Unreleased] - all-five-native HostSlider capability

- Native-complete analysis now accepts `HostSlider` on XAML after its native
  WinUI adjustable range-control lowering.
- `HostSlider` is now native-complete on Compose, Flutter, Qt, SwiftUI, and
  XAML; packages no longer need backend-specific slider implementations.

## [Unreleased] - SwiftUI HostSlider capability

- Native-complete analysis now accepts `HostSlider` on Compose, Flutter, Qt,
  and SwiftUI after their real adjustable range-control lowerings, while
  continuing to report `primitive.slider-unimplemented` on XAML.
- This keeps newly registered `HostSlider` packages from being mislabeled as
  native-complete while emitter work proceeds one backend at a time.

## [Unreleased] - default authored-child package expansion

- Package references splice their default inline MLL child block into a typed
  `node`/`list<node>` mount before backend emission.
- One acceptance fixture proves the expanded tree remains `native-complete` on
  SwiftUI, Qt/QML, XAML, Flutter, and Compose.
- A surviving child mount receives the stable
  `composition.child-slot-parameter-unimplemented` degradation, keeping direct
  standalone component artifacts honest until backend child parameters land.

## [Unreleased] - portable Text accessibility capability

- Native-complete analysis accepts the cross-backend `Text` contract for
  literal or slot-backed accessible names, heading/none roles, and static
  hidden state.
- Unsupported label forms, text roles, and dynamic hidden state now produce
  stable property-level degradation codes instead of being silently ignored.

## [Unreleased] - application token palettes

- Added token-aware composition and package-build entry points.
- One override map now applies to root and recursively referenced package
  styles, enabling reusable components to inherit app branding.
- Package manifests may declare scoped token defaults. Dependency palettes are
  lower precedence than consuming-package palettes, while explicit application
  input wins last; project-shell and degradation-analysis paths use the same
  resolved palette.
- Existing build and composition APIs retain the built-in Mosaic palette.

## [Unreleased] - XAML native drag capability

- Native-complete analysis recognizes XAML `HostDraggable` and
  `HostDropTarget` now that the emitter supplies native pointer, touch,
  keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- The package-expanded TaskApp now reports zero XAML degradations and can be
  emitted under the strict `native-complete` profile.

## [Unreleased] - XAML native table capability

- Native-complete analysis recognizes the canonical indexed UI31/Grid shape
  when the XAML emitter supplies native UIA Table/Grid and
  TableItem/GridItem provider patterns.
- Unsupported or structurally ambiguous XAML HostTable trees retain the stable
  `accessibility.table-semantics-missing` degradation.
- Concrete TaskApp XAML output retained four drag/drop paths at this historical
  milestone; the native drag capability above subsequently closes them.

## [Unreleased] - selected runtimes are required

- Passing `--runtime-library` now emits a runtime-required native shell even
  under the permissive degradation-reporting profile. This removes the sample
  fallback without suppressing unrelated capability reports, allowing XAML
  TaskApp to bundle its concrete engine while its drag/drop gaps remain explicit.

## [Unreleased] - SwiftUI native table capability

- Native-complete analysis recognizes the canonical dynamic UI31/Grid shape
  when the SwiftUI emitter supplies native `Table` / `TableColumnForEach`
  semantics and the version-gated `List` fallback.
- Unsupported or structurally ambiguous SwiftUI HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- Permissive TaskApp SwiftUI output now reports only the sample-runtime
  fallback before compiling on both macOS and the iOS 16 deployment target.

## [Unreleased] - SwiftUI native drag capability

- Native-complete analysis no longer reports SwiftUI `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  touch, keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- TaskApp's SwiftUI degradation report now retains only the separate native
  table-semantics gap plus permissive-shell fallback when applicable.

## [Unreleased] - Qt native table capability

- Native-complete analysis recognizes the canonical UI31/Grid structure when
  the Qt emitter supplies `TableView`, `HorizontalHeaderView`, and a generated
  `QAbstractTableModel` adapter.
- Unsupported or structurally ambiguous Qt HostTable trees retain the stable
  `accessibility.table-semantics-missing` degradation.
- Permissive TaskApp Qt acceptance now reports only the sample-runtime fallback
  before compiling and launching the generated native application.

## [Unreleased] - Qt native drag capability

- Native-complete analysis no longer reports Qt `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  touch, keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- Complete TaskApp Qt acceptance now requires exactly the remaining table
  semantics degradation plus the permissive sample-runtime fallback, then
  compiles and launches the generated native application headlessly.

## [Unreleased] - analyzer-clean Flutter project bootstrap

- Flutter project shells install Mosaic-owned `analysis_options.yaml` and
  `test/widget_test.dart` files alongside the matching lint dependency.
- The generated smoke test imports the actual pub package name and replaces
  Flutter's stock `MyApp` counter test before `flutter create` adds runners.
- Package artifacts now report both bootstrap files to callers.

## [Unreleased] - Flutter Rust engine bundling

- Flutter project shells accept a selected target Rust cdylib, copy it under
  Mosaic's conventional name, and register it through a generated stable Dart
  build hook as a bundled code asset.
- Strict Flutter builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- Native acceptance builds the generated Flutter app, verifies the installed
  engine, and runs the standard binding conformance without
  `MOSAIC_APP_LIBRARY`.

## [Unreleased] - SwiftUI Rust engine bundling

- SwiftUI project shells accept a selected target Rust dylib and copy it into
  the SwiftPM `Runtime` resource bundle under Mosaic's conventional name.
- Strict SwiftUI builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- macOS acceptance verifies the bundled bytes and runs the standard binding
  conformance through the app-local path without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - XAML Rust engine bundling

- XAML project shells accept a selected target Rust DLL, install it as
  `mosaic_app.dll`, and use the existing MSBuild native-library copy target to
  place it beside the WinUI executable.
- Strict XAML builds report `runtime.library-not-bundled` and stop before
  application emission when no engine was selected.
- Windows acceptance verifies the copied engine hash and runs the exact .NET
  binding conformance from its output directory without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Qt Rust engine bundling

- Qt project shells accept the selected target Rust engine, copy it beside the
  built executable, and include it in the CMake install tree under Mosaic's
  conventional runtime filename.
- Strict Qt installable builds report `runtime.library-not-bundled` and stop
  before application emission when no engine was selected.
- Linux acceptance verifies the installed library bytes, launches the generated
  native QML app, and runs the exact Qt binding conformance from the install
  directory without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Compose Rust engine bundling

- Add a target-library-aware profiled build API. Compose project shells copy a
  selected `.dylib`, `.so`, or `.dll` into the platform-specific application
  resources under Mosaic's conventional runtime filename.
- Strict Compose distributable builds now report
  `runtime.library-not-bundled` and stop before application emission when no
  engine was selected.
- Native packaging acceptance composes the shared Rust engine with a real
  Mosaic package, verifies the installed library bytes, and exercises the
  app-relative loader without `MOSAIC_APP_LIBRARY`.

## [Unreleased] - Compose native drag capability

- Native-complete analysis no longer reports Compose `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native pointer,
  keyboard, acceptance, lifecycle, RTL, and accessibility behavior.
- SwiftUI and XAML retain the stable `interaction.drag-drop-inert`
  degradation until their native implementations land.

## [Unreleased] - Compose native table capability

- Native-complete analysis recognizes the canonical UI31/Grid shape as a
  semantic Compose collection now that the emitter publishes table dimensions,
  heading metadata, and per-cell row/column coordinates.
- Unsupported or structurally ambiguous Compose HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- The complete package-expanded TaskApp is now a zero-degradation strict
  Compose build and is packaged as a native desktop application in CI.

## [Unreleased] - Flutter native table capability

- Native-complete analysis recognizes the canonical UI31/Grid shape as a
  semantic Flutter table now that the emitter produces `DataTable`/
  `DataColumn`/`DataRow`/`DataCell` widgets.
- Unsupported or structurally ambiguous Flutter HostTable trees retain the
  stable `accessibility.table-semantics-missing` degradation.
- The complete package-expanded TaskApp is now a zero-degradation strict
  Flutter build and is compiled as a native desktop application in CI.

## [Unreleased] - Flutter native drag capability

- Native-complete analysis no longer reports Flutter `HostDraggable` and
  `HostDropTarget` nodes as inert now that the emitter supplies native
  pointer/touch, keyboard, acceptance, lifecycle, and accessibility behavior.
- SwiftUI and XAML retain the stable
  `interaction.drag-drop-inert` degradation until their native implementations
  land.

## [Unreleased] - ignored native dialog and link contract inventory

- Native-complete analysis now rejects XAML dialogs whose open state still
  requires application code-behind, XAML's unsupported no-dismiss policy, and
  ignored dialog lifecycle events in XAML and SwiftUI.
- External `HostLink.onActivate` event loss in XAML and SwiftUI is now reported
  at the exact package-expanded property path. Supported internal-link dispatch
  and SwiftUI modal-close shapes remain clean.

## [Unreleased] - complete Flutter package source set

- Generated Flutter project shells now copy every exported widget into `lib/`
  while continuing to mount the first export as the application entry widget.
- Whole-package Dart analysis can no longer miss a broken sibling component
  that was emitted only as a top-level distribution artifact.

## [Unreleased] - complete Qt package QML module

- Generated Qt project shells now list every exported QML component in
  `qt_add_qml_module` while continuing to mount the first export.
- Whole-package Qt compilation can no longer miss a broken sibling QML file.

## [Unreleased] - complete SwiftUI package source set

- Generated SwiftUI project shells now copy every exported view into the
  SwiftPM application target while continuing to mount the first export.
- Whole-package Swift compilation can no longer miss a broken sibling view
  that was emitted only as a top-level distribution artifact.

## [Unreleased] - complete Compose package source set

- Generated Compose project shells now copy every exported component into the
  Gradle Kotlin source set while continuing to mount the first export as the
  application entry component.
- Whole-package Kotlin compilation can no longer miss a broken sibling
  component that was emitted only as a top-level distribution artifact.

## [Unreleased] - ignored native control property inventory

- Native-complete analysis now reports stable, property-level degradations for
  tri-state checkbox state ignored by Compose, Flutter, and SwiftUI, and radio
  grouping ignored by Compose, Flutter, Qt, and SwiftUI.
- Strict builds reject those authored behavior losses before emitting app
  artifacts. Explicit `indeterminate: false` remains a clean semantic no-op.

## [Unreleased] - native-complete Qt runtime shell

- Qt project shells emitted under `BuildProfile::NativeComplete` now require
  Mosaic's standard QObject runtime binding and validate required MIL props
  before QML construction.
- Strict Qt shells remove conditional binding compilation and nullable event
  dispatch, while mapping Rust MIL prop names to generated QML member names.
- Linux CI compiles a zero-degradation strict Qt package and exercises normal,
  missing-prop, and missing-runtime conformance paths.

## [Unreleased] - native-complete XAML runtime shell

- XAML project shells emitted under `BuildProfile::NativeComplete` now require
  Mosaic's standard .NET runtime binding before WinUI activation and validate
  required MIL props before showing the component.
- Strict XAML shells omit the reflection host, generated sample props, and
  app-owned dispatch stubs while permissive output remains backward-compatible.
- Windows CI compiles a zero-degradation strict WinUI package and exercises the
  required-runtime success and missing-runtime paths.

## [Unreleased] - native-complete SwiftUI runtime shell

- SwiftUI project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Foundation/C runtime binding and its initial props
  before mounting the generated view.
- Strict SwiftUI shells omit reflection-host, event-print, and generated sample
  paths while permissive output remains backward-compatible.
- macOS runtime CI builds a zero-degradation strict SwiftPM project in addition
  to round-tripping the standard binding.

## [Unreleased] - native-complete Flutter runtime shell

- Flutter project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Dart FFI runtime and the first props envelope before
  mounting the generated widget.
- Strict Flutter shells omit nullable-host, event-print, and generated sample
  paths while permissive output remains backward-compatible.
- Flutter runtime CI now analyzes a zero-degradation strict project in addition
  to round-tripping the standard binding.

## [Unreleased] - native-complete Compose runtime shell

- Compose project shells emitted under `BuildProfile::NativeComplete` now
  require Mosaic's standard Rust runtime and a complete props envelope before
  mounting the generated component.
- Strict Compose shells omit the package-owned reflection bridge, event-print
  fallback, and generated sample values for required props. Permissive output
  retains those preview and compatibility paths.
- Compose runtime CI now compiles a zero-degradation strict project in addition
  to round-tripping the standard JNA binding.

## [Unreleased] - native-complete package profile

- Added deterministic package-expanded degradation analysis and the
  `mosaic-degradations.json` build artifact.
- Added `BuildProfile::Permissive` and `BuildProfile::NativeComplete`; strict
  builds reject known degradations before emitting application artifacts.
- Seeded the capability inventory with documented native drag/drop, table
  semantics, Flutter dialog/link, and generated sample-runtime gaps.

## [Unreleased] - shared package composition

- Added `compose_component` and `compose_component_with_model` as the canonical
  MIL/MLL/MSL composition API: qualified layouts are resolved and dependency
  styles are merged before backend emission.
- Package component artifacts and generated project shells now consume that
  shared result instead of maintaining duplicate compilation pipelines.

## [Unreleased] - reproducible XAML SDK selection

- XAML package shells now preserve the emitter's `global.json`, keeping WinUI
  project builds on the .NET 9 SDK family they target when newer SDKs are also
  installed.

## [Unreleased] - standard Qt Rust runtime binding

- Qt project shells now install Mosaic's package-independent QObject binding
  and connect it through the existing QML host seam.
- Explicit package host assets retain precedence for specialized integrations.

## [Unreleased] - standard Flutter Rust runtime binding

- Flutter project shells now install Mosaic's package-independent Dart FFI
  binding instead of the no-op default host.
- The standard host owns startup, successful event sequencing, snapshots,
  buffers, prop updates, and teardown while preserving injectable custom hosts.

## [Unreleased] - standard XAML Rust runtime binding

- XAML project shells now install Mosaic's package-independent .NET binding and
  prefer it over legacy package-owned `MosaicHost` adapters when the Rust DLL is
  available.
- The standard host uses `NativeLibrary` and `System.Text.Json` to own startup,
  successful event sequencing, prop projection, snapshots, buffers, and teardown.

## [Unreleased] - standard SwiftUI Rust runtime binding

- SwiftUI package shells now install Mosaic's package-independent Foundation/C
  binding and prefer it over legacy package-owned `MosaicHost` adapters.
- The generated C target dynamically resolves the fixed Rust application ABI,
  while the Swift host owns startup, event sequencing, snapshots, buffers, and
  runtime teardown without app-authored platform glue.

## [Unreleased] - standard Compose Rust runtime binding

- Compose Desktop project shells now install Mosaic's package-independent JNA
  binding and prefer it over legacy package-owned `MosaicHost` adapters.
- The generated shell closes its host on disposal, releasing the opaque Rust
  runtime handle and every returned Rust buffer through the fixed C ABI.

## [Unreleased] - reactive Compose native host bridge

- Compose Desktop project shells now subscribe to optional host prop-change
  callbacks, allowing native content-surface interactions to reproject chrome
  without duplicating state in generated UI.
- Compose shells include pinned JNA and JSON runtime dependencies for
  package-owned native host adapters.

## [Unreleased] - keep web test assets out of production

- HTML and Web Component `.test.*` / `.spec.*` host assets are copied without
  being injected as production page modules, matching the existing React host
  asset rule.

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
