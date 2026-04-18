# paint-vm-svg

Pure F# SVG backend for `paint-vm`.

`paint-vm-svg` renders `paint-instructions` scenes to complete SVG strings,
including groups, layers, clip paths, gradients, glyph runs, and image
references. The implementation is native F#, so the educational backend stays
 symmetric with the C# port instead of delegating to it.

## Dependencies

- paint-instructions
- paint-vm

## What It Provides

- `createSvgContext` and `createSvgVM`
- `renderToSvgString`
- `assembleSvg`
- Filter, gradient, clip-path, and safe image-href handling

## Usage

```fsharp
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVmSvg

let scene =
    PaintInstructions.paintScene 200 100 "#ffffff" [
        PaintInstructions.paintRectWith
            { PaintInstructions.defaultPaintRectOptions with Fill = Some "#3b82f6"; CornerRadius = Some 4.0 }
            10 10 180 80
    ]

let svg = PaintVmSvg.renderToSvgString scene
```

## Development

```bash
bash BUILD
```
