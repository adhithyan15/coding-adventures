### Fixed — `max-width` reached nothing; Flutter was the last of eight (#14851)

With #14833 landing Compose, Flutter was the only backend of the eight still
discarding it. All seven others were measured on a minimal probe rather than
assumed: html, react and webcomponent emit `max-width`, SwiftUI
`.frame(maxWidth:)`, Qt `Layout.maximumWidth`, XAML `MaxWidth`, Compose
`.widthIn(max = ..)`.

Engram authors it on all seven of its screens and Trestle on its task list, so
a Flutter build of either rendered those as full-width sprawl.

```dart
ConstrainedBox(
  constraints: const BoxConstraints(maxWidth: 760),
  child: SizedBox(
    width: double.infinity,
    child: …,
  ),
)
```

Flutter has no max-width *argument* — `ConstrainedBox` is a widget that wraps —
so this reuses the `emit_widget_tree` / `emit_widget_tree_inner` split built
for `Opacity` in #14708, which applies a wrapping widget once on the way out
and so covers every return site by construction.

The `width: double.infinity` is not decoration: a ceiling is not a width, and
without a fill the child sizes to its content where CSS gives a block
`max-width: 760px` its parent's width up to 760. Compose needed the same
pairing. Unlike Compose the order cannot go wrong here, because the constraint
and the fill sit on different widgets rather than in one modifier chain.

`Opacity` wraps outside the cap, so a fade applies to the capped box rather
than inside it — pinned by a test.

Verified on generated projects with the real conformance widget test, not only
`flutter analyze`: Trestle and Engram both build and pass, 7 `maxWidth`
constraints emitted across Engram and 1 in Trestle, with the drop reporter
correctly no longer naming the property.

**A companion change was built, measured, and dropped.** A cap bounds its
subtree at runtime, so it looked as though `part_max_width` should mark that
subtree `width_bounded: true` before emitting it, letting a Row inside a cap
hand its children `Flexible`. Measured, it changed **not one byte** of Trestle's
or Engram's output, and a unit test placing a Row directly under a cap still
emitted zero `Flexible`. The reason is that the capped node is itself typically
a direct Row child, and

```rust
let width_bounded = width.is_some() || part_flex_grow(..).is_some()
    || (!ctx.direct_row_child && ctx.width_bounded);
```

discards an inherited flag for any direct Row child, so the mark was overwritten
immediately. Restoring boundedness from a cap is a real gap, filed separately;
it needs that formula to treat an explicit cap as a source of boundedness rather
than a threaded flag.

