# draw_instructions_text

ASCII/Unicode text renderer for the `draw_instructions` scene model.

This package converts `DrawScene` structures into box-drawing character strings suitable for terminal output. It proves the draw-instructions abstraction is truly backend-neutral: the same scene that produces SVG can also render as Unicode art.

## How It Works

The renderer maps pixel-coordinate scenes to a fixed-width character grid. Each cell is one character. The mapping uses a configurable scale factor (default: 8px per column, 16px per row).

### Character Palette

| Character | Purpose |
|-----------|---------|
| `\u250C \u2510 \u2514 \u2518` | Corners |
| `\u2500 \u2502` | Horizontal / vertical edges |
| `\u252C \u2534 \u251C \u2524` | Tee junctions |
| `\u253C` | Cross junction |
| `\u2588` | Filled block |

### Intersection Merging

When two drawing operations overlap at the same cell, the renderer merges them using a direction bitmask (UP, DOWN, LEFT, RIGHT) and resolves the combined tag to the correct box-drawing character.

## Usage

```elixir
alias CodingAdventures.DrawInstructions
alias CodingAdventures.DrawInstructionsText

scene = DrawInstructions.create_scene(160, 48, [
  DrawInstructions.draw_rect(0, 0, 160, 48, "transparent",
    stroke: "#000", stroke_width: 1),
  DrawInstructions.draw_line(0, 16, 160, 16),
  DrawInstructions.draw_text(8, 8, "Hello", align: "start"),
])

IO.puts(DrawInstructionsText.render(scene))
```

### Custom Scale

```elixir
DrawInstructionsText.render_text(scene, scale_x: 4, scale_y: 8)
```

### With render_with

```elixir
DrawInstructions.render_with(scene, DrawInstructionsText)
```

## Dependencies

- `coding_adventures_draw_instructions` (local path dependency)

## Part of coding-adventures

This package is part of the coding-adventures educational computing stack.
