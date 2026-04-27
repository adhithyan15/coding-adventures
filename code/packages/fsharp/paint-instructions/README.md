# paint-instructions

Pure F# paint IR unions and records for describing 2D scenes without choosing a renderer.

`paint-instructions` sits between layout and rendering. Barcode layout,
document rendering, and future paint backends can all exchange one portable
scene model made of records, discriminated unions, and stable instruction tags.

## Dependencies

- pixel-container

## What It Provides

- Scene, shape, layer, clip, gradient, glyph, and image instruction records
- Path command and filter effect unions with stable `Kind` members
- Builder helpers that keep call sites compact while returning plain data
- Image sources that can point at URIs or in-memory `PixelContainer` buffers

## Usage

```fsharp
open CodingAdventures.PaintInstructions

let scene =
    PaintInstructions.paintScene
        320
        120
        "#ffffff"
        [
            PaintInstructions.paintRect 0 0 320 120
            PaintInstructions.paintLine 16 64 304 64 "#111827"
        ]
```

## Development

```bash
bash BUILD
```
