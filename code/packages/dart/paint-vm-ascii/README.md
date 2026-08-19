# coding_adventures_paint_vm_ascii

A pure terminal backend for backend-neutral `PaintScene` values, implementing
the full `P2D02-paint-vm-ascii.md` contract.

## What is this?

`paint-vm-ascii` converts scene coordinates into character cells and draws:

- `rect` — filled and/or stroked rectangles (stroke via box-drawing
  corner/edge characters, fill via `█`)
- `line` — horizontal, vertical, and diagonal (Bresenham) lines
- `glyph_run` — direct character placement; `glyphId` is treated as a
  literal Unicode code point (this backend has no font resolution)
- `group` / `clip` / `layer` — recurse into children; `group`/`layer` reject
  non-identity transforms, non-default opacity, filters, and non-normal
  blend modes (returns `PaintVmAsciiErr(UnsupportedInstruction(...))` rather
  than degrading silently, per P2D02)

```dart
import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    hide version;
import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart';

void main() {
  final scene = createScene(
    width: 16,
    height: 16,
    background: 'transparent',
    instructions: [
      paintRect(x: 0, y: 0, width: 8, height: 16, fill: '#000000'),
      paintGlyphRun(
        glyphs: [PaintGlyphPlacement(glyphId: 'H'.codeUnitAt(0), x: 8, y: 0)],
        fontRef: 'terminal-mono',
        fontSize: 16,
        fill: '#000000',
      ),
    ],
  );

  switch (renderDefault(scene)) {
    case PaintVmAsciiOk(:final text):
      print(text);
    case PaintVmAsciiErr(:final error):
      print(error);
  }
}
```

Output is trimmed for terminal use: trailing spaces per line and trailing
blank rows are removed, and drawing is clipped to the scene-sized buffer.
Empty, `transparent`, and `none` fills are ignored. `PaintPath` (arbitrary
vector geometry — this repository's other Dart instruction type) returns
`PaintVmAsciiErr(UnsupportedInstruction("path"))` rather than silently
dropping unsupported geometry.

## Hardening

This backend validates every instruction's geometry before rendering it,
and bounds work independently of caller-supplied magnitudes:

- `rect`/`line`/`clip` geometry is checked for non-finite values (including
  the `x+width`/`y+height` extents a clip uses — two individually-finite
  `double`s can sum to `+Infinity`) before it's ever converted to a cell
  coordinate.
- Scene dimensions are capped both per-axis and by total cell count (a
  product-only cap is bypassable by a zero-width, huge-height scene).
- The single `double`-to-cell-index conversion point saturates its output to
  a fixed bound, so a large-but-ordinary finite coordinate can never land on
  an extreme `int` value and defeat downstream clip clamping through integer
  overflow.
- A glyph with a non-finite position is skipped (not fatal to the whole
  render); an unsafe terminal glyph (control character, bidi override, or
  UTF-16 surrogate code point) is replaced with `?`.
- `group`/`clip`/`layer` nesting is capped at 64 levels deep. Recursion has
  no other bound, so a scene built from deeply nested single-child wrapper
  instructions could otherwise exhaust the call stack — a
  `StackOverflowError` that, unlike every other error this backend reports,
  can't be caught and returned as a normal `PaintVmAsciiErr`.

### A Dart-specific relaxation

Unlike the JVM ports (Java, Kotlin), whose `Char`/`char` types are a single
fixed-width UTF-16 code unit and therefore must reject any code point above
the Basic Multilingual Plane, Dart's `String` naturally represents a
supplementary-plane code point as a surrogate pair (`String.fromCharCode`
builds it correctly). This backend accepts the full valid Unicode
scalar-value range for `glyph_run` rather than only the BMP — see
`_toSafeTerminalGlyph` in `lib/src/paint_vm_ascii.dart`.

## Requirements

- Dart SDK `^3.0.0`
- `coding_adventures_paint_instructions` (local `path:` dependency)

## Building

```sh
dart pub get
dart test
```
