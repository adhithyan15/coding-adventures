# mosaic-emit-xaml

WinUI 3 / XAML backend for the Mosaic three-language pipeline. Lowers a
`.mil` + `.mll` + `.msl` triple to a WinUI 3 `UserControl` — a triple of
generated files (`.xaml` + `.xaml.cs` + `.Event.cs`) — that consumers can
drop into a Windows App SDK project.

See `code/specs/mosaic-emit-xaml.md` for the design.

## Status: PR-1 — scaffold + simple kernel primitives

This first PR implements the scaffold and the **nine simple kernel
primitives** from UI29 §2.1:

| UI29 primitive | XAML lowering |
|---|---|
| `Box`       | `<Border>` (or `<ContentPresenter>` when no padding/background) |
| `Row`       | `<StackPanel Orientation="Horizontal">` |
| `Column`    | `<StackPanel Orientation="Vertical">` |
| `Stack`     | `<Grid>` (single cell, all children stack on z-axis) |
| `Text`      | `<TextBlock>` |
| `Image`     | `<Image Source="..."/>` |
| `Spacer`    | `<Rectangle/>` glue |
| `Divider`   | `<Border BorderThickness="..." />` |
| `Icon`      | `<FontIcon Glyph="..."/>` |

Plus the UI24 event-dispatch contract (one `Dispatch` event per UserControl)
and slot → `DependencyProperty` translation.

## What is NOT in this first PR

The remaining six UI29 kernel primitives and the component-reference
resolver are tracked as follow-ups per the spec's §17:

- **PR-2**: `If` / `Else` / `For` lowering + the `ExprLowerer`
- **PR-3**: `HostInput`, `HostButton`, `HostScroll`
- **PR-4**: `HostTable` with its four section sub-tags
- **PR-5**: component-reference resolver (`<pkg:ComponentName/>`) +
  `--package-mode` CLI flag
- **PR-6**: `mosaic-pkg-grid` compiled through this backend + VisiCalc
  Windows demo

Any moslayout primitive not in the table above currently surfaces as
`PipelineEmitError::UnsupportedPrimitive` so authors get a clear "not yet
supported" diagnostic instead of broken XAML.

## Output shape

For a component `MyComponent` the emitter returns:

- `xaml`: full XAML markup with `<UserControl>` root, embedded
  `<UserControl.Resources>` from the mosstyle (deferred for PR-1 — only
  base `part` blocks land), lowered moslayout tree as the content.
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

## Tests

`cargo test -p mosaic-emit-xaml` runs the per-primitive unit tests, plus
end-to-end smoke tests that build a small `MosmodelComponent` + `LayoutDef`
+ `StyleDef`, run `from_pipeline`, and assert structural properties of the
generated XAML and C# (presence of the right tags, attributes, and slot
properties).

The actual `dotnet build` smoke test against the emitted output is a
Windows-only follow-up — gated by `#[cfg(target_os = "windows")]` and
running on the `windows-latest` CI matrix.
