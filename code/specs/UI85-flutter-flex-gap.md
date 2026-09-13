# UI85 — Flutter flex gap

Issue: [#14804](https://github.com/adhithyan15/coding-adventures/issues/14804)

## Status

Accepted for implementation.

## Problem

Mosstyle's `gap` reaches Flutter's emitter but disappears before generated
Dart. The loss is visible in Trestle, Engram, VisiCalc, Venture, and every
other product whose `Row` or `Column` relies on authored spacing. It is not a
string-concatenation defect: adjacent widgets are rendered with no separator.

Flutter's `Row` and `Column` do not expose a `gap` or `spacing` argument.
Replacing them with `Wrap` would change layout semantics: a non-wrapping row
could wrap, flex constraints would change, and `Expanded` children would no
longer be legal in the same way. The lowering must therefore preserve the
existing flex container and insert spacing between its evaluated children.

## Decision

An authored `gap` on a `Row` or `Column` wraps that container's completed
`List<Widget>` in one generated helper:

```dart
children: _mosaicWithGap(
  <Widget>[
    first,
    if (visible) second,
    ...items.map(buildItem),
  ],
  8,
  Axis.horizontal,
),
```

The helper walks the already-evaluated list and inserts a `SizedBox` before
every child after the first. Horizontal containers use `SizedBox(width: gap)`;
vertical containers use `SizedBox(height: gap)`.

This order is load-bearing. Mosaic conditionals and loops lower to Dart
collection expressions. Inserting separators into the source list would have
to predict which conditional children survive at runtime and could create
leading, trailing, or doubled gaps. Operating on the completed list spaces the
widgets that actually exist.

The helper is emitted exactly once per generated file and only when at least
one `Row` or `Column` has an authored `gap`. A `gap` on `Stack`, `Box`, or a
non-flex primitive remains unsupported; it must not silently acquire flex
semantics. UI84's consumption-based drop reporter is responsible for making
those remaining losses explicit.

## Value handling

The emitter uses the same finite pixel parser as existing Flutter width,
height, and padding lowering. Mosstyle pixel values therefore become a Dart
`double`; an invalid or non-finite defensive input becomes `0`, preserving the
emitter's existing compile-safe fallback. A zero gap returns the original list
without allocating separators.

## Acceptance criteria

- `Row [part]` plus `gap: N` preserves `Row` and inserts horizontal spacing.
- `Column [part]` plus `gap: N` preserves `Column` and inserts vertical spacing.
- Conditional and repeated children are spaced after their Dart collection
  expressions have evaluated.
- `Row`, `Column`, `Stack`, and `Box` without an applicable gap emit
  byte-for-byte-equivalent child-list shapes and do not pay for the helper.
- Multiple gapped containers emit one helper definition.
- The Flutter emitter's focused Rust tests and lint checks pass.

## Non-goals

- Implementing `flex-wrap` or replacing flex containers with `Wrap`.
- Treating `gap` as meaningful on `Stack` or arbitrary widgets.
- Completing Flutter or Qt style-drop reporting; that mechanism is specified
  separately by UI84 and tracked by #12022.
- Claiming pixel-identical output across native backends. The contract is the
  shared spacing intent expressed through Flutter-native layout widgets.
