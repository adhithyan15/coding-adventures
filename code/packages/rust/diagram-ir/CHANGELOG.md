# Changelog — diagram-ir

## 0.1.0

Initial release.

- `GraphDiagram`, `GraphNode`, `GraphEdge`, `EdgeKind` — pre-layout semantic IR
- `DiagramDirection` (`Tb`, `Lr`, `Rl`, `Bt`)
- `DiagramShape` (`Rect`, `RoundedRect`, `Ellipse`, `Diamond`)
- `DiagramLabel`, `DiagramStyle`, `ResolvedDiagramStyle`
- `resolve_style` / `resolve_style_with_base` — apply defaults
- `LayoutedGraphDiagram`, `LayoutedGraphNode`, `LayoutedGraphEdge`, `Point` — post-layout IR
