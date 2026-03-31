# draw_instructions_svg

SVG renderer for backend-neutral draw instructions.

This package serializes generic draw scenes to SVG without knowing any
producer-specific rules (barcodes, tables, charts, etc.).  It is intentionally
boring: one job, done well.

## Supported primitives

| Instruction | SVG output                              |
|-------------|-----------------------------------------|
| `:rect`     | `<rect>` with optional stroke attrs     |
| `:text`     | `<text>` with optional font-weight      |
| `:line`     | `<line>` with stroke attrs              |
| `:group`    | `<g>` wrapping children                 |
| `:clip`     | `<clipPath>` + clipped `<g>`            |

## Usage

```elixir
alias CodingAdventures.DrawInstructions
alias CodingAdventures.DrawInstructionsSvg

scene =
  DrawInstructions.create_scene(200, 100, [
    DrawInstructions.draw_rect(0, 0, 200, 100, "#ffffff", stroke: "#000000"),
    DrawInstructions.draw_text(100, 50, "Hello SVG", font_weight: "bold"),
    DrawInstructions.draw_line(0, 80, 200, 80, "#cccccc")
  ])

svg_string = DrawInstructionsSvg.render(scene)
# Or via the behaviour:
svg_string = DrawInstructions.render_with(scene, DrawInstructionsSvg)
```

## Architecture

This module implements the `CodingAdventures.DrawInstructions` behaviour
(`@callback render/1`), so it can be passed directly to `render_with/2`.
Metadata is serialized as `data-*` attributes for downstream tooling.
