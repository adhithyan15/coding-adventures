# paint-vm-svg

Pure C# SVG backend for `paint-vm`.

`paint-vm-svg` renders `paint-instructions` scenes to a complete SVG string,
including groups, layers, clip paths, gradients, glyph runs, and image
references. The backend is DOM-free, so it works cleanly in tests, CLI tools,
and server-side rendering paths.

## Dependencies

- paint-instructions
- paint-vm

## What It Provides

- `CreateSvgContext()` and `CreateSvgVm()` for reusable execution
- `RenderToSvgString(scene)` for one-shot rendering
- `AssembleSvg(scene, context)` for manual composition
- Filter, gradient, clip-path, and safe image-href handling

## Usage

```csharp
using CodingAdventures.PaintVmSvg;
using static CodingAdventures.PaintInstructions.PaintInstructions;

var scene = PaintScene(200, 100, "#ffffff", [
    PaintRect(10, 10, 180, 80, new PaintRectOptions { Fill = "#3b82f6", CornerRadius = 4 }),
]);

var svg = PaintVmSvg.RenderToSvgString(scene);
```

## Development

```bash
bash BUILD
```
