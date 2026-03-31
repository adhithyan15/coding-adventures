# draw-instructions-svg

SVG renderer for `coding_adventures_draw_instructions` scenes.

This package serializes a generic draw scene into a complete SVG document. It
handles all instruction types (rect, text, line, group, clip) and preserves
metadata as `data-*` attributes on SVG elements.

## Usage

```ruby
require "coding_adventures_draw_instructions"
require "coding_adventures_draw_instructions_svg"

DI = CodingAdventures::DrawInstructions
SVG = CodingAdventures::DrawInstructionsSvg

rect = DI.draw_rect(x: 10, y: 20, width: 100, height: 50, fill: "#3366cc")
text = DI.draw_text(x: 60, y: 50, value: "Hello", font_weight: "bold")

scene = DI.create_scene(
  width: 200,
  height: 100,
  instructions: [rect, text],
  metadata: { label: "My Scene" }
)

# Direct call
svg_string = SVG.render_svg(scene)

# Or via render_with with the SvgRenderer class
renderer = SVG::SvgRenderer.new
svg_string = DI.render_with(scene, renderer)
```

## Development

```bash
bash BUILD
```
