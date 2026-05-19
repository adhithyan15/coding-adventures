# Changelog

All notable changes to this package will be documented in this file.

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
