# draw-instructions-text (Ruby)

ASCII/Unicode text renderer for the `coding_adventures_draw_instructions` scene model.

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

When two drawing operations overlap at the same cell, the renderer merges them into the correct junction character using a direction bitmask (up, down, left, right).

## Usage

```ruby
require "coding_adventures_draw_instructions_text"

DI = CodingAdventures::DrawInstructions
DIT = CodingAdventures::DrawInstructionsText

scene = DI.create_scene(width: 160, height: 48, instructions: [
  DI.draw_rect(x: 0, y: 0, width: 160, height: 48, fill: "transparent",
               stroke: "#000", stroke_width: 1),
  DI.draw_line(x1: 0, y1: 16, x2: 160, y2: 16),
  DI.draw_text(x: 8, y: 8, value: "Hello", align: "start"),
])

puts DIT.render_text(scene)
```

## API

- `DrawInstructionsText.render_text(scene, scale_x:, scale_y:)` -- scene in, string out
- `DrawInstructionsText::TextRenderer.new(scale_x:, scale_y:)` -- duck-typed renderer for use with `render_with`

## Architecture

This package is part of the draw-instructions family:

```
coding_adventures_draw_instructions (scene model)
  |-- coding_adventures_draw_instructions_svg (SVG output)
  |-- coding_adventures_draw_instructions_text (this package: terminal text output)
```
