# paint-vm-ascii (Haskell)

A pure terminal backend for backend-neutral `PaintScene` values. It converts
scene coordinates into character cells and draws visible filled rectangles
with Unicode full-block characters.

```haskell
import CodingAdventures.PaintInstructions
import CodingAdventures.PaintVmAscii

scene = (emptyScene 16 16 "transparent")
  { psInstructions = [makeRect 0 0 8 16 "#000000"] }

main = putStrLn (either show id (renderDefault scene))
```

`AsciiOptions` defaults to an 8-by-16 scene-unit character cell. Output rows
and trailing blank rows are trimmed, and drawing is clipped to the scene-sized
buffer. Empty, `transparent`, and `none` fills are ignored.

The current Haskell paint IR exposes rectangles and paths. This backend
supports rectangles and returns `UnsupportedInstruction "path"` for paths,
so unsupported vector geometry cannot silently disappear from the output.

## Development

Run the tests from this directory:

```sh
cabal test all
```
