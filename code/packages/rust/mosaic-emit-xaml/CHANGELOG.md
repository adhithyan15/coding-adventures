# Changelog — mosaic-emit-xaml

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
