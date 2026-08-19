# com.codingadventures:paint-vm-ascii

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
  blend modes (returns `PaintVmAsciiResult.Err(UnsupportedInstruction(...))`
  rather than degrading silently, per P2D02)

```java
import com.codingadventures.paintinstructions.*;
import com.codingadventures.paintvmascii.*;

PaintScene scene = new PaintScene(16, 16, "transparent", List.of(
    PaintInstructions.paintRect(0, 0, 8, 16, "#000000"),
    PaintInstructions.paintGlyphRun(
        List.of(new PaintGlyphPlacement('H', 8, 0)), "terminal-mono", 16, "#000000")
));

PaintVmAsciiResult result = PaintVmAscii.renderDefault(scene);
if (result instanceof PaintVmAsciiResult.Ok(String text)) {
    System.out.println(text);
}
```

Output is trimmed for terminal use: trailing spaces per line and trailing
blank rows are removed, and drawing is clipped to the scene-sized buffer.
Empty, `transparent`, and `none` fills are ignored. `PaintPath` (arbitrary
vector geometry — this repository's other Java instruction type) returns
`Err(UnsupportedInstruction("path"))` rather than silently dropping
unsupported geometry.

## Hardening

This backend validates every instruction's geometry before rendering it,
and bounds work independently of caller-supplied magnitudes:

- `rect`/`line`/`clip` geometry is checked for non-finite values (including
  the `x+width`/`y+height` extents a clip uses — two individually-finite
  `double`s can sum to `+Infinity`) before it's ever converted to a cell
  coordinate.
- Scene dimensions are capped both per-axis and by total cell count (a
  product-only cap is bypassable by a zero-width, huge-height scene).
- The single `Double`-to-cell-index conversion point saturates its output
  to a fixed bound, so a large-but-ordinary finite coordinate can never
  land on `Integer.MIN_VALUE`/`MAX_VALUE` and defeat downstream clip
  clamping through integer overflow.
- A glyph with a non-finite position is skipped (not fatal to the whole
  render); an unsafe terminal glyph (control character, bidi override,
  UTF-16 surrogate, or any code point that would need a surrogate pair to
  represent as a `char`) is replaced with `?`.

## Requirements

- Java 21
- `com.codingadventures:paint-instructions` (local composite build)

## Building

```sh
gradle test
```
