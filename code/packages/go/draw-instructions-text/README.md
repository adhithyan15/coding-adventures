# draw-instructions-text (Go)

ASCII/Unicode text renderer for the `draw-instructions` scene model.

This package converts `DrawScene` objects into box-drawing character strings suitable for terminal output. It proves the draw-instructions abstraction is truly backend-neutral: the same scene that produces SVG can also render as text.

## How It Works

The renderer maps pixel-coordinate scenes to a fixed-width character grid using a configurable scale factor (default: 8px per char column, 16px per char row).

### Character Palette

| Element | Characters |
|---------|-----------|
| Corners | `\u250C \u2510 \u2514 \u2518` |
| Edges | `\u2500 \u2502` |
| Tees | `\u252C \u2534 \u251C \u2524` |
| Cross | `\u253C` |
| Fill | `\u2588` |

### Intersection Logic

When two drawing operations overlap at the same cell, the renderer merges them into the correct junction character using a direction bitmask (up, down, left, right). A horizontal line crossing a vertical line becomes `\u253C`. A line meeting a box edge becomes the appropriate tee character.

## Usage

```go
import (
    di "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions"
    text "github.com/adhithyan15/coding-adventures/code/packages/go/draw-instructions-text"
)

rect := di.DrawRect(0, 0, 160, 48, "transparent", nil)
rect.Stroke = "#000"
rect.StrokeWidth = 1

scene := di.CreateScene(168, 64, []di.DrawInstruction{
    rect,
    di.DrawLine(0, 16, 160, 16, "#000", 1, nil),
    di.DrawText(8, 8, "Hello", nil),
}, "", nil)

fmt.Println(text.RenderText(scene, nil))
```

## API

- `RenderText(scene, opts)` -- convenience function, scene in, string out
- `NewTextRenderer(opts)` -- create a reusable renderer
- `DefaultTextRenderer` -- renderer with default scale (8px/col, 16px/row)
- `TextRendererOptions{ScaleX, ScaleY}` -- control pixel-to-char mapping

## Architecture

This package is part of the draw-instructions family:

```
draw-instructions (scene model)
  |-- draw-instructions-svg (SVG output)
  |-- draw-instructions-text (this package: terminal text output)
```
