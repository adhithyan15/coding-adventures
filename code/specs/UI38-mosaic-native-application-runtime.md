# UI38 — one Rust application engine for every Mosaic backend

**Status:** canonical design and native-application completion backlog.
**Supersedes:** UI33's target-language application layer. UI33's read-only view,
unidirectional event flow, and generated-host principles remain; application state
and operations now live once in Rust instead of being reimplemented per host.

---

## 1. The product bar

Mosaic is complete when an application author can ship a decent-looking,
accessible native application from:

1. one Rust application engine;
2. one package of `.mil`, `.mll`, and `.msl` files; and
3. reusable Mosaic packages selected in `mosaic-package.toml`.

The author must not write Swift, Kotlin, Dart, C#, C++, JavaScript, or platform
adapter code for ordinary application behavior. Pixel identity is not required.
Platform-appropriate rendering and later platform-specific style overrides are
allowed and desirable.

The native acceptance set is:

| Backend | Native surface | Counts toward the bar |
|---|---|---|
| SwiftUI | Apple platforms | yes |
| XAML | Windows | yes |
| Qt/QML | Linux and portable desktop | yes |
| Compose | Android and desktop | yes |
| Flutter | mobile and desktop | yes |
| React | web | parity/supporting target |
| Web Component | web | parity/supporting target |
| HTML | web | parity/supporting target |
| Electron | wrapped web content | compatibility only; never native proof |

Source emission, compile-only tests, and generated shells do not prove this bar.
A backend counts only when its artifact installs or launches, renders native or
framework-native controls, completes the reference workflow, persists state, and
passes its accessibility contract.

## 2. Why the current architecture stops short

Mosaic already compiles packages into all target languages, but the generated
shells expose host seams that each application fills by hand. That duplicates the
application state machine and platform integration across hosts. It also lets an
emitter silently replace unsupported interactive primitives with inert containers.

The result can compile without being an application. UI38 closes that gap with:

- one Rust-owned state and operation boundary;
- generated, package-independent host bindings;
- explicit host effects instead of app-specific adapters;
- a strict backend capability profile; and
- launch, interaction, persistence, and accessibility conformance.

## 3. Architecture

```text
Mosaic package (.mil + .mll + .msl)
        │ typed props/events
        ▼
Rust application engine implementing MosaicApp
        │ stable request/update protocol
        ▼
mosaic-app-runtime
        │ C ABI on native targets / Wasm exports on web targets
        ▼
generated backend binding + reusable effect host
        │
        ├─ SwiftUI
        ├─ XAML
        ├─ Qt/QML
        ├─ Compose
        ├─ Flutter
        └─ web emitters
```

Generated views remain read-only. A native control produces a semantic event; the
Rust engine handles it and returns a new render model plus requested host effects.
Effect results return as events. No generated view mutates application state.

## 4. Rust contract

`mosaic-app-runtime` owns the public contract. The initial source-level API is:

```rust
pub trait MosaicApp {
    type Error: std::error::Error + Send + Sync + 'static;

    fn start(&mut self, context: StartContext)
        -> Result<AppUpdate, Self::Error>;

    fn dispatch(&mut self, event: Event)
        -> Result<AppUpdate, Self::Error>;

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error>;

    fn restore(&mut self, snapshot: Snapshot)
        -> Result<AppUpdate, Self::Error>;
}
```

The contract types have an implementation-independent wire representation:

```rust
pub struct StartContext {
    pub protocol_version: u32,
    pub locale: String,
    pub color_scheme: ColorScheme,
    pub text_scale: f32,
    pub platform: Platform,
    pub restored_snapshot: Option<Snapshot>,
}

pub struct Event {
    pub protocol_version: u32,
    pub sequence: u64,
    pub name: String,
    pub payload: serde_json::Value,
}

pub struct AppUpdate {
    pub props: serde_json::Value,
    pub effects: Vec<Effect>,
    pub announcements: Vec<Announcement>,
}

pub struct Update {
    pub protocol_version: u32,
    pub revision: u64,
    pub props: serde_json::Value,
    pub effects: Vec<Effect>,
    pub announcements: Vec<Announcement>,
}
```

`props` must validate against the package's exported root component contract.
Event names and payloads must validate against its exported emits. Named
records/enums in the model language will make that contract author-friendly, but
the runtime boundary must not depend on a particular generated host language.
The application never chooses a transport revision: `mosaic-app-runtime` wraps each
successful `AppUpdate` in an `Update` and assigns its revision.
Every start, event, and update envelope carries `protocol_version`; the runtime
rejects a mismatch before invoking application code. Snapshot schema versions
remain independent. Startup also rejects a non-finite or non-positive `text_scale`.
An application method returning an error must leave its observable state unchanged;
the runtime does not consume sequence or revision state, so the host can retry.

### 4.1 Determinism and ordering

- Events are processed serially by sequence number.
- The runtime assigns exactly one monotonically increasing revision to each
  successful start, restore, or accepted event.
- A host must finish applying an update before dispatching the next event.
- Unknown, duplicated, or out-of-order events are protocol errors, not no-ops.
- The engine does not call into the host while handling an event. Effects are data
  returned in `Update`, preventing re-entrant host/application calls.

### 4.2 Snapshot contract

A snapshot is versioned opaque bytes with an application schema identifier. Hosts
may persist it but never interpret it. Restore must either return a complete first
update or a typed incompatible-snapshot error; silently starting empty is invalid.

## 5. Stable bridge

Native artifacts link a `cdylib` or static library exposing a small C ABI. The ABI
passes owned byte buffers rather than Rust layouts:

```c
mosaic_status mosaic_app_create(mosaic_bytes start, mosaic_handle *app,
                                mosaic_buffer *initial_update);
mosaic_status mosaic_app_dispatch(mosaic_handle app, mosaic_bytes event,
                                  mosaic_buffer *update);
mosaic_status mosaic_app_snapshot(mosaic_handle app, mosaic_buffer *snapshot);
mosaic_status mosaic_app_restore(mosaic_handle app, mosaic_bytes snapshot,
                                 mosaic_buffer *update);
void mosaic_buffer_free(mosaic_buffer buffer);
void mosaic_app_destroy(mosaic_handle app);
```

The same request/update envelopes are exported from WebAssembly. FFI functions
must not unwind, leak Rust-owned memory, or expose architecture-dependent structs.
Protocol and application errors use distinct status codes and include a bounded
UTF-8 diagnostic in the returned output buffer. A panic is contained and poisons
the affected handle so unknown application state cannot continue.

Bindings for Swift, Kotlin/JNI, Dart FFI, C#, C++/Qt, JavaScript, and TypeScript are
generated from this fixed runtime ABI. They belong to Mosaic, not to applications.

## 6. Host effects

Effects cover capabilities that cannot be performed portably inside the Rust
engine. The v1 standard effect set is:

| Family | Operations |
|---|---|
| key-value storage | get, set, remove, list |
| files | open, save, read, write through a user-approved handle |
| clipboard | read and write text |
| URLs | open an external URL |
| notifications | request permission, schedule, cancel |
| time | current time and one-shot/repeating timers |
| lifecycle | opened, foregrounded, backgrounded, close requested |
| preferences | locale, color scheme, text scale, reduced motion |
| accessibility | polite and assertive announcement |

Every request carries an effect id. Completion, cancellation, denial, and failure
return as normal events carrying that id. Permission denial is a first-class result.
Backends must report unsupported effects during packaging under the strict profile;
runtime `unimplemented` placeholders are forbidden.

Additional capabilities live in versioned packages. They must not expand the
primitive kernel merely because they need a host API.

## 7. Generated host responsibility

For every backend, the artifact builder must generate code that:

1. creates the Rust engine and supplies `StartContext`;
2. validates and applies the initial props;
3. maps semantic control events to protocol events;
4. applies each returned revision atomically;
5. executes standard effects and returns results;
6. persists/restores snapshots using the standard storage host;
7. forwards lifecycle and preference changes; and
8. destroys the engine and releases all bridge buffers.

Applications may inject test effect hosts. Production defaults are supplied by
Mosaic. An app-owned `MosaicHost`, reducer, state mirror, or adapter is a conformance
failure even when the artifact launches.

## 8. Strict `native-complete` profile

`mosaic compile --profile native-complete` and the equivalent package setting must
fail before emission when any selected backend:

- lowers an interactive primitive to a non-interactive container;
- ignores a declared event, payload field, state, style, or accessibility property;
- emits `TODO`, `unimplemented`, placeholder, or sample application behavior;
- lacks a requested standard effect;
- requires an application-owned host adapter; or
- cannot express the component's semantic accessibility role and name.

The existing permissive profile remains useful for previews and incremental backend
development. It must emit a machine-readable degradation report. A package cannot
claim native completeness from permissive output.

## 9. Accessibility and decent defaults

Accessibility is a compiler/runtime invariant, not an app-by-app checklist:

- interactive primitives require a semantic role, accessible name, focus behavior,
  keyboard/assistive-action equivalent, and disabled state;
- focus order follows semantic document order unless explicitly and validly changed;
- text respects platform text scaling without clipping essential content;
- colors default to platform-aware tokens with contrast-preserving states;
- reduced motion and high-contrast preferences reach the style/runtime layers;
- dynamic updates can request polite or assertive announcements; and
- pointer-only interaction cannot pass `native-complete`.

The standard theme should look coherent by default while using each backend's native
metrics and controls. Pixel differences are expected.

## 10. Reference application acceptance

The first proof is **Trestle Native v1**, deliberately smaller than the existing
super-app design. It includes project/task list, create/edit/delete, completion,
due date, priority, labels, list/detail navigation, persistence, light/dark modes,
compact/regular layouts, keyboard operation, and screen-reader names.

It defers board drag-and-drop, calendar, spreadsheet, and critical-path views until
their primitives pass the same backend profile.

Trestle Native v1 is complete only when CI demonstrates:

- one Rust engine crate and one Mosaic package;
- zero app-owned platform adapters;
- build/install/launch on all five native backend families;
- the same scripted create → edit → complete → restart → verify workflow;
- no degradation report entries or generated placeholders; and
- automated semantic accessibility assertions plus a documented manual smoke pass.

## 11. Prioritized completion backlog

Priority is recalculated after every merged slice. Prefer the smallest item that
unblocks multiple downstream targets; never count source generation as completion.

### P0 — cross-backend execution spine

- [x] Implement `mosaic-app-runtime` contract types and deterministic engine tests.
- [x] Implement the panic-safe native C ABI with buffer-ownership tests.
- [ ] Implement WebAssembly exports over the same JSON envelopes.
- [x] Generate package-independent bindings for the five native backend families.
  - [x] Compose/JVM through the standard JNA binding.
  - [x] SwiftUI through the standard Foundation/C dynamic binding.
  - [x] XAML through the standard .NET native binding.
  - [x] Flutter through the standard Dart FFI binding.
  - [x] Qt/QML through the standard Qt Core binding.
- [ ] Generate standard effect hosts, beginning with storage and lifecycle.
- [ ] Bundle the selected Rust application engine library into every generated
  installable native artifact and resolve it from an app-relative location.
  - [x] Compose accepts an explicit target `cdylib`, copies it into the native
    distribution resources, resolves it through Compose's app-resources JVM
    property, and rejects strict distributable builds that omit it. The shared
    Rust engine plus Mosaic conformance package compile, package, and launch as
    a macOS `.app` without an injected library path; Linux CI verifies the same
    installed-resource bytes and runtime round trip.
  - [x] Qt accepts an explicit target `cdylib`, copies it beside the CMake-built
    executable and into the install tree, resolves it from the application
    directory, and rejects strict installable builds that omit it. Linux CI
    verifies the installed bytes, launches the generated QML application, and
    runs the complete standard-binding conformance without an injected path.
  - [x] XAML accepts an explicit target Rust DLL, installs it as
    `mosaic_app.dll`, copies it beside the WinUI executable through the standard
    MSBuild project, resolves it from `AppContext.BaseDirectory`, and rejects
    strict builds that omit it. Windows CI verifies the output bytes and runs
    the complete standard-binding conformance without an injected path. Visible
    WinUI launch remains tracked separately on an interactive Windows worker.
  - [x] SwiftUI accepts an explicit target Rust dylib, copies it into SwiftPM's
    `Runtime` resource bundle, resolves it through `Bundle.module`, and rejects
    strict installable builds that omit it. macOS CI verifies the bundled bytes
    and complete standard-binding conformance without an injected library path.
  - [x] Flutter accepts an explicit target `cdylib`, registers the conventional
    runtime through a generated stable Dart build hook, and lets Flutter package
    it as a platform-native code asset. Strict installable builds reject a
    missing selection; CI builds the native app, verifies the installed engine,
    and runs the complete standard-binding conformance without an injected path.
- [x] Make SwiftUI project shells compile components with an empty event enum by
  emitting unreachable but type-correct wire helpers with exhaustive empty
  switches; macOS CI builds the canonical no-event conformance app through
  SwiftPM.
- [ ] Add `native-complete` capability/degradation analysis to the compiler.
  - [x] Add a package-builder API and CLI profile with deterministic,
    package-expanded degradation reports and pre-emission strict rejection.
  - [x] Make Compose and Flutter `native-complete` project shells require the
    standard Rust runtime and runtime-provided props, with no optional-host or
    sample-data path.
  - [x] Repeat the runtime-required shell policy for SwiftUI.
  - [x] Repeat the runtime-required shell policy for XAML.
  - [x] Repeat the runtime-required shell policy for Qt: require the standard
    QObject binding, validate Rust props before QML construction, map MIL names
    to QML names, and cover normal, missing-prop, and missing-runtime paths.
  - [ ] Inventory ignored properties, events, styles, effects, and accessibility
    metadata across every native emitter, and add the equivalent package setting.
    - [x] Report property-level degradations for ignored tri-state checkbox
      state and radio-group behavior, including package-expanded paths.
    - [x] Report known XAML/SwiftUI dialog lifecycle and external-link event
      losses, including XAML dialog state that still requires app code-behind.
    - [x] Lower portable `Text` accessible names, heading roles, and hidden
      semantics across SwiftUI, Compose, Flutter, Qt/QML, and XAML; report
      unsupported text roles and dynamic hidden/name forms as stable
      property-level degradations.
    - [ ] Inventory the remaining ignored layout properties, MSL styles,
      effects, accessibility metadata, and dialog/link target semantics.
    - [ ] Define a serializable native-view reference contract for `node` and
      component slots; Flutter's JSON runtime cannot currently materialize a
      Dart `Widget` value for those otherwise typed composition seams.
    - [ ] Define display coercion for non-text values used as `Text.content`.
      Compose currently emits a numeric MIL slot as `Double` directly into
      `Text`, which fails the generated project's Kotlin compile boundary.
    - [ ] Apply UI35 `accepts` kind filtering to the existing React and HTML
      interaction lowerings; both predate that enforcement and currently accept
      every drag kind even when a target authors a filter.
  - [ ] Remove every reported TaskApp degradation on all five native backends.
    - [x] Remove Flutter's four inert drag/drop reports by lowering UI35 to
      native `Draggable`/`DragTarget` widgets with equivalent keyboard and
      screen-reader operation.
    - [x] Remove Flutter's final table-semantics degradation by lowering the
      canonical dynamic UI31/Grid shape to native `DataTable` primitives.
      The full package-expanded TaskApp now passes Flutter's strict profile,
      whole-project Dart analysis, and a native desktop build in CI.
    - [x] Remove Compose's four inert drag/drop reports by lowering UI35 to
      Compose Desktop's native `dragAndDropSource`/`dragAndDropTarget`
      modifiers with an instance-scoped keyboard target registry, RTL-aware
      navigation, live-region state, accepted-only outcomes, and one shared
      drop payload path.
    - [x] Remove Compose's final table-semantics degradation by annotating the
      canonical dynamic UI31/Grid shape with native collection dimensions,
      heading metadata, and row/column coordinates. The complete
      package-expanded TaskApp now passes Compose's strict profile, Kotlin
      compilation, and native desktop distribution packaging in CI.
    - [x] Remove Qt's four inert drag/drop reports by lowering UI35 to native
      Qt Quick `DragHandler`/`Drag` and `DropArea` primitives with
      component-scoped keyboard traversal, RTL behavior, kind filtering,
      accepted-only lifecycle outcomes, one drop-payload path, and Qt 6.8
      accessibility announcements. Complete TaskApp Qt CI now compiles and
      launches the generated app with only table semantics left to close.
    - [x] Remove Qt's final table-semantics degradation by adapting the
      canonical dynamic UI31/Grid shape to `QAbstractTableModel`, `TableView`,
      and `HorizontalHeaderView`, with accessible cell activation carrying the
      existing row/column navigation payload. Permissive TaskApp Qt acceptance
      now retains only the sample-runtime fallback.
    - [x] Remove SwiftUI's final table-semantics degradation by adapting the
      canonical dynamic UI31/Grid shape to native `Table` and
      `TableColumnForEach`, retaining the interactive Cell subtree and using a
      native `List` compatibility path before macOS 14.4 / iOS 17.4. The full
      TaskApp compiles for macOS and its iOS 16 deployment target; permissive
      output now retains only the sample-runtime fallback.
    - [x] Promote the strict SwiftUI macOS TaskApp with the concrete
      `task-mosaic-app` runtime, byte-for-byte SwiftPM resource verification,
      and direct launch without `MOSAIC_APP_LIBRARY`. Keep iOS 16 compilation
      as a separate source-portability gate rather than packaging a macOS dylib.
    - [x] Promote the concrete `task-mosaic-app` runtime into the complete XAML
      WinUI artifact, verify the DLL beside `TaskApp.exe` byte-for-byte, and drive
      TaskApp startup props plus a semantic event through the generated .NET
      binding without `MOSAIC_APP_LIBRARY`. Visible launch remains an interactive
      Windows-worker gate.
    - [x] Remove XAML's table-semantics degradation by wrapping the canonical
      indexed UI31/Grid shape in component-scoped WinUI controls whose automation
      peers implement UIA Table/Grid and TableItem/GridItem provider patterns,
      while retaining the authored editable cell subtree and conservative visual
      fallback for unsupported shapes.
    - [x] Remove XAML's final four drag/drop degradations by lowering UI35 to
      component-scoped WinUI drag events with shared pointer/touch and keyboard
      acceptance, lifecycle dispatch, RTL traversal, and UI Automation
      announcements. Complete TaskApp Windows CI now uses `native-complete` and
      requires an empty degradation report.
- [ ] Add launch-and-dispatch conformance fixtures for every native backend.
  - [x] XAML/.NET loads the shared Rust conformance DLL and round-trips startup,
    typed props, semantic dispatch, revised props, buffers, and teardown.
  - [x] SwiftUI/Foundation loads the shared Rust conformance dylib and round-trips
    startup, dispatch, snapshot/restore, notification, and teardown in macOS CI.
  - [x] Qt/QML loads the shared Rust conformance library and round-trips startup,
    dispatch, snapshot/restore, buffers, and teardown in headless Linux CI.
  - [x] Flutter/Dart FFI loads the shared Rust conformance library and round-trips
    startup, dispatch, snapshot/restore, notification, buffers, and teardown in
    headless Linux CI.
  - [x] Compose/JNA loads the shared Rust conformance library and round-trips
    startup, dispatch, snapshot/restore, notification, buffers, and teardown on
    the Linux JVM.
- [x] Gate the complete package-expanded XAML TaskApp on GitHub-hosted Windows
  with real WinUI compilation and executable production.
  - [x] Allocate `For` row-view-model and projection names per loop rather than
    per `as:` alias; package expansion currently makes the sheet and task-list
    `row` loops collide on `TaskApp_RowVm` / `TaskAppRowVmRows`.
  - [x] Keep nested `For` bindings and expression helpers inside the generated
    DataTemplate binding scope instead of referring to page members or an
    enclosing template from a typed child template.
- [ ] Add a self-hosted Windows worker with an interactive desktop as the required
  WinUI launch-and-interaction gate. GitHub-hosted workers compile the complete
  TaskApp and round-trip the real Rust engine through the generated .NET binding,
  but can terminate WinUI before `OnLaunched` with stowed-exception status
  `0xc000027b`; they therefore cannot honestly prove a visible native surface.
- [x] Make the generated Flutter project analyzer-clean after its documented
  `flutter create` bootstrap: replace the stock `MyApp` widget test, provision
  its lint configuration, and omit unused authored `For` bindings. Generated
  shells now own a package-name-correct Mosaic smoke test and lint dependency;
  Linux CI runs whole-project `flutter analyze`, preserves the generated test
  across `flutter create`, and executes the permissive toolkit test suite.
- [x] Complete Compose TaskApp Kotlin typing: normalize Mosaic value truthiness
  in boolean positions and supply required native input commit payloads.
- [x] Restore complete Compose TaskApp generation after package expansion added
  per-row `HostTooltip` nodes: lower them to Compose Foundation's native overlay
  with plain-text semantics instead of rejecting the current package graph.
- [x] Route every compile entry point through one package-composition pipeline;
  standalone and package builds now consume the same resolved layout and merged
  dependency-style IR before selecting a backend.
- [ ] Type-check the complete generated Trestle application on every target, not
  only focused fixtures; the complete package-expanded TaskApp now passes the
  Swift compiler and SwiftPM linker with type-correct truthiness, collision-safe
  generated members, and bounded native-view type inference. Flutter now lowers
  Mosaic value truthiness into Dart boolean positions and supplies declared native
  input payloads; the complete TaskApp passes Dart analysis, builds as a native
  macOS Flutter bundle, and remains running when launched with the generated
  no-runtime fallback. Qt's plain multiline `Input`
  lowering and fixed-dimension wrapper deduplication are complete, and the complete
  TaskApp now passes Qt's QML compiler, AUTOMOC, C++ compiler, and linker. Fractional
  font sizing is type-correct and the app remains running in a headless launch.
  Generated shells now select Qt's customization-capable Basic Controls style by
  default, so styled `HostButton` backgrounds render without the native macOS
  style's repeated warnings or ignored paint; an explicit host
  `QT_QUICK_CONTROLS_STYLE` still wins. The headless macOS launch retains one
  lower-priority Qt font-alias performance diagnostic for Basic's `Sans Serif`
  fallback, which is not present in the generated TaskApp QML.
  Standalone Qt signal allocation is now collision-safe: Notes' `onDelete`
  lowers to `mosaicEmitDelete()` while its engine envelope remains `onDelete`.
  The real standalone Notes project passes Qt's QML compiler, resource/AOT
  generation, AUTOMOC, C++ compiler, linker, and a headless launch.
  Compose now emits the Notes package's legacy multiline `Input` as a native
  editor and normalizes Mosaic truthiness at every Kotlin Boolean boundary.
  Required native input commit payloads receive the controlled field value.
  The complete package-expanded TaskApp passes Kotlin compilation, creates a
  native macOS application and DMG, and remains running when launched with the
  generated no-runtime fallback.

### P1 — reusable application vocabulary

- [ ] Add named records, enums, optional fields, and keyed collections to mosmodel.
- [x] Add schema-versioned theme-token inputs to mosstyle and package builds,
  with global values, per-backend overrides, recursive package inheritance,
  aliases, and fail-closed validation.
  - [x] Let packages declare a safe, package-relative token palette so reusable
    libraries carry scoped defaults automatically; dependency defaults are
    overridden by the consuming package and then explicit application input.
- [ ] Ship `mosaic-std-foundation`: tokens, type scale, spacing, surfaces, icons.
- [ ] Ship `mosaic-std-controls`: buttons, inputs, select/picker, switch, slider.
- [ ] Ship `mosaic-std-navigation`: app shell, toolbar, sidebar/rail, tabs, routes.
- [ ] Ship `mosaic-std-feedback`: alert, dialog, toast, progress, empty/error states.
- [ ] Ship `mosaic-std-data`: list, virtualized list, table, form and field patterns.
- [ ] Ship `mosaic-std-services` and umbrella `mosaic-std` manifests.
- [ ] Make the standalone legacy `mosaic-pkg-grid` package compile on every
  native backend; SwiftUI currently rejects its exported `Cell` component when
  built directly even though package-expanded TaskApp table composition passes.
- [x] Make the existing `mosaic-pkg-toolkit` compile across all five native
  backends as a migration baseline.
  - [x] Lower `HostLink` to native Compose annotated links, including internal
    routing dispatch and indexed toolkit navigation payloads.
  - [x] Lower `HostDialog` to native modal `Dialog` and non-modal `Popup`
    overlays, including dismissal policy, lifecycle dispatch, semantic title,
    and nested toolkit content.
  - [x] Lower `Icon` through the native font-glyph stack, with semantic
    `spinner` mapped to an accessible `CircularProgressIndicator`; all 23
    toolkit components now emit for Compose.
  - [x] Make the Compose package project type-check every emitted component,
    not only the first mounted entry component. Every export is now copied into
    Gradle's source set, Accordion projects `bodies[i]` through Compose's native
    integer loop-index shadow, and Linux CI compiles the complete 23-component
    toolkit project.
  - [x] Lower `Icon` to SF Symbols on SwiftUI, with semantic `spinner` mapped
    to an accessible native `ProgressView` and runtime glyph/label support.
  - [x] Make the SwiftUI package project type-check every exported view.
    Accordion collection access uses the native `ForEach` integer shadow and
    macOS CI builds the complete 23-component toolkit through SwiftPM.
  - [x] Lower `Icon` to accessible Qt semantic glyphs and map `spinner` to the
    native indeterminate `BusyIndicator`, with live glyph/label bindings.
  - [x] Make the Qt package project compile every exported QML component in
    one `qt_add_qml_module`; Linux CI builds all 23 toolkit exports.
  - [x] Make Flutter control lowerings compile inside toolkit components whose
    names shadow Material controls, derive native callback payload fields from
    MIL emits, preserve decimal numbers, and route indexed links correctly.
  - [x] Make the Flutter package project type-check every exported widget.
    Every export is copied into `lib/`; Linux CI analyzes all 23 components and
    builds a native Flutter desktop application from the documented runner
    bootstrap while continuing to mount the first export.
  - [x] Make the XAML package project compile every exported control. Windows
    CI builds all 23 component XAML/code-behind triples together in the
    generated WinUI project while continuing to mount the first export.
- [ ] Enforce accessible names, focus, keyboard actions, scaling, contrast, and
  reduced-motion behavior in the strict profile.

### P2 — first complete product

- [ ] Refactor Trestle v1 behavior behind `MosaicApp`.
- [ ] Replace positional task arrays with named models.
- [ ] Compose the v1 UI exclusively from Mosaic standard packages and primitives.
- [ ] Add the shared workflow, persistence, and accessibility acceptance suite.
- [ ] Package and launch Trestle Native v1 on every native backend family.

### P3 — distribution and expansion

- [ ] Add real semantic-version resolution, transitive resolution, `mosaic.lock`,
  cache/vendor support, and a `mosaic add` workflow.
- [ ] Close capability gaps exposed by Trestle without adding app-specific widgets.
- [ ] Promote board, calendar, sheet, and browser workflows one at a time through
  the same native-complete acceptance gate.

## 12. First implementation slices

The next PRs should be independently mergeable in this order:

1. runtime contract data types, sequence/revision validation, and unit tests;
2. C ABI lifecycle/dispatch/buffer ownership around a fixture app;
3. backend capability inventory and `native-complete` diagnostic plumbing;
4. storage/lifecycle effect protocol and one generated binding end to end;
5. repeat the binding conformance fixture for the remaining native backends.

This order proves the execution spine early, permits backends to converge in small
slices, and gives the standard library and Trestle a stable boundary to target.
