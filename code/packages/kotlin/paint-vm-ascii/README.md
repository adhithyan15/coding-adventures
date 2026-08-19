# paint-vm-ascii (Kotlin)

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

```kotlin
import com.codingadventures.paintinstructions.*
import com.codingadventures.paintvmascii.*

val scene = PaintScene(16, 16, "transparent", listOf(
    paintRect(0, 0, 8, 16, "#000000"),
    paintGlyphRun(listOf(PaintGlyphPlacement('H'.code, 8.0, 0.0)), "terminal-mono", 16.0, "#000000"),
))

when (val result = renderDefault(scene)) {
    is PaintVmAsciiResult.Ok -> println(result.text)
    is PaintVmAsciiResult.Err -> println(result.error)
}
```

Output is trimmed for terminal use: trailing spaces per line and trailing
blank rows are removed, and drawing is clipped to the scene-sized buffer.
Empty, `transparent`, and `none` fills are ignored. `PaintPath` (arbitrary
vector geometry — this repository's other Kotlin instruction type) returns
`Err(UnsupportedInstruction("path"))` rather than silently dropping
unsupported geometry.

## Hardening

This backend validates every instruction's geometry before rendering it,
and bounds work independently of caller-supplied magnitudes:

- `rect`/`line`/`clip` geometry is checked for non-finite values (including
  the `x+width`/`y+height` extents a clip uses — two individually-finite
  `Double`s can sum to `+Infinity`) before it's ever converted to a cell
  coordinate.
- Scene dimensions are capped both per-axis and by total cell count (a
  product-only cap is bypassable by a zero-width, huge-height scene).
- The single `Double`-to-cell-index conversion point saturates its output
  to a fixed bound, so a large-but-ordinary finite coordinate can never
  land on `Int.MIN_VALUE`/`MAX_VALUE` and defeat downstream clip clamping
  through integer overflow.
- A glyph with a non-finite position is skipped (not fatal to the whole
  render); an unsafe terminal glyph (control character, bidi override,
  UTF-16 surrogate, or any code point that would need a surrogate pair to
  represent as a JVM `Char`) is replaced with `?`.

## Requirements

- Kotlin 2.1.20, JVM target 21
- `com.codingadventures:paint-instructions` (local composite build)

## Building

```sh
gradle test
```
