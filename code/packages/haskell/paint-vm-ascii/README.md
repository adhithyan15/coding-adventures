# paint-vm-ascii (Haskell)

A pure terminal backend for backend-neutral `PaintScene` values. It converts
scene coordinates into character cells and implements the full
`P2D02-paint-vm-ascii.md` contract:

- `rect` — filled and/or stroked rectangles (stroke via box-drawing
  corner/edge characters, fill via `█`)
- `line` — horizontal, vertical, and diagonal (Bresenham) lines
- `glyph_run` — direct character placement; `glyph_id` is treated as a
  literal Unicode code point (this backend has no font resolution)
- `group` / `clip` / `layer` — recurse into children; `group`/`layer` reject
  non-identity transforms, non-default opacity, filters, and non-normal
  blend modes (returns `Left (UnsupportedInstruction ...)` rather than
  degrading silently, per P2D02)

```haskell
import CodingAdventures.PaintInstructions
import CodingAdventures.PaintVmAscii

scene = (emptyScene 16 16 "transparent")
  { psInstructions =
      [ makeRect 0 0 8 16 "#000000"
      , makeGlyphRun [PaintGlyphPlacement (fromEnum 'H') 8 0] "terminal-mono" 16 "#000000"
      ]
  }

main = putStrLn (either show id (renderDefault scene))
```

`AsciiOptions` defaults to an 8-by-16 scene-unit character cell. Output rows
and trailing blank rows are trimmed, and drawing is clipped to the scene-sized
buffer. Empty, `transparent`, and `none` fills are ignored.

The Haskell paint IR's remaining instruction kind is `path` (arbitrary
vector geometry); this backend returns `UnsupportedInstruction "path"` for
it rather than silently dropping unsupported geometry.

## Development

Run the tests from this directory:

```sh
cabal test all
```
