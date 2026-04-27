# paint-vm-ascii

Pure F# terminal backend for `paint-vm`.

`paint-vm-ascii` executes `paint-instructions` scenes into a character grid
made from box-drawing glyphs, block fill cells, and direct text glyphs. The
implementation is native F#, not a wrapper over the C# backend, so the paint
stack stays educational and language-idiomatic on both sides.

## Dependencies

- paint-instructions
- paint-vm

## What It Provides

- `AsciiOptions` plus `defaultAsciiOptions`
- `createAsciiContext`, `createAsciiVM`, and `createAsciiVMWith`
- `renderToAscii` and `renderToAsciiWith`
- Support for rects, lines, glyph runs, groups, clips, and plain layers
- Loud failures for transforms, opacity, blend modes, and filters that a text
  backend cannot faithfully represent

## Usage

```fsharp
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVmAscii

let options = { ScaleX = 1.0; ScaleY = 1.0 }

let scene =
    PaintInstructions.paintScene 5 3 "#fff" [
        PaintInstructions.paintRectWith
            { PaintInstructions.defaultPaintRectOptions with Stroke = Some "#000" }
            0 0 4 2
    ]

let ascii = PaintVmAscii.renderToAsciiWith options scene
```

## Development

```bash
bash BUILD
```
