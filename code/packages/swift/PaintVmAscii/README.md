# PaintVmAscii (Swift)

A pure terminal backend for backend-neutral `PaintScene` values, implementing
the full `P2D02-paint-vm-ascii.md` contract.

## What is this?

`PaintVmAscii` converts scene coordinates into character cells and draws:

- `.rect` — filled and/or stroked rectangles (stroke via box-drawing
  corner/edge characters, fill via `█`)
- `.line` — horizontal, vertical, and diagonal (Bresenham) lines
- `.glyphRun` — direct character placement; `glyphId` is treated as a
  literal Unicode code point (this backend has no font resolution)
- `.group` / `.clip` / `.layer` — recurse into children; `.group`/`.layer`
  reject non-identity transforms, non-default opacity, filters, and
  non-normal blend modes (`render` throws `PaintVmAsciiError.unsupportedInstruction`
  rather than degrading silently, per P2D02)

```swift
import PaintInstructions
import PaintVmAscii

let scene = createScene(
    width: 16,
    height: 16,
    instructions: [
        paintRect(x: 0, y: 0, width: 8, height: 16, fill: "#000000"),
        paintGlyphRun(
            glyphs: [PaintGlyphPlacement(glyphId: Int(Character("H").asciiValue!), x: 8, y: 0)],
            fontRef: "terminal-mono", fontSize: 16, fill: "#000000"
        ),
    ],
    background: "transparent"
)

let text = try renderDefault(scene)
print(text)
```

Output is trimmed for terminal use: trailing spaces per line and trailing
blank rows are removed, and drawing is clipped to the scene-sized buffer.
Empty, `transparent`, and `none` fills are ignored.

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
  land on an extreme `Int` value and defeat downstream clip clamping
  through integer overflow.
- `group`/`clip`/`layer` nesting is capped at 64 levels deep, closing a
  `StackOverflowError`-class hang for a deeply nested scene.
- A glyph with a non-finite position is skipped (not fatal to the whole
  render); an unsafe terminal glyph (control character, bidi override, or
  UTF-16 surrogate code point) is replaced with `?`.
- The diagonal-line Bresenham loop seeds its error term to
  `deltaCol - deltaRow` (the standard initialization), not `0` — a
  zero-seeded error term can overshoot the target row for some slopes and
  loop forever. The same bug was found in this package's Haskell/Java/
  Kotlin siblings; see
  [issue #12093](https://github.com/adhithyan15/coding-adventures/issues/12093).

### A Swift-specific hardening note

Unlike Dart/Kotlin/Java, Swift's `Int` arithmetic operators (`+`, `-`, `*`)
**trap** on overflow rather than wrapping or silently producing a wrong
result. Two places in this backend deliberately avoid `Int` addition on
caller-supplied values for exactly that reason — see the comments on
`ceilDiv` and `renderRectangle`'s `x + width`/`y + height` computation in
`Sources/PaintVmAscii/PaintVmAscii.swift`. Both are rewritten to do the
arithmetic in `Double` (which saturates rather than trapping) instead.

## Requirements

- Swift 5.9+
- `PaintInstructions` (local package dependency)

## Building

```sh
swift test
```
