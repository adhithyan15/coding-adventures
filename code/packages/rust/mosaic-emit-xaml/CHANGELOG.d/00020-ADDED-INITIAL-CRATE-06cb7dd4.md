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

