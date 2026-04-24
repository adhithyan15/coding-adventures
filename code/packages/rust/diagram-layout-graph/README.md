# diagram-layout-graph

Topological rank assignment and absolute geometry layout for graph diagrams.

Takes a `GraphDiagram` from `diagram-ir` and produces a `LayoutedGraphDiagram`
with concrete bounding boxes and edge routes. Feeds directly into `diagram-to-paint`.

## Usage

```rust
use diagram_ir::{GraphDiagram, GraphNode, GraphEdge, DiagramDirection,
                  DiagramLabel, EdgeKind};
use diagram_layout_graph::layout_graph_diagram;

let diagram = GraphDiagram { /* ... */ };

// Pass None for options (defaults) and None for measurer (heuristic widths).
// On Apple platforms, pass Some(&NativeMeasurer::new()) for CoreText sizing.
let layout = layout_graph_diagram(&diagram, None, None);

for node in &layout.nodes {
    println!("Node {} at ({}, {}) size {}×{}",
        node.id, node.x, node.y, node.width, node.height);
}
```

## Node sizing

When a `TextMeasurer` is supplied (third argument), node widths are measured via
real font metrics:

```
width = max(min_node_width, h_padding × 2 + measured.width)
```

When `None` is passed, the heuristic fallback is used:

```
width = max(min_node_width, h_padding × 2 + label.len() × char_width)
```

On Apple platforms, inject `layout_text_measure_native::NativeMeasurer` for
CoreText-accurate sizing that matches the text shaping pipeline.

## Algorithm

1. Build a directed-graph from the edges.
2. Topological sort → rank layers (level 0 = roots, level N = leaves).
3. Assign bounding boxes based on rank and direction (`TB`/`LR`/`RL`/`BT`).
4. Route edges as 2-point polylines; self-loops as 5-point detours.
5. Compute edge label midpoints.

## Layout constants

| Constant        | Default |
|-----------------|---------|
| `margin`        | 24 px   |
| `rank_gap`      | 96 px   |
| `node_gap`      | 56 px   |
| `title_gap`     | 48 px   |
| `min_node_width`| 96 px   |
| `node_height`   | 52 px   |

## Spec

[DG01 — Rust DOT Diagram Pipeline](../../../specs/DG01-dot-pipeline-rust.md)
