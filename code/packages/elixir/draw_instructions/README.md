# draw_instructions

Backend-neutral 2D draw instructions for reusable scene generation.

This package is the seam between producers and renderers. A barcode package can
emit rectangles and text, a table component can emit lines and clip regions,
and a renderer package can turn that scene into SVG or another output format.

## Primitives

| Primitive  | Purpose                                         |
|------------|-------------------------------------------------|
| `draw_rect`  | Filled or stroked rectangle                   |
| `draw_text`  | Positioned text label with optional bold       |
| `draw_line`  | Straight line segment (always stroked)         |
| `draw_group` | Semantic grouping of child instructions        |
| `draw_clip`  | Rectangular clipping region for children       |

## Usage

```elixir
alias CodingAdventures.DrawInstructions

rect = DrawInstructions.draw_rect(0, 0, 100, 50, "#cccccc", stroke: "#000000")
text = DrawInstructions.draw_text(50, 25, "Hello", font_weight: "bold")
line = DrawInstructions.draw_line(0, 50, 100, 50)
clip = DrawInstructions.draw_clip(5, 5, 90, 40, [text])

scene = DrawInstructions.create_scene(100, 100, [rect, line, clip])

# Render with any module or map renderer
DrawInstructions.render_with(scene, SomeSvgRenderer)
```

## Architecture

Producers build scenes using the helper functions.  Renderers implement
`render/1` (via the `@callback` or as a plain function) to consume scenes.
This decoupling means adding a new output format never requires changing
producer code.
