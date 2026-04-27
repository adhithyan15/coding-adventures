# paint-vm-ascii

Pure C# terminal backend for `paint-vm`.

`paint-vm-ascii` executes `paint-instructions` scenes into a character grid
made from box-drawing glyphs, block fill cells, and direct text glyphs. It is
useful for terminal previews, layout debugging, and future 1D barcode snapshot
tests where we want a deterministic text rendering target.

## Dependencies

- paint-instructions
- paint-vm

## What It Provides

- `AsciiOptions` with `ScaleX` and `ScaleY`
- `CreateAsciiContext()` and `CreateAsciiVm()` for reusable execution
- `RenderToAscii(scene, options?)` for one-shot rendering
- Support for rects, lines, glyph runs, groups, clips, and plain layers
- Loud failures for transforms, opacity, blend modes, and filters that a text
  backend cannot faithfully represent

## Usage

```csharp
using CodingAdventures.PaintVmAscii;
using static CodingAdventures.PaintInstructions.PaintInstructions;

var scene = PaintScene(5, 3, "#fff", [
    PaintRect(0, 0, 4, 2, new PaintRectOptions { Stroke = "#000" }),
]);

var ascii = PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = 1, ScaleY = 1 });
// ┌───┐
// │   │
// └───┘
```

## Development

```bash
bash BUILD
```
