# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-05-19

### Added — UI29 kernel primitives (partial)

Extends the SwiftUI backend skeleton (v0.1.0) with four of UI29's kernel
primitives. The remaining three (`If`, `For`, `HostTable`) wait on the
moslayout grammar additions (U29-G3) and a `HostTable` spec; they still
return `UnknownPrimitive` so authors who reach for them get a clear
"not yet supported" diagnostic.

- `Stack` lowers to `ZStack { ... }` — the z-axis / overlay container.
- `HostScroll` lowers to `ScrollView { ... }` — SwiftUI's built-in
  scrollable region. Implicit scroll-state and viewport handling means
  no offset/extent slots need to be threaded through the lowering.
- `HostInput` lowers to `TextField(placeholder, text: .constant(value))`
  with these prop bindings:
  - `placeholder: "..."` → the first arg of `TextField("...", text: ...)`
  - `value: slot: x` → `text: .constant(x)` (see binding nuance below)
  - `read-only: slot: x` / `read-only: true` / `read-only: false` →
    `.disabled(...)` modifier
  - `onChange: emit: onE` → `.onChange(of: value) { dispatch(.e(value: value)) }`
  - `onCommit: emit: onE` → `.onSubmit { dispatch(.e(value: value)) }`
  - `onCancel: emit: onE` → `.onExitCommand { dispatch(.e) }` (macOS-only;
    documented as a known limitation for iOS / iPadOS).
- `HostButton` lowers to `Button(action: { dispatch(.tap) }) { Text(label) }`
  with these prop bindings:
  - `label: "..."` → `Text("...")` inside the label closure
  - `label: slot: x` → `Text(x)` inside the label closure
  - `disabled: slot: x` / `disabled: true` / `disabled: false` →
    `.disabled(...)` modifier
  - `onTap: emit: onE` → `action: { dispatch(.e) }`

### Binding nuance — `.constant(value)`

SwiftUI `TextField` requires a `Binding<String>`, not a plain `String`.
Mosaic components receive slots as immutable `let`s, so we have two
reasonable options:

1. **`.constant(value)` wrapper** — emit `.constant(value)` and rely on
   `dispatch(.commit(value: ...))` for updates. Inline typing does NOT
   echo back into the bound slot; only `onSubmit` (Enter) carries new
   text. Matches UI24's dispatch-driven flux pattern.
2. **Local `@State` proxy** — wrap the body in a `@State` buffer that
   initializes from the slot and dispatches `onChange` per keystroke.
   More complex generated code; deferred to a future PR.

This release ships option (1) and documents the choice in code comments,
the README, and the per-function doc comment on `emit_host_input`.

### Added — tests

- 8 new tests covering: `Stack → ZStack` empty + with-children, HostInput
  `.constant(value)` binding shape, HostInput `read-only` keyword + slot
  forms, HostInput `onCommit` → `.onSubmit` dispatch, HostButton
  `action: + label` structure, HostScroll → `ScrollView`, and a
  recognised-vs-deferred matrix that pins `If` / `For` / `HostTable` as
  still returning `UnknownPrimitive`.
- The crate-version pin test now expects `0.2.0`.

### Crate version

- Bumped from `0.1.0` → `0.2.0`.

## [0.1.0] - 2026-05-19

### Added — initial skeleton (UI28 WB4)

- New crate `mosaic-emit-swiftui` providing a `from_pipeline(interface,
  layout, style) -> Result<PipelineEmitResult, PipelineEmitError>` entry
  point that consumes the three-file Mosaic pipeline (`.mil` / `.mll` /
  `.msl`) and emits a `.swift` source file containing a SwiftUI `View`
  struct.
- Primitive lowering for `Box` → `Group`, `Row` → `HStack`, `Column` →
  `VStack`, `Text` → `Text("...")` / `Text(slotName)`, `Spacer` →
  `Spacer()`, `Image` → `Image(systemName: "...")` placeholder, and
  `Divider` → `Divider()`. Other primitives (`Scroll`, `Stack`, `Icon`,
  `Grid`, `Input`) return `UnknownPrimitive` errors and land in
  follow-up PRs.
- Slot lowering: `text` → `String`, `number` → `Double`, `bool` →
  `Bool`, `image` → `String`, `color` → `String`, `node` → `AnyView`,
  `list<T>` → `[<inner Swift>]`. Slots become `let` stored properties on
  the View struct; `dispatch: (NameEvent) -> Void` is appended last.
- Event lowering: one Swift `enum case` per declared emit, with the `on`
  prefix stripped and the rest lower-camelCased (mirrors the React
  backend's union-variant `type` literal). Payload parameters become
  named associated values (`case navigate(row: Double, col: Double)`).
  Empty emit lists produce an uninhabitable `enum NameEvent {}` — the
  SwiftUI analog of TypeScript's `type NameEvent = never`.
- 13 unit tests covering: empty layout, slot lowering for every
  primitive type, empty / non-empty event enum, primitive surface views,
  Text-literal escaping, Text-slot-ref camelCase conversion, Image
  source / fallback, kebab→camelCase across slot+emit+param names,
  component name mismatch error, UnknownPrimitive error, full smoke
  test for a multi-slot Row+Text+Spacer+Text component, and a version
  pin.
- `PipelineEmitError` variants: `ComponentNameMismatch`,
  `UnsafeSlotName`, `UnsafeEmitName`, `UnknownPrimitive` (mirrors React
  backend; no React-only reserved-name variant because Swift has no
  equivalent prop-name conflict surface).

### Known limitations (deferred to follow-up PRs)

- **UI28 §2 Cell / Column-as-metadata / Grid v3** — this PR keeps the
  legacy UI14 `Column → VStack` lowering so existing demos still
  compile. The UI28 SwiftUI lowering (`Grid → SwiftUI.Table {
  TableColumn(...) }` per spec §4.4) is a separate follow-up.
- **`connects` wiring** — emit refs on layout nodes are not yet
  attached to SwiftUI gesture modifiers (`.onTapGesture { dispatch(.tap) }`).
- **`mosstyle::StyleDef` inlining** — the argument is accepted to lock
  the signature, but `View` modifier chains (`.background(...)`,
  `.padding(...)`) are not yet emitted.
- **`Scroll`, `Stack`, `Icon`, `Grid` (v2), `Input` primitives** — each
  returns `UnknownPrimitive` and lands in its own follow-up.
