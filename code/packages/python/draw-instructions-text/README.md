# draw-instructions-text (Python)

ASCII/Unicode text renderer for the `draw-instructions` scene model.

This renderer converts `DrawScene` objects into box-drawing character strings,
proving that the draw-instructions abstraction is truly backend-neutral: the
same scene that produces SVG can also render as terminal-friendly text.

## How It Works

The renderer maps pixel-coordinate scenes to a fixed-width character grid.
Each cell is one character. The mapping uses a configurable scale factor
(default: 8 px per column, 16 px per row).

### Character Palette

| Purpose  | Characters              |
|----------|-------------------------|
| Corners  | `\u250c` `\u2510` `\u2514` `\u2518`             |
| Edges    | `\u2500` `\u2502`                   |
| Tees     | `\u252c` `\u2534` `\u251c` `\u2524`             |
| Cross    | `\u253c`                       |
| Fill     | `\u2588`                       |

### Intersection Logic

When two drawing operations overlap at the same cell, direction flags are
OR-ed together and resolved to the correct junction character. A horizontal
line crossing a vertical line becomes `\u253c`. A line meeting a box edge becomes
the appropriate tee (`\u252c` `\u2534` `\u251c` `\u2524`).

## Installation

```bash
pip install coding-adventures-draw-instructions-text
```

## Usage

```python
from draw_instructions import create_scene, draw_rect, draw_line, draw_text
from draw_instructions_text import render_text

scene = create_scene(160, 48, [
    draw_rect(0, 0, 160, 48, "transparent", stroke="#000", stroke_width=1),
    draw_line(0, 16, 160, 16, "#000", 1),
    draw_text(8, 12, "Hello", align="start"),
])

print(render_text(scene))
```

## API

### `render_text(scene, *, scale_x=8, scale_y=16) -> str`

Convenience function that renders a scene with the given scale.

### `TextRenderer(*, scale_x=8, scale_y=16)`

A renderer class implementing the `DrawRenderer[str]` protocol. Use with
`render_with()` from draw-instructions.

### `TEXT_RENDERER`

Pre-configured `TextRenderer` instance with default scale.
