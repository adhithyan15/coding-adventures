# draw-instructions-text (Rust)

ASCII/Unicode text renderer for the `draw-instructions` scene model.

This crate converts `DrawScene` objects into box-drawing character strings,
proving that the draw-instructions abstraction is truly backend-neutral: the
same scene that produces SVG can also render as terminal-friendly text.

## How It Works

The renderer maps pixel-coordinate scenes to a fixed-width character grid.
Each cell is one character. The mapping uses a configurable scale factor
(default: 8 px per column, 16 px per row).

### Intersection Logic

When two drawing operations overlap at the same cell, direction flags are
OR-ed together and resolved to the correct junction character. A horizontal
line crossing a vertical line becomes a cross. A line meeting a box edge
becomes the appropriate tee.

## Usage

```rust
use draw_instructions::{create_scene, draw_rect, draw_line, Metadata};
use draw_instructions_text::{render_text, TextRenderer};

let scene = create_scene(
    160, 48,
    vec![
        draw_rect(0, 0, 160, 48, "transparent", Metadata::new()),
        draw_line(0.0, 16.0, 160.0, 16.0, "#000", 1.0),
    ],
    "", Metadata::new(),
);

println!("{}", render_text(&scene));
```

## API

### `render_text(scene: &DrawScene) -> String`

Renders a scene using default scale (8 px/col, 16 px/row).

### `TextRenderer::new() -> TextRenderer`

Creates a renderer with default scale.

### `TextRenderer::with_scale(scale_x: f64, scale_y: f64) -> TextRenderer`

Creates a renderer with custom scale factors.

### `TextRenderer` implements `Renderer<String>`

Can be used with `render_with()` from the draw-instructions crate.
