# diagram-layout-temporal

Layout engine for temporal diagrams (DG04): Gantt charts and git-graphs.

## Position in pipeline

```
TemporalDiagram (diagram-ir)
  → diagram-layout-temporal
      → LayoutedTemporalDiagram (diagram-ir)
      → diagram-to-paint
      → PaintScene
```

## Usage

```rust
use diagram_layout_temporal::layout_temporal_diagram;
use mermaid_parser::parse_gantt;
use diagram_ir::{TemporalDiagram, TemporalKind, TemporalBody};

let gantt    = parse_gantt("gantt\n  dateFormat YYYY-MM-DD\n  section A\n    Task :done, t1, 2026-01-01, 5d").unwrap();
let temporal = TemporalDiagram { kind: TemporalKind::Gantt, title: None, body: TemporalBody::Gantt(gantt) };
let layout   = layout_temporal_diagram(&temporal, 800.0);
```

## Algorithms

**Gantt**: Two-pass date resolution (absolute `YYYY-MM-DD` first, then `after <id>` deps). Time axis scaled to canvas width.

**Git**: Branch lanes (horizontal rows), commits as circles on lanes, merges as bezier arcs.
