# diagram-to-paint

Lowers a `LayoutedGraphDiagram` (from `diagram-layout-graph`) into a `PaintScene`
(from `paint-instructions`) for rendering by `paint-metal` and other backends.

## Usage

```rust
use dot_parser::parse_to_diagram;
use diagram_layout_graph::layout_graph_diagram;
use diagram_to_paint::diagram_to_paint;

let diagram = parse_to_diagram("digraph G { A -> B -> C }").unwrap();
let layout  = layout_graph_diagram(&diagram, None);
let scene   = diagram_to_paint(&layout, None);

// scene is ready for paint-metal::render(&scene)
println!("Scene: {}×{} with {} instructions",
    scene.width, scene.height, scene.instructions.len());
```

## What each element produces

| Element          | PaintInstruction(s)                              |
|------------------|--------------------------------------------------|
| Rect node        | `PaintRect` (corner_radius = 0)                  |
| RoundedRect node | `PaintRect` (corner_radius from style)           |
| Ellipse node     | `PaintEllipse`                                   |
| Diamond node     | `PaintPath` (4-vertex diamond polygon)           |
| Node label       | `PaintGlyphRun` (coretext: scheme)               |
| Edge             | `PaintPath` (polyline)                           |
| Arrowhead        | `PaintPath` (filled triangle)                    |
| Edge label       | `PaintGlyphRun`                                  |
| Diagram title    | `PaintGlyphRun`                                  |

## Font scheme

All text uses `coretext:<PostScript-name>@<size>` — the font reference scheme
recognised by `paint-metal`'s CoreText glyph-run overlay. Override the PostScript
name via `DiagramToPaintOptions::ps_font_name`.

## Spec

[DG01 — Rust DOT Diagram Pipeline](../../../specs/DG01-dot-pipeline-rust.md)
