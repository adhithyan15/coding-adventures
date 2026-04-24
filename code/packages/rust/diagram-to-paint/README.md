# diagram-to-paint

DG03 — Converts a `LayoutedGraphDiagram` into a `PaintScene` for rendering by
`paint-metal` and other backends. Text rendering is **delegated to `layout-to-paint`**
so every paint backend receives real glyph IDs from the TXT00 shaping pipeline.

## Usage

```rust
use dot_parser::parse_to_diagram;
use diagram_layout_graph::layout_graph_diagram;
use diagram_to_paint::{DiagramToPaintOptions, diagram_to_paint};
use layout_ir::{Color, font_spec};
use text_native_coretext::{CoreTextResolver, CoreTextMetrics, CoreTextShaper};

let shaper   = CoreTextShaper::new();
let metrics  = CoreTextMetrics::new();
let resolver = CoreTextResolver::new();

let diagram = parse_to_diagram("digraph G { A -> B -> C }").unwrap();
let layout  = layout_graph_diagram(&diagram, None, None);

let opts = DiagramToPaintOptions {
    background:          Color { r: 255, g: 255, b: 255, a: 255 },
    device_pixel_ratio:  1.0,
    label_font:          font_spec("Helvetica", 14.0),
    title_font:          { let mut f = font_spec("Helvetica", 18.0); f.weight = 700; f },
    shaper:              &shaper,
    metrics:             &metrics,
    resolver:            &resolver,
};
let scene = diagram_to_paint(&layout, &opts);

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
| Node label       | `PaintGlyphRun` (real glyph IDs via TXT00)       |
| Edge             | `PaintPath` (polyline)                           |
| Arrowhead        | `PaintPath` (filled triangle)                    |
| Edge label       | `PaintGlyphRun` (real glyph IDs via TXT00)       |
| Diagram title    | `PaintGlyphRun` (real glyph IDs via TXT00)       |

## Painter's algorithm order

1. Edge lines and arrowheads (behind nodes).
2. Node shapes (filled, covering edge endpoints).
3. All text via `layout-to-paint` (on top).

## Text pipeline

All text is rendered by building a `PositionedNode` tree and calling
`layout-to-paint::layout_to_paint` **once** at the end. The shaper produces
real font-specific glyph IDs — not Unicode codepoints — so Metal, Direct2D,
SVG, and Canvas backends all produce correct, readable text.

## Spec

[DG03 — Diagram to Paint](../../../specs/DG03-diagram-to-paint.md)
