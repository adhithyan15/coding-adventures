# diagram-ir

Semantic diagram intermediate representation (IR) shared across all diagram
pipelines in the coding-adventures monorepo (DG00).

## What it is

`diagram-ir` is the shared vocabulary between diagram parsers and layout engines.
Every diagram source format (DOT, Mermaid, PlantUML) produces a `GraphDiagram`.
Every layout engine (`diagram-layout-graph`, `diagram-layout-sequence`) consumes
a `GraphDiagram` and produces a `LayoutedGraphDiagram`.

```text
dot-parser / mermaid-parser
  → GraphDiagram          ← this crate
  → diagram-layout-graph
  → LayoutedGraphDiagram  ← this crate
  → diagram-to-paint
  → PaintScene
```

## Key types

| Type                     | Description                               |
|--------------------------|-------------------------------------------|
| `GraphDiagram`           | Pre-layout semantic graph (nodes + edges) |
| `GraphNode`              | Node with id, label, shape, optional style |
| `GraphEdge`              | Edge with source, target, kind, optional label |
| `DiagramDirection`       | `Tb / Lr / Rl / Bt` flow direction        |
| `DiagramShape`           | `Rect / RoundedRect / Ellipse / Diamond`  |
| `DiagramStyle`           | Optional style overrides (all `Option<_>`) |
| `ResolvedDiagramStyle`   | Fully-resolved style after applying defaults |
| `LayoutedGraphDiagram`   | Post-layout diagram with absolute geometry |
| `LayoutedGraphNode`      | Node with `x, y, width, height`           |
| `LayoutedGraphEdge`      | Edge with routed `Vec<Point>` polyline    |

## Usage

```rust
use diagram_ir::{GraphDiagram, GraphNode, GraphEdge, DiagramDirection,
                  DiagramLabel, EdgeKind, resolve_style};

let node = GraphNode {
    id: "start".to_string(),
    label: DiagramLabel::new("Start"),
    shape: None,
    style: None,
};
```

## Spec

See `code/specs/DG00-diagram-ir.md` for the full architecture and
`code/specs/DG01-dot-pipeline-rust.md` for the Rust-specific pipeline.


## Bounded spacing and transport reconciliation

MAX_SWIMLANE_FLOWCHART_SPACING defines the own IR production range 0..=65536 for nodeSpacing, rankSpacing and diagramPadding. This bounded policy is separate from unrestricted upstream parity.

Validation is source-attributed: Windows applicable packages and standalone
checks are recorded separately from Unix-only/hosted and whole-PR gates.
Final-source Miri is **DEFERRED/PENDING**, never passed.


## Global curves and checked ranks

Global Swimlane flowchart curve uses the existing own curve enum; absent configuration retains None and all previous defaults.

Affected Windows suites/strict lint are source-attributed to e2fd7e9c;
producer native macOS rendering, general floating work and whole-PR/hosted
gates remain separate. Final-source Miri is **DEFERRED/PENDING**.
