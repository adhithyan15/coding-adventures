# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added - Native activation for MSL focused states

SwiftUI output now connects UI15's built-in `state focused` blocks on native
focus-capable host controls to a generated local `@FocusState`. The same shared
MSL properties and transitions activate when `TextField`, `Button`, `Toggle`,
or `Link` receives native keyboard or pointer focus, including independent
instances inside `ForEach`. Explicit `state-when-focused` predicates remain
author-controlled. A Task App acceptance gate proves its Mosaic-authored
project-composer focus ring reaches generated SwiftUI without handwritten
AppKit UI.

### Added - Native activation for MSL hover states

SwiftUI output now activates UI15's built-in `state hover` blocks without
requiring authors to repeat the interaction as a `state-when-hover` layout
predicate. The emitter generates a small native SwiftUI hover wrapper only
when the compiled component uses a hover state. Every wrapper owns its own
`@State`, including wrappers inside `ForEach`, so hovering one repeated row
does not restyle the whole list. Existing explicit `state-when-hover`
predicates remain author-controlled and do not install pointer tracking.

### Added - Native SwiftUI lowering for MSL transitions

Part-level and state-local transitions from `mosstyle-compiler` now lower to
property-scoped SwiftUI `.animation(_:value:)` modifiers. Resolved millisecond
and second durations, standard ease curves, and cubic Bézier timing curves map
to native `Animation` values. State-local transitions apply while entering the
matching `state-when-*` condition and fall back to the part transition, or no
exit animation when no part transition exists. The emitter also now lowers the
MSL `opacity` property so common fade transitions work natively.

### Added - Mosaic event envelopes for SwiftUI hosts

Generated non-empty `{Component}Event` enums now include `mosaicName`,
`mosaicPayload`, and `mosaicEnvelope` helpers. `Sources/App/App.swift` uses the
envelope in its sample dispatch closure, giving SwiftUI native hosts a stable
wire shape to JSON-encode into shared Mosaic/Engram business logic.

### Fixed - `--emit-project` SwiftPM shell supplies view inputs

`Sources/App/App.swift` now mounts `{Component}View(...)` with deterministic
sample values for every declared slot plus a dispatch closure. Previously the
project shell emitted `{Component}View()` even though generated SwiftUI views
store each slot and `dispatch` as required initializer inputs, so any component
with slots (including EngramApp) produced a SwiftPM shell that was not
compile-shaped.

### Fixed — editable `HostInput` (the formula-bar issue)

A `HostInput` with an `onChange` handler and a bound `value` slot now lowers to
a **writable** `TextField` binding — `Binding(get: { value }, set: { dispatch(.onChange(value: $0)) })`
— instead of the read-only `text: .constant(value)`. Previously the generated
`TextField` could not be typed into at all (the constant binding discarded
every keystroke, and the separate `.onChange(of:)` modifier only fired when the
*prop* changed). The setter now dispatches the change per keystroke, so hosts
get a genuinely editable field; the redundant `.onChange(of:)` modifier is no
longer emitted for editable inputs (emitting both would feed back: setter →
host updates slot → `.onChange` fires → dispatches again). Inputs without an
`onChange` handler keep the read-only `.constant(...)` form (label-like
display). This is what makes the VisiCalc SwiftUI demo's formula bar actually
editable.

### Added — UI32-K-swiftui — `--emit-project` SwiftPM macOS shell

L7 of UI32 ([spec PR #4286](https://github.com/adhithyan15/coding-adventures/pull/4286); L2-L6: #4297, #4309, #4315, #4319, #4325). `mosaic-compile --backend swiftui --emit-project` now produces a SwiftPM scaffold:

- `Package.swift` — pinned `swift-tools-version: 5.10` + `platforms: [.macOS(.v13)]` per UI32 §3.6.3. Single executable target `App` at `Sources/App/`.
- `Sources/App/App.swift` — SwiftUI `@main App` + `WindowGroup` mounting `{Component}View()` (matches the emitter's `{name}View` struct convention).
- `README.md` — `swift run` recipe + file map. Notes the user must move `{Component}.swift` into `Sources/App/` for SwiftPM to compile it (v1 layout doesn't auto-place the component file).

New public API (matches L2-L6 pattern):

- `pub struct EmitOptions` — `emit_project`, `pinned_swift_tools`, `pinned_macos_min`.
- `pub struct ProjectFiles` — `package_swift`, `app_swift`, `readme`.
- `pub enum ProjectShellError` — `SwiftKeywordCollision(String)` surfaced through `PipelineEmitError::UnsafeSlotName`.
- `pub struct PipelineEmitResultWithProject`.
- `pub fn from_pipeline_with_options(...)`. Existing `from_pipeline(...)` unchanged.

UI32 §3.6.2 SwiftUI row contract: Swift reserved keywords (`Class`, `Protocol`, `Actor`, `Self`, `Any`, `Type`, etc. — PascalCase subset) MUST be rejected to avoid backtick-quoting in identifier positions. `SWIFT_RESERVED_KEYWORDS` reject-list enforces this; collision → fail-loud via `ProjectShellError::SwiftKeywordCollision`.

10 new tests cover the spec §3 gates plus a Swift-keyword truth table (10 accept/reject vectors) and an App.swift structural test (@main + WindowGroup + `<Component>View()` mount). Total tests: 81 (was 71, +10).

### Added — UI31-K-swiftui — `HostTable` RTL contract

The SwiftUI `HostTable` lowering (which produces a structural
`VStack(alignment: .leading, spacing: 0)` of `HStack` rows) now
honours the UI31 §3.2 RTL contract via SwiftUI's `Environment`
key-path knob `\.layoutDirection`:

- `dir: rtl` → `.environment(\.layoutDirection, .rightToLeft)`
  modifier attached to the VStack; flips horizontal layout
  direction for the whole table.
- `dir: ltr` → `.environment(\.layoutDirection, .leftToRight)` —
  explicit-LTR, useful for tables that should stay LTR inside an
  RTL window (e.g. data-heavy spreadsheets).
- `dir: auto` → no modifier; the spec-mandated "let the host
  decide" semantic is the SwiftUI default — the ambient
  `Environment(\.layoutDirection)` flows through from the system
  locale → app → ancestor view cascade.
- `dir: slot: layout-direction` →
  `.environment(\.layoutDirection, layoutDirection)`, where the
  slot must evaluate to a `LayoutDirection`. The slot name passes
  through `is_safe_swift_identifier` so it can't smuggle malicious
  Swift through the modifier's expression position.
- Unknown keywords drop silently — the allow-list is the security
  gate. Test #6 feeds the literal payload
  `".rightToLeft).onAppear { pwn() }"` (specifically shaped to
  break out of the modifier-call argument list) and asserts `pwn()`
  never reaches the output.

7 new tests cover the a11y gate (VStack + HStack structure
preserved — not a flat ZStack or Group), the three allow-listed
keywords (incl. the SwiftUI-unique no-emit for `auto` and the
explicit `.leftToRight` for cross-locale tables), the slot-ref
binding, the silent-drop with an injection-shaped payload, and a
no-`dir` regression guard. Total tests: 71 (was 64).

### Added — UI29-4 `HostLink` + `HostTooltip` + `HostNumberInput` (U29-4-K-swiftui)

Three new UI29-4 kernel primitives lower to native SwiftUI views:

- **`HostLink` → `Link(label, destination: URL(string: href)!)`**
  (iOS 14+/macOS 11+). OS-managed URL open by default. When
  `external: false` + `onActivate` are bound, the lowering swaps
  to a `Button(action: { dispatch(.x(href: "...")) }) { Text(label) }`
  so the host's in-app router takes over instead of opening
  externally. When `external != false` but `onActivate` is bound,
  the v1 emitter currently drops the dispatch (SwiftUI's `Link`
  has no click-hook closure); documented as a v2 follow-up.
- **`HostTooltip` → `VStack { child(ren) }.help("text")`** (macOS
  / iOS 16+). Hovering (macOS) or long-pressing (iOS) the wrapped
  view shows the tooltip; screen readers read it via
  `accessibilityHint`.
- **`HostNumberInput` → `TextField(placeholder, value: .constant(slot),
  format: .number)`** (iOS 15+/macOS 12+). `disabled` adds a
  trailing `.disabled(...)` modifier; `onChange` adds an
  `.onChange(of: slot) { dispatch(.x(value: slot)) }` modifier
  (pre-iOS-17 closure shape — host can adapt to the new
  `(old, new)` shape if needed).

5 new tests cover: bare HostLink with Link + URL, the
external-false + onActivate Button swap, HostTooltip's VStack +
.help wrapper, HostNumberInput's TextField + .number format, and
the .onChange modifier wiring.

### Added — UI29-2 `HostCheckbox` + `HostRadio` kernel primitives (U29-2-K-swiftui)

Both new primitives lower to SwiftUI `Toggle` with the platform's
default toggle style. The semantic distinction is in the dispatched
payload:

- `HostCheckbox` dispatches `checked: Bool` on every flip via a
  `Binding(get:set:)` whose setter calls `dispatch(.x(checked:
  newValue))`. Without an `onToggle` emit the binding degrades to
  `.constant(checked)` (read-only but type-checks).
- `HostRadio` dispatches `value: String` only on positive transition
  via a `Binding(get:set:)` whose setter wraps the call in `if
  newValue { dispatch(.x(value: …)) }` — flips to `false` (a sibling
  radio caused this one to deselect) are silently dropped to match
  the kernel-canonical `onSelect = "this radio was chosen"`
  semantics.
- The `group:` prop on `HostRadio` is preserved as a `// group: …`
  Swift comment ahead of the `Toggle`. SwiftUI has no implicit radio
  grouping; the comment keeps the metadata visible for a future
  structural pass that synthesises a `Picker` from sibling radios
  sharing a `group:`.
- `label:` becomes the first positional `Toggle(...)` argument
  (string literal or slot identifier); `disabled:` becomes a trailing
  `.disabled(...)` modifier.

Deferred to a follow-up:

- `HostCheckbox.indeterminate` slot. SwiftUI's `Toggle` has no
  tri-state visual; rendering a "mixed" state needs a custom
  `ToggleStyle` or an `Image` of `checkmark.square.fill`.
- `.toggleStyle(.checkbox)` for an actual checkbox look on macOS.
  That style is macOS-only and breaks iOS compilation; a follow-up
  can add platform-conditional emission or move the choice to a
  userland modifier.

9 new tests cover the bare-toggle shape, slot-driven `.constant(…)`
binding, the `Binding(get:set:)` setter for `onToggle`, string label,
`.disabled(…)` modifier, the radio's `// group:` comment, the
positive-transition setter for `onSelect`, and the slot-typed
`value:` flowing into the dispatch payload.

## [0.5.0] - 2026-05-21

### Added — UI29-1 `HostDialog` kernel primitive (U29-1-K-swiftui)

`HostDialog` is now recognised by the SwiftUI emitter and lowers to an
invisible `Color.clear.frame(width: 0, height: 0)` anchor view carrying
a `.sheet(...)` (modal=true, default) or `.popover(...)` (modal=false)
view modifier. SwiftUI exposes dialogs as view modifiers, not
standalone views; anchoring on `Color.clear` lets `HostDialog` remain
a single tree-walker node in the kernel emitter.

Prop mapping:

- `open: slot: x` → `isPresented: .constant(x)` (immutable-slot pattern;
  same `.constant(...)` choice `HostInput` uses).
- `modal: true` (default) → `.sheet(...)`.
- `modal: false` → `.popover(...)`. SwiftUI's `.popover` does NOT
  accept an `onDismiss:` argument, so when modal=false the emitter
  silently drops the `onClose` wiring — the host should observe its
  own `open` slot change and dispatch the close event itself.
- `title: "..."` / `title: slot: x` → `.navigationTitle(...)` inside
  the content closure.
- `dismiss-on-backdrop: false` → `.interactiveDismissDisabled(true)`
  inside the content closure.
- `onClose: emit: onX` → `onDismiss: { dispatch(.x) }` (`.sheet` only).

The dialog's children render inside the content closure's `VStack`,
walked via `emit_children` so nested kernel primitives lower the same
way they do anywhere else.

### Added — tests

- 8 new tests covering: empty HostDialog (Color.clear + .sheet),
  `open` slot → `.constant(x)`, `modal: true` → `.sheet`, `modal: false`
  → `.popover` (and `onClose` NOT wired), children render inside content
  VStack with correct order, `onClose` → `onDismiss` callback,
  `title` slot → `.navigationTitle` (plus string-literal sanity), and
  `dismiss-on-backdrop: false` → `.interactiveDismissDisabled(true)`
  (plus negative case for the default).

### Changed

- Recognised-vs-deferred matrix test now lists `HostDialog` as
  recognised alongside `HostTable` / `HostInput` / etc.
- Crate version bumped `0.4.0` → `0.5.0`.

### Spec

- `code/specs/UI29-1-host-dialog.md` ships in the same commit (specs
  must precede implementation per repo workflow §8). The SwiftUI
  lowering section pins exactly what this PR implements.

## [0.4.0] - 2026-05-20

### Added — UI29 `For` / `If` / `Else` meta-primitives (U29-K-swiftui)

The two UI29 meta-primitives now have SwiftUI lowerings, completing
the kernel surface for this backend.

`For (each: <slot-or-expr>, as: <name>, index: <name>?) { ... }` lowers
to SwiftUI's `ForEach`. The id keypath switches based on whether
`index:` is bound:

- `as:` only           → `ForEach(<coll>, id: \.self) { <as> in <body> }`
- `as:` + `index:`     → `ForEach(Array(<coll>.enumerated()), id: \.offset) { (<idx>, <as>) in <body> }`

`If (when: <slot-or-expr>) { <then> } Else { <else>? }` lowers to a
Swift view-builder `if`/`else`. The `Else` is paired with its
preceding `If` sibling in a peek-and-consume walk inside container
bodies, so `If`+`Else` always emit as a single `if cond { ... } else
{ ... }` block rather than two stray nodes.

For both primitives, `SlotRef` props are camelCased into Swift
identifiers and `Expr` props pass through verbatim (the moslayout
parser hands them in as the reconstructed source substring).

Orphan `Else` (an `Else` not preceded by an `If`) is rejected by the
moslayout analyzer; the emitter is defensive and renders a Swift
comment `// orphan Else — ignored` so any escapee still produces a
compilable file.

### Added — tests

12 new tests cover the For/If/Else surface: SlotRef vs Expr each, the
index-on/off id-keypath switch, body-uses-as-binding, nested For,
if-only, if/else paired, expr-condition, orphan-Else comment, the
combined expr+pair case, two adjacent If-without-Else siblings, and a
For body whose children include an If/Else pair.

## [0.3.0] - 2026-05-19

### Added — UI29 `HostTable` kernel primitive

`HostTable` is now recognised by the SwiftUI emitter and lowers to a
`VStack(alignment: .leading, spacing: 0)` of `HStack` rows rather than
SwiftUI's data-driven `Table` view. SwiftUI's `Table` needs a
`[RowType]` collection plus per-column `TableColumn(key:)` declarations
that don't naturally fall out of the structural "compose from children"
emitter — full `SwiftUI.Table` integration waits on `For`-inside-table
in a follow-up PR.

Sub-tag handling:

- `HostTableHead` → `HStack` rows whose `Text(...)` children carry
  `.bold()` modifiers.
- `HostTableBody` → `HStack` rows, plain.
- `HostTableFoot` → preceded by `Divider()`, then `HStack` rows.
- `HostTableColGroup` → emits a Swift comment
  `// HostTableColGroup ignored in SwiftUI` (no SwiftUI analog).

When a `HostTableHead` is followed by any non-head section, a
`Divider()` is auto-inserted between them so the visual head/body
separation matches the HTML `<thead>` / `<tbody>` convention.

Orphan sub-tags (`HostTableHead` / `Body` / `Foot` / `ColGroup` used
outside a `HostTable` parent) emit a self-documenting Swift comment
rather than erroring; comments are statement-level no-ops, so the
generated file still type-checks.

`part_name` on a `HostTable` is currently surfaced as a Swift comment
`// part: <name>` directly before the VStack opener. SwiftUI has no
native equivalent of CSS `part`; a future style-inlining PR can swap
this for a real modifier.

### Added — tests

- 8 new tests covering: empty HostTable, head-only (bold), body-only,
  foot preceded by divider, head+body ordering with auto-divider,
  ColGroup-emits-comment, orphan sub-tag handling, and `part_name`
  emission. The recognised-vs-deferred matrix test now lists
  `HostTable` as recognised; only `If` and `For` remain deferred.

### Crate version

- Bumped from `0.2.0` → `0.3.0`.

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
