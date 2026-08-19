# PaintInstructions (Swift)

Backend-neutral paint scene model for the coding-adventures monorepo.

## What it does

This package defines the tiny intermediate representation that sits between
abstract data (barcode grids, cowsay text, vector graphics) and concrete
pixel/text backends (SVG, Canvas 2D, Metal, Direct2D, terminal ASCII, …).

Rather than hard-coding "draw SVG rectangles" inside a QR encoder, encoders
produce a `PaintScene` — a structured list of paint instructions — and let a
separate backend turn that into pixels or characters.

## Types

`PaintInstruction` is a closed sum type (a Swift `enum` with associated
values — the idiomatic equivalent of the sealed class/interface hierarchy
used by this same package in Kotlin, Java, and Dart) with six cases,
matching the full `P2D02-paint-vm-ascii.md` instruction set:

| Case | Payload | Description |
|------|---------|--------------|
| `.rect` | `PaintRectInstruction` | Axis-aligned rectangle, filled and/or stroked |
| `.line` | `PaintLineInstruction` | Straight stroked line segment |
| `.glyphRun` | `PaintGlyphRunInstruction` | A run of positioned glyphs |
| `.group` | `PaintGroupInstruction` | Children sharing an optional transform/opacity |
| `.clip` | `PaintClipInstruction` | Children clipped to a rectangle |
| `.layer` | `PaintLayerInstruction` | Children sharing optional filters/blend mode/opacity/transform |

Switching over a `PaintInstruction` without a `default:` case produces a
compiler error if a new case is ever added — the same exhaustiveness safety
net `sealed` gives those other languages.

Other types: `PaintGlyphPlacement` (one glyph's position within a
`glyphRun`; for the ASCII backend, `glyphId` is a literal Unicode code
point rather than a font-internal glyph index), `Transform2D` (a 2D affine
transform), `PaintScene` (a complete frame), `PaintColorRGBA8` (a parsed
RGBA colour).

## Usage

```swift
import PaintInstructions

let scene = createScene(
    width: 100,
    height: 100,
    instructions: [
        paintRect(x: 25, y: 25, width: 50, height: 50, fill: "#ff0000"),
    ]
)

// Build via the helper functions — they return PaintInstruction directly,
// so producers never need to name the concrete payload struct.
let stroke = paintRect(x: 0, y: 0, width: 8, height: 16, fill: "", stroke: "#000000")
```

Consumers that need to inspect an instruction's concrete fields switch (or
`if case`) on it:

```swift
switch instruction {
case .rect(let r):
    print(r.x, r.y, r.width, r.height, r.fill)
case .glyphRun(let run):
    print(run.glyphs.count)
default:
    break
}
```

## Dependencies

None — pure Swift, no platform-specific code.

## Running tests

```sh
swift test
```
