# draw-instructions

Backend-neutral 2D drawing primitives for reusable scene generation.

This package is the seam between producers and renderers. A barcode package can
emit generic rectangles, text, lines, and clip regions, and a renderer package
can turn that scene into SVG, PNG, or any other output format without learning
domain-specific rules.

## Primitives

| Primitive | Description |
|-----------|-------------|
| `DrawRectInstruction` | Filled and/or stroked rectangle |
| `DrawTextInstruction` | Positioned text label with font control |
| `DrawLineInstruction` | Straight line segment (always stroked) |
| `DrawGroupInstruction` | Hierarchical grouping of instructions |
| `DrawClipInstruction` | Rectangular clip region for children |
| `DrawScene` | Top-level container with dimensions and background |

## Usage

```ruby
require "coding_adventures_draw_instructions"

DI = CodingAdventures::DrawInstructions

rect = DI.draw_rect(x: 10, y: 20, width: 100, height: 50, fill: "#3366cc")
text = DI.draw_text(x: 60, y: 50, value: "Hello", font_weight: "bold")
line = DI.draw_line(x1: 0, y1: 0, x2: 200, y2: 0, stroke: "#ccc")

scene = DI.create_scene(
  width: 200,
  height: 100,
  instructions: [rect, text, line],
  metadata: { label: "My Scene" }
)

# Pass to any renderer (duck-typed: any object with a render method)
output = DI.render_with(scene, my_svg_renderer)
```

## Development

```bash
bash BUILD
```
