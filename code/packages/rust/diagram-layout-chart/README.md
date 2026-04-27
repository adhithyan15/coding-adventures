# diagram-layout-chart

Layout engine for chart-family diagrams (DG04): XY bar/line, pie, and Sankey.

## Position in pipeline

```
ChartDiagram (diagram-ir)
  → diagram-layout-chart
      → LayoutedChartDiagram (diagram-ir)
      → diagram-to-paint
      → PaintScene
```

## Usage

```rust
use diagram_layout_chart::layout_chart_diagram;
use diagram_ir::{ChartDiagram, ChartKind, ChartOrientation, ChartSeries, SeriesKind};

let diagram = ChartDiagram {
    title: Some("Revenue".into()),
    kind: ChartKind::Xy,
    series: vec![ChartSeries { kind: SeriesKind::Bar, label: None, data: vec![40.0, 60.0] }],
    // ...
};
let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
// layout.items contains AxisSpine, Bar, LinePath etc.
```

## Supported chart kinds

| Kind   | Description |
|--------|-------------|
| `Xy`   | Categorical x-axis with bar/line series |
| `Pie`  | Angular slices starting at 12 o'clock |
| `Sankey` | Left-to-right proportional bands |
