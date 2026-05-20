# Changelog — mosaic-emit-xaml

## [Unreleased] — PR-5 — Component reference resolution

### Added — `ComponentRegistry` public type

- New `ComponentRegistry` + `ComponentRef` types re-exported from
  the crate root. The registry maps PascalCase tag names →
  `(xmlns_prefix, xmlns_value, package_name)` and is the input the
  emitter consumes when resolving a non-kernel tag.
- The CLI (mosaic-compile) is responsible for populating the
  registry from parsed dependency manifests; the emitter takes the
  already-resolved data and emits the XAML reference.
- Tests use the registry directly — `ComponentRegistry::new()` +
  `.register("Grid", "grid", "using:Mosaic.Package.Grid", "mosaic-pkg-grid")`.

### Changed — `from_pipeline` signature

The fourth argument changed from `manifest: Option<&()>` (a stub from
PR-1) to `registry: Option<&ComponentRegistry>`. Callers that don't
need component references continue to pass `None`; the behaviour for
them is identical to PR-4.

### Added — Non-kernel tag → `<{prefix}:{Tag} ... />` reference

When a layout node's tag isn't in the UI29 kernel:

- **With a registry** AND the tag is registered → emits
  `<{prefix}:{Tag} ... />` with the registered xmlns prefix. The
  matching `xmlns:{prefix}="{value}"` declaration lands on the
  `<UserControl>` root tag.
- **With a registry** AND the tag is NOT registered →
  `PipelineEmitError::UnknownComponent(tag)` (the spec's intended
  error for missing manifest dependency).
- **Without a registry** → `PipelineEmitError::UnsupportedPrimitive(tag)`
  (preserves pre-PR-5 behaviour for callers that don't use packages).

Kernel primitives ALWAYS win over registry entries — if a registry
happens to define an entry for `Box` / `Text` / etc., the kernel
emitter is used and the registry entry is ignored. This protects
against accidental shadowing.

### Added — Component-reference prop resolution

The emitter walks the component-reference's `props` and produces
XAML attribute fragments:

- `slot ref` → `Attribute="{x:Bind Path}"` (PascalCased)
- `string literal` → `Attribute="literal"` (XAML-escaped)
- `number` → `Attribute="N"`
- `keyword (for-bound name)` → `Attribute="{x:Bind Name}"` (treated
  as a bound name when in scope)
- `keyword (other)` → `Attribute="literal"` (passes through)
- `expr` → routed through the PR-2 ExprLowerer (bindable path or
  helper call)
- `emit ref` → DEFERRED — surfaced as a XAML comment listing the
  skipped props so the gap is visible in diffs. Host-side handler-stub
  generation is PR-5+ work and lands in a follow-up.

### Added — xmlns deduplication

Two references to the same package produce ONE `xmlns:prefix="..."`
declaration on the `<UserControl>` root. The internal map is keyed
by xmlns prefix; `BTreeMap` storage gives deterministic alphabetical
output ordering.

### Tests

- 12 new tests cover: registry register/lookup round-trip, registry
  empty-lookup misses, no-registry → UnsupportedPrimitive, empty
  registry → UnknownComponent, prefixed XAML tag emission, xmlns
  declaration injection, slot-ref / string-literal / emit-ref prop
  mapping, multi-package xmlns emission, xmlns dedup for repeated
  package use, kernel-primitive shadowing protection.
- Total: 104 tests (was 92 in PR-4, +12).

### Known limitations carried to PR-6

- **CLI integration** (`mosaic-compile --backend xaml --package-mode`)
  still pending. The CLI needs to read each dependency's
  `mosaic-package.toml`, parse it via `mosaic-package-manifest`, and
  populate the `ComponentRegistry` before invoking `from_pipeline`.
  Same status as the swiftui/qt backends.
- **Emit-ref props on component references** are surfaced as a
  comment but not wired. The host-side handler stubs and the
  package's own `Dispatch` event subscription are PR-5+ work that
  lands either at the tail end of the xaml series or in a generic
  cross-backend PR.
- **`--use-community-datagrid` flag** still inert (PR-4 carryover).

## [Unreleased] — PR-4 — HostTable + section sub-tags

### Added — `HostTable` lowering (spec §5)

- `HostTable [name] { section sub-tags... }` lowers to a hand-rolled
  `<Grid>` with `Grid.RowDefinitions` driven by the present section
  sub-tags. Each section appears at most once per HostTable; duplicates
  produce a `DuplicateTableSection` error.

### Added — Section sub-tag handling

- **`HostTableColGroup`** — recognised but ignored in PR-4 (the
  column-widths layout question per spec §5.2 needs more design).
- **`HostTableHead`** — emits as `<StackPanel Grid.Row="N" Orientation="Vertical">`
  containing the header row(s). Auto-sized row.
- **`HostTableBody`** — emits inside `<ScrollViewer Grid.Row="N" VerticalScrollBarVisibility="Auto">`
  for vertical overflow. `*`-sized row (fills remaining space).
- **`HostTableFoot`** — same shape as Head but at the last Grid.Row.
  Auto-sized.

Each section's `Row` children become `<StackPanel Orientation="Horizontal">` (via the existing `emit_stack_panel` reused from PR-1).
Sections also accept `For` and `If` children so authors can iterate /
conditionally include rows. Any other child of a section is an
`UnsupportedPrimitive` error.

### Added — Empty HostTable case

An empty HostTable (no section sub-tags) lowers to a single `<Grid/>`
self-closing element, preserving any part-style attributes.

### Added — Section sub-tags as direct nodes error

`HostTableHead` / `HostTableBody` / `HostTableFoot` / `HostTableColGroup`
appearing outside a HostTable (i.e. as direct children of a non-table
container) surface as `UnsupportedPrimitive("HostTable<X> outside HostTable")`.

### Tests

- 11 new tests covering: head-only Grid shape; head+body two-row Grid;
  body-only ScrollViewer wrap; foot-only no-ScrollViewer; full quad
  ColGroup+Head+Body+Foot Grid.Row assignment; empty HostTable; duplicate-section
  error; unknown-child-of-HostTable error; non-Row-child-of-section
  error; `For` inside a section iterating over rows; orphan section
  sub-tag at top level; part-style application on the outer `<Grid>`.
- One PR-1 test (`host_table_errors_with_unsupported_primitive`)
  updated to verify the empty-Grid lowering.
- Total: 92 tests (was 81 in PR-3, +11).

### Known limitations carried to later PRs

- **HostTableColGroup column-widths layout** — the spec §5.2 caveat
  about WinUI 3's lack of a native semantic-table control means
  column widths need either explicit `Grid.ColumnDefinitions` or
  per-cell `Width` settings. PR-4 emits the StackPanel-per-row
  structure but doesn't yet propagate column widths; the
  ColGroup sub-tag is recognised and ignored. A follow-up tackles
  this together with the `--use-community-datagrid` flag.
- **`--use-community-datagrid` flag** — exists on `EmitOptions` but
  not yet acted on. When set, future PR will switch the lowering to
  `<controls:DataGrid>` from CommunityToolkit.WinUI for full UIA
  fidelity (spec §5.3 caveat).
- **Component references** still `UnsupportedPrimitive` pending PR-5.

## [Unreleased] — PR-3 — HostInput / HostButton / HostScroll

### Added — `HostInput` lowering (spec §4.1)

- `HostInput` lowers to `<TextBox>` with the spec's attribute mapping:
  - `value: slot: V` → `Text="{x:Bind V, Mode=TwoWay}"`
  - `value: "..."` → `Text="..."` literal
  - `read-only: slot: R` → `IsReadOnly="{x:Bind R}"`
  - `read-only: true` / `false` keyword → literal `IsReadOnly="True"` / `False`
  - `placeholder: "..."` → `PlaceholderText="..."`
  - `max-length: N` → `MaxLength="N"` (integer-cast from the f64 prop)
  - `multiline: true` → adds `AcceptsReturn="True" TextWrapping="Wrap"`
- Event wiring lands as private code-behind handlers:
  - `onChange: emit: X` → `TextChanged` handler dispatching
    `XEvent.{X}(textbox.Text)` (payload-carrying)
  - `onCommit: emit: X` + `onCancel: emit: Y` → merged `KeyDown`
    handler keyed on `VirtualKey.Enter` / `VirtualKey.Escape`
  - `onFocus: emit: X` → `GotFocus` handler

### Added — `HostButton` lowering (spec §4.2)

- `HostButton` lowers to `<Button>` with:
  - `label: slot: L` → `Content="{x:Bind L}"`
  - `label: "..."` → `Content="..."` literal
  - `disabled: slot: D` → `IsEnabled="{x:Bind Not(D)}"` plus a generated
    `private bool Not(bool b) => !b;` helper added once per component
  - `disabled: true` / `false` keyword → literal `IsEnabled="False"` / `True`
  - `onClick: emit: X` → `Click` handler dispatching `XEvent.{X}()`

### Added — `HostScroll` lowering (spec §4.3)

- `HostScroll` lowers to `<ScrollViewer>` wrapping its children.
  Direction keyword maps to scroll-bar visibility:
  - default (vertical): `VerticalScrollBarVisibility="Auto"` + `HorizontalScrollBarVisibility="Disabled"`
  - `direction: horizontal`: H=Auto, V=Disabled
  - `direction: both`: both Auto

### Added — `x:Name` allocation for Host* primitives

- When the node has a `part_name`, the `x:Name` is the part name
  PascalCased (`formula-field` → `FormulaField`). Matches the spec's
  examples and the convention React/SwiftUI use for code-behind refs.
- When the node lacks a `part_name`, the emitter allocates a
  monotonically-increasing per-component counter (`HostInput_1`,
  `HostInput_2`, ...). Stable across rebuilds.

### Added — `HostHandler` registration on `EmitContext`

- The Host* event handlers are accumulated on `EmitContext::host_handlers`
  during the walk and emitted inline in the code-behind partial class
  after the PR-2 helper methods. The dedup is by handler name, mirroring
  the helper-dedup pattern.

### Tests

- 19 new tests covering: each Host* primitive's attribute mappings;
  event-handler emission (TextChanged with payload, merged KeyDown for
  Commit+Cancel, Click); `x:Name` allocation with and without
  `part_name`; the `Not(bool)` helper generation for disabled
  polarity flip; multi-counter assignment across multiple unnamed
  HostInputs.
- Two PR-1 tests
  (`host_input_errors_with_unsupported_primitive`) updated to verify
  the new successful lowering shape instead of the previous error.
- Total: 81 tests (was 62 in PR-2, +19).

### Known limitations carried forward to later PRs

- **`HostTable`** + section sub-tags still `UnsupportedPrimitive`
  pending PR-4.
- **Component references** still `UnsupportedPrimitive` pending PR-5.
- **`BoolToVisibilityConverter` C# class** still references-only;
  hosts need to ship one. A follow-up emits the converter alongside
  the rest.
- **HostInput event payload** captures the *raw* `tb.Text` of the
  `TextBox` at dispatch time. A future PR may switch to two-way
  bindings for the slot in addition to the dispatch (mirroring
  the React emitter's `e.target.value` pattern).
- **HostButton accelerator-key wiring** (e.g. `accelerator: "Ctrl+S"`
  → `KeyboardAccelerator`) is out of scope for PR-3.

## [Unreleased] — PR-2 — If / Else / For + ExprLowerer

### Added — `For` lowering (spec §6.1)

- `For (each: <expr>, as: <name>, index: <name>?) { ... }` now lowers
  to `<ItemsRepeater ItemsSource="{x:Bind ...}">` with an
  `<ItemsRepeater.ItemTemplate>` containing a `<DataTemplate
  x:DataType="local:{Component}_{AsName}Vm">`.
- One `RowVm` C# record is generated per `For` block and surfaced as
  an `EmittedFile` in `XamlEmitResult::for_view_models`. The record
  shape is
  `public sealed record {Component}_{AsName}Vm(ElementType ElementProperty[, int Index]);`.
- RowVms dedupe within a component: two `For` blocks binding the same
  `as:` name share one generated record.
- The element type is derived from the iterated slot's declared
  mosmodel type (`list<text>` → `string`, `list<number>` → `double`,
  `list<bool>` → `bool`, etc.). Expressions like `row.cells` that
  don't resolve to a typed slot default to `object`.
- `For`'s bound name and optional index are pushed into the
  `EmitContext::for_scope` for the duration of the body walk; nested
  `For` blocks are supported, innermost shadowing outermost.

### Added — `If` / `Else` lowering (spec §6.2)

- `If (when: <expr>) { ... } [Else { ... }]` lowers to twin
  `<ContentControl>`s whose `Visibility` is bound to the expression
  and (for the `Else` branch) the negation via `ConverterParameter=invert`.
- A `BoolToVisibilityConverter` resource is added to
  `<UserControl.Resources>` exactly once per component when any `If`
  is emitted (the converter implementation itself is expected to ship
  with the host project or via a future PR; for now the emitter just
  references the `x:Key`).
- `Else` is paired with the preceding `If` by the new
  `emit_xaml_children` look-ahead. A standalone `Else` errors with
  `UnsupportedPrimitive("Else without preceding If")`.

### Added — `ExprLowerer` (spec §6.3)

- A small recursive-descent parser-and-lowerer over the UI29 §3.3
  expression grammar. Returns one of:
  - `Bindable(path)` — direct `{x:Bind X}` path. Covers bare slot ref
    (`slot: foo`), bare for-bound name (`row`), boolean literal
    (`true`/`false`), and dotted member access (`row.value.bg`).
  - `Helper(call)` — a registered helper-method call (e.g.
    `Expr_a3f24b6c(R, C)`). Covers indexers (`row[c]`), comparisons
    (`==`, `!=`, `<`, `<=`, `>`, `>=`), logical (`&&`, `||`), and
    unary `!`.
  - `Unsupported(reason)` — anything else gets a human-readable
    diagnostic via `PipelineEmitError::UnsupportedExpression`.
- Helper methods land inline in the code-behind partial class as
  `private <Type> Expr_<hash>(<params>) => <body>;`. The body is a
  direct transliteration of the moslayout expression to C# (operators
  carry through identically; `slot:` becomes `this.`; for-bound names
  become PascalCased parameters).
- Helpers dedupe by name (deterministic FNV-1a hash of the original
  expression source).

### Deviation from spec §13

The spec's `if_helpers` field on `XamlEmitResult` is intended to carry
helper sources as separate files. PR-2 inlines the helpers directly
into the code-behind `partial class` instead, leaving `if_helpers`
empty. The motivation is that one `.xaml.cs` file is simpler to
review and slot into a WinUI 3 project than a sibling `.cs` per
helper. If a reviewer prefers the separate-file shape, the inlining
can flip cheaply — the registration mechanism (`EmitContext::helpers`)
already shapes the data the right way for either output.

### Tests

- 21 new tests cover: For with slot ref / numeric list / index
  binding; RowVm record shape and dedup; If with slot ref / true
  keyword / paired Else; converter resource emission and uniqueness;
  ExprLowerer for each lowering category (bindable bare ref, bindable
  dotted, bindable literal, helper indexer, helper comparison, helper
  logical, helper unary not, helper dedup, helper with for-bound
  parameters); standalone-Else error; an end-to-end `For` + `If`/`Else`
  combination producing the expected ItemsRepeater + paired
  ContentControl nesting.
- Total test count: 62 (was 41 in PR-1).

### Public API changes

- `XamlEmitResult::for_view_models` now populated (was always empty
  in PR-1).
- `XamlEmitResult::if_helpers` remains an empty `Vec` (helpers inline
  into `code_behind`; see Deviation above).
- No breaking changes to consumers.

### Known limitations carried forward to later PRs

- The `BoolToVisibilityConverter` class itself is referenced by
  `x:Key` but not emitted. Hosts need to ship one (a 5-line C# class).
  A follow-up may bundle it as a fixed asset or emit it once per
  component.
- `HostInput`, `HostButton`, `HostScroll`, `HostTable` (+ section
  sub-tags), component references — all still `UnsupportedPrimitive`
  pending PR-3 / PR-4 / PR-5.

## [0.1.0] — Unreleased — PR-1 scaffold

### Added — initial crate

First implementation per `code/specs/mosaic-emit-xaml.md` §17 PR-1.

- Public API: `from_pipeline(interface, layout, style, manifest, options)
  -> Result<XamlEmitResult, XamlEmitError>`.
- `XamlEmitResult` carries three generated source strings: `xaml`,
  `code_behind`, `events` (per spec §2 output shape). The `project`,
  `for_view_models`, and `if_helpers` fields exist on the struct but are
  always empty / `None` in PR-1 — they fill in across PR-2..PR-6.
- The nine simple UI29 kernel primitives lower:
  - `Box` → `<Border>` (or `<ContentPresenter>` for the bare-container case)
  - `Row` → `<StackPanel Orientation="Horizontal">`
  - `Column` → `<StackPanel Orientation="Vertical">`
  - `Stack` → `<Grid>` (z-axis container)
  - `Text` → `<TextBlock>` with slot-binding or literal content
  - `Image` → `<Image Source="..."/>`
  - `Spacer` → `<Rectangle/>` flex glue
  - `Divider` → `<Border BorderThickness="..."/>`
  - `Icon` → `<FontIcon Glyph="..."/>`
- UI24 event-dispatch contract: emits a `partial class {Component}Event`
  with one nested `sealed record` per declared emit, plus a `public event
  EventHandler<{Component}Event>? Dispatch;` on the UserControl.
- Slot → `DependencyProperty` translation per spec §8. The mapping table
  covers `text` / `number` / `bool` / `color` / `image` / `node` /
  `list<T>` / `list<list<T>>` from mosmodel.
- Component-name mismatch validation across `.mil` / `.mll` (the `.msl`
  is allowed to disagree per UI23 §4).
- Errors: `ComponentNameMismatch`, `UnsupportedPrimitive`,
  `UnsupportedExpression`, `UnknownComponent`, `UnmappableSlotType`,
  `UnmappableStyleProperty`, `DuplicateTableSection`, `UnsafeSlotName`,
  `UnsafeEmitName`. The PR-1 emitter only fires the first three plus the
  identifier checks; the rest become reachable in PR-2..PR-5.

### Known limitations (deferred per the spec's PR sequence)

- **`If` / `Else` / `For` / `Expr`** — these surface as
  `UnsupportedPrimitive` / `UnsupportedExpression` in PR-1. The
  `ExprLowerer` plus the `<ContentControl>` / `<ItemsRepeater>` lowerings
  land in PR-2.
- **`HostInput` / `HostButton` / `HostScroll`** — same: `UnsupportedPrimitive`
  in PR-1, real lowering in PR-3.
- **`HostTable` + section sub-tags** — `UnsupportedPrimitive` in PR-1,
  real lowering in PR-4. The four sub-tags (`HostTableColGroup`,
  `HostTableHead`, `HostTableBody`, `HostTableFoot`) are recognised by
  name only as a "you need PR-4" diagnostic.
- **Component references (non-kernel tags)** — `UnsupportedPrimitive` in
  PR-1; the manifest-driven resolver lands in PR-5.
- **`mosstyle::StyleDef`** — accepted in the signature so consumers can
  build against the stable interface today; only base `part` blocks
  inline as a `<UserControl.Resources>` `<Style>` per part. State
  blocks (`state hover { ... }`) get a placeholder
  `<VisualStateGroup>`; the full `<VisualState>` setter wiring is a
  follow-up.
- **`<UserControl.Resources>` theming cascade** — host overrides land
  with the component-reference resolver in PR-5.
- **`--use-community-datagrid`** flag — placeholder on `EmitOptions`,
  has no effect until PR-4.
- **`--package-mode`** flag — placeholder on `EmitOptions`, has no
  effect until PR-5.
- **`dotnet build` Windows-only smoke test** — gated by
  `#[cfg(target_os = "windows")]`; PR-1 includes the test scaffold but
  the actual `dotnet` CLI invocation lands once we have a real WinUI 3
  consumer (a follow-up after PR-6 builds the VisiCalc demo).
- **`mosaic-compile --backend xaml` CLI wiring** — the CLI driver's
  `run_pipeline` currently only routes `--backend react`; the swiftui
  and qt backends also aren't wired today. A small follow-up PR will
  add the three new arms together (xaml/swiftui/qt) once the team
  agrees on the multi-file output convention (XAML emits three files
  per component; pure react/swift/qt emit one each).
