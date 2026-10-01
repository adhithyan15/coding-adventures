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
| `HostScroll` | `<ScrollViewer>`, scrollbar visibilities from the kernel `axis` prop (UI61) |
| `HostTable` | component-scoped WinUI table controls with native UIA Table/Grid peers for the canonical dynamic shape; structural `<Grid>` fallback otherwise |
| `HostDraggable` | component-scoped `<ContentControl>` using WinUI `CanDrag`, pointer/touch drag events, keyboard operation, and UIA announcements |
| `HostDropTarget` | component-scoped `<ContentControl>` using WinUI drop events, authored acceptance filtering, keyboard traversal, and UIA announcements |
| `HostDialog` | `<ContentDialog>` / `<Flyout>` |
| `HostNavigationSplit` | `<NavigationView>` — `PaneDisplayMode="Auto"` (or `"Left"` for `collapse : never`), pane in `PaneCustomContent`, `pane-title` as `PaneTitle`, `pane-width` as `OpenPaneLength` (UI29-6) |

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

UI49 states owned by a `.mil` `one-of` slot bind to that slot's generated
dependency property and compare it with the closed-set state name through a
generated ordinal string converter. Multiple enum axes compose in model slot
order, while explicit structural and interaction predicates retain higher
precedence. Native Host controls receive the supported control setters; paint
states also reach `Box`, `Row`, `Column`, and `Stack` through their native
`Border` target, even when the part has no base style.

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

## Layout variants

A component may have more than one layout (UI30): `EngramApp.mll` and
`EngramApp.touch.mll` share `EngramApp.mil`. One WinUI app carries all of
them and the window switches between them as its environment changes (UI48
ENV2/ENV3, §7.11).

**Each layout is a control of its own.** `from_pipeline_variant` emits a
variant as `<Component><Variant>` — `variant_type_name("EngramApp", "touch")`
is `EngramAppTouch`, with `-` and `_` separating words as on every other
backend — and names every type its layout needs after it
(`EngramAppTouch_DeckVm`, `EngramAppTouchMosaicSlider`). The interface is the
default's: a WinUI project compiles all of its C# into one namespace, so the
variant declares no event union (`events` is empty) and its `Dispatch` is
`EventHandler<EngramAppEvent>`.

```rust
use mosaic_emit_xaml::pipeline::{from_pipeline_variant, EmitOptions};

let touch = from_pipeline_variant(
    &interface, &touch_layout, &style, None, "touch",
    &EmitOptions { package_exports: vec!["EngramApp".into()], ..Default::default() },
)?;
// touch.xaml        → EngramApp.touch.xaml     (x:Class="…EngramAppTouch")
// touch.code_behind → EngramApp.touch.xaml.cs  (raises EngramAppEvent)
// touch.events      → empty: EngramApp.Event.cs declares the union once
```

A variant whose type would take a name already in the namespace is refused
with `PipelineEmitError::InvalidLayoutVariant`: the component itself, its
`…Event` union or its `…Mosaic…` support types; the same for every export
in `EmitOptions::package_exports`; and the shell's own types,
`SHELL_RESERVED_NAMES` (`MainWindow`, `MosaicRuntimeHost`, `MosaicHost`, …).

**The window selects.** Give the default's `from_pipeline` the rules as
`EmitOptions::layout_variants` — one `LayoutChoice { variant, conditions }`
per rule, in order, conditions keyed by `mosaic-app-runtime`'s wire names
(`sizeClass`, `pointer`, …); the package builder computes them from
`[[app.layouts]]` or the conventions — and a control-rooted project shell
switches roots:

| piece                         | what it does                                             |
|-------------------------------|----------------------------------------------------------|
| `MainWindow.xaml`             | an empty `Grid x:Name="LayoutHost"` where the component was |
| `MosaicLayoutRules`           | the rules as data, under wire names                      |
| `MosaicLayoutVariant(env)`    | first rule whose conditions all hold, else null (default) |
| `CreateLayoutRoot(variant)`   | `new EngramAppTouch()`, wired to the one dispatch handler |
| `MountLayout(variant)`        | props first (strict in native-complete), then swap       |
| `QueueLayoutSwitch` / `SwitchLayout` | deferred to the dispatcher, one at a time; checks `MosaicRuntimeHost.IsSettling`; a failed mount keeps the old root |

The environment is `MosaicRuntimeHost.EnvironmentReport` of the window — the
same reducer the ENV4 report uses — so a shell that selects needs the
standard binding beside it, which the package builder always writes. A
`HostDialog`-rooted window does not select (`layout_root_is_dialog`). Without
`layout_variants` every project file is exactly what it was.

## Tests

`cargo test -p mosaic-emit-xaml` runs the per-primitive unit tests, plus
end-to-end smoke tests that build a small `MosmodelComponent` + `LayoutDef`
+ `StyleDef`, run `from_pipeline`, and assert structural properties of the
generated XAML and C# (presence of the right tags, attributes, and slot
properties).

`fixtures/layout-variants` is the layout-switching gate (UI48 §7.11): CI's
Windows lane builds it native-complete against the conformance runtime and
`code/scripts/mosaic-xaml-layout-variants-smoke.ps1` resizes the real window
across 600 effective pixels, both ways, asserting which root is mounted and
that the runtime's props are on it.

GitHub-hosted Windows CI is the final compiler gate for generated WinUI project
shells. A local or self-hosted Windows runner with an interactive desktop remains
the launch/interaction gate; portable tests validate the IR-to-XAML structure and
package fixtures on every platform.
