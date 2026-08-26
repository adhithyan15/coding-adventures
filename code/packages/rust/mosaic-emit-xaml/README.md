# mosaic-emit-xaml

WinUI 3 / XAML backend for the Mosaic three-language pipeline. Lowers a
`.mil` + `.mll` + `.msl` triple to a WinUI 3 `UserControl` — a triple of
generated files (`.xaml` + `.xaml.cs` + `.Event.cs`) — that consumers can
drop into a Windows App SDK project.

See `code/specs/mosaic-emit-xaml.md` for the design.

## Status

The backend covers the structural kernel, control flow, native Host controls,
component references, event dispatch, project-shell generation, and
mosstyle base styling:

| UI29 primitive | XAML lowering |
|---|---|
| `Box`       | `<Border>` (or `<ContentPresenter>` when no padding/background) |
| `Row`       | `<Grid>` + one `ColumnDefinition` per child, with flex sizing (§3.1) |
| `Column`    | `<Grid>` + one `RowDefinition` per child, with flex sizing (§3.1) |
| `Stack`     | `<Grid>` (single cell, all children stack on z-axis) |
| `Text`      | `<TextBlock>` |
| `Image`     | `<Image Source="..."/>` |
| `Spacer`    | `<Rectangle/>` glue |
| `Divider`   | `<Border BorderThickness="..." />` |
| `Icon`      | `<FontIcon Glyph="..."/>` |
| `If` / `Else` | `<ContentControl>` with bound visibility |
| `For` | `<ItemsRepeater>` with generated row view-models |
| `Input` | `<TextBox>` with `AcceptsReturn` and wrapping for `multiline: true` |
| `HostInput` | `<TextBox>` |
| `HostButton` | `<Button>` |
| `HostSurface` | Styled `<Border>` containing a node-bound `<ContentPresenter>` |
| `HostCheckbox` | `<CheckBox>` |
| `HostRadio` | `<RadioButton>` |
| `HostSlider` | component-scoped native `<Slider>` with change and pointer/key/blur commit events |
| `HostLink` | `<HyperlinkButton>` or routed `<Button>` |
| `HostNumberInput` | `<NumberBox>` |
| `HostScroll` | `<ScrollViewer>` |
| `HostTable` | component-scoped WinUI table controls with native UIA Table/Grid peers for the canonical dynamic shape; structural `<Grid>` fallback otherwise |
| `HostDraggable` | component-scoped `<ContentControl>` using WinUI `CanDrag`, pointer/touch drag events, keyboard operation, and UIA announcements |
| `HostDropTarget` | component-scoped `<ContentControl>` using WinUI drop events, authored acceptance filtering, keyboard traversal, and UIA announcements |
| `HostDialog` | `<ContentDialog>` / `<Flyout>` |

Plus the UI24 event-dispatch contract (one `Dispatch` event per UserControl)
and slot → `DependencyProperty` translation.

Canonical UI31 tables keep their authored `Grid`/`ItemsRepeater` visuals and
arbitrary interactive cell subtree, while generated automation peers expose
table dimensions, header associations, row/column coordinates, accessible cell
names, and arrow-key navigation. Structurally ambiguous tables retain the visual
fallback and remain visible to native-completeness reporting rather than claiming
semantics the emitter cannot prove.

UI35 drag/drop uses WinUI's native `DragStarting`, `DragEnter`, `DragOver`,
`DragLeave`, `Drop`, and `DropCompleted` lifecycle. Each generated component
instance owns an isolated target scope, so nested or repeated components cannot
accept one another's internal drags. Space/Enter grab and drop, arrow keys move
between eligible targets (including RTL order), and Escape cancels. Pointer,
touch, and keyboard paths share the same authored `accepts` filter and accepted
drop payload; focusable source/target peers publish names, help text, and live UIA
notifications without changing application state locally.

`HostSlider` uses WinUI's native adjustable control and inherited RangeValue
automation peer. The generated component-scoped subclass suppresses
initialization and unfocused controlled updates, keeps live change callbacks
separate from release/key/blur commits, and preserves native focus, touch,
keyboard, high-contrast, and platform-theme behavior. Positive `step` values
map to native snap points. `step: 0` configures enough native stops to remain
below physical pointer resolution while retaining a usable one-percent keyboard
increment.

## MSL states and motion

Native Host controls consume `state-when-*` layout predicates and matching
MSL state blocks. The emitter writes property-scoped WinUI
`VisualStateGroup`s so different properties can keep distinct transition
durations and easing curves. The groups live on a transparent first-child
`Grid`, as required for WinUI to evaluate declarative triggers automatically.
Part-level transitions animate entry and exit; state-local transitions
override entry.

UI15's built-in `state hover` needs no matching layout predicate on controls
that lower to WinUI's native ButtonBase family (`HostButton`, `HostCheckbox`,
`HostRadio`, and `HostLink`). Its trigger binds directly to `IsPointerOver`.
Inside a `For`, the binding and VisualStates remain in the DataTemplate
namescope, so hovering one repeated row does not restyle its siblings. An
explicit `state-when-hover` still wins when hover-like styling is intentionally
driven by application state instead of the pointer.

UI15's built-in `state focused` is also native for focus-capable Host controls
(`HostInput`, `HostNumberInput`, `HostButton`, `HostCheckbox`, `HostRadio`,
`HostSlider`, and `HostLink`). The emitter binds WinUI's `FocusState` through one generated
converter resource, so pointer, keyboard, and programmatic focus activate the
same shared MSL properties and transitions. Repeated controls keep their
VisualStates in the DataTemplate namescope. An explicit
`state-when-focused` remains application-controlled.

UI15's built-in `state pressed` is native for the same ButtonBase family used
by hover (`HostButton`, `HostCheckbox`, `HostRadio`, and `HostLink`). Its
trigger binds directly to `IsPressed`, remains row-local inside DataTemplates,
and takes precedence over simultaneous focused or hover styling. An explicit
`state-when-pressed` remains application-controlled.

Named CSS curves lower to native WinUI easing functions. Arbitrary
`cubic-bezier(...)` curves currently use `CubicEase`; exact control points
require a future Windows Composition lowering. Template-local predicates are
lowered when they can bind directly to the row view-model or be projected into
it; unsupported cross-namescope expressions are omitted instead of generating
invalid XAML.

Any moslayout primitive not in the table above currently surfaces as
`PipelineEmitError::UnsupportedPrimitive` so authors get a clear "not yet
supported" diagnostic instead of broken XAML.

## Output shape

For a component `MyComponent` the emitter returns:

- `xaml`: full XAML markup with `<UserControl>` root, embedded
  resources, native VisualStates from mosstyle, and the lowered moslayout
  tree as content.
- `code_behind`: the C# `partial class MyComponent : UserControl` with one
  `DependencyProperty` per `.mil` slot, a `Dispatch` event, and the
  `InitializeComponent()` boilerplate.
- `events`: the discriminated event-union as C# records (one nested record
  per emit, matching UI24 §3.1's `export type GridEvent = ...` shape).
  Non-empty unions also expose `MosaicName`, `MosaicPayload`, and
  `MosaicEnvelope`, preserving original emit names such as `onReveal` and
  payload keys such as `value` or `checked` for native host bridges.

## Public API

```rust
use mosaic_emit_xaml::{from_pipeline, EmitOptions, XamlEmitResult};

let result: XamlEmitResult = from_pipeline(
    &interface,   // MosmodelComponent (.mil)
    &layout,      // LayoutDef        (.mll)
    &style,       // StyleDef         (.msl)
    None,         // optional package manifest — deferred to PR-5
    &EmitOptions::default(),
)?;
// result.xaml         → write to MyComponent.xaml
// result.code_behind  → write to MyComponent.xaml.cs
// result.events       → write to MyComponent.Event.cs
```

`mosaic-compile --backend xaml` wires this up at the CLI level in the same
PR.

## Project-shell runtime policy

`EmitOptions::require_runtime` selects the `native-complete` project shell. It
loads Mosaic's standard .NET/Rust binding before WinUI activates the window,
validates every required MIL prop before showing the generated component, and
routes all generated events back through the Rust engine. This shell contains no
reflection-host or sample-prop fallback. The default `false` setting retains the
permissive preview and compatibility behavior.

## Tests

`cargo test -p mosaic-emit-xaml` runs the per-primitive unit tests, plus
end-to-end smoke tests that build a small `MosmodelComponent` + `LayoutDef`
+ `StyleDef`, run `from_pipeline`, and assert structural properties of the
generated XAML and C# (presence of the right tags, attributes, and slot
properties).

GitHub-hosted Windows CI is the final compiler gate for generated WinUI project
shells. A local or self-hosted Windows runner with an interactive desktop remains
the launch/interaction gate; portable tests validate the IR-to-XAML structure and
package fixtures on every platform.
