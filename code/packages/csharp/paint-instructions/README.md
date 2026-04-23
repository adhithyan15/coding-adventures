# paint-instructions

Pure C# paint IR records for describing 2D scenes without choosing a renderer.

`paint-instructions` sits between layout and rendering. Barcode layout,
document rendering, and future paint backends can all exchange one portable
scene model made of records, enums, and tagged instruction variants.

## Dependencies

- pixel-container

## What It Provides

- Scene, shape, layer, clip, gradient, glyph, and image instruction records
- Path command and filter effect variants with stable `Kind` strings
- Builder helpers that keep call sites compact while returning plain records
- Image sources that can point at URIs or in-memory `PixelContainer` buffers

## Usage

```csharp
using CodingAdventures.PaintInstructions;
using static CodingAdventures.PaintInstructions.PaintInstructions;

var scene = PaintScene(
    320,
    120,
    "#ffffff",
    [
        PaintRect(0, 0, 320, 120, new PaintRectOptions { Fill = "#ffffff" }),
        PaintLine(16, 64, 304, 64, "#111827", new PaintLineOptions { StrokeWidth = 2 }),
    ],
    new SceneOptions { Id = "barcode-canvas" });
```

## Development

```bash
bash BUILD
```
