# Changelog — diagram-layout-graph

## 0.1.0

Initial release.

- `layout_graph_diagram(diagram, options) -> LayoutedGraphDiagram` — main entry point
- `GraphLayoutOptions` — tuning knobs for margin, gaps, node sizing
- Topological rank assignment via `directed-graph::Graph`
- Cycle detection with flat-layout fallback
- TB/LR/RL/BT direction support
- Self-loop routing (5-point detour above the node)
- Node width heuristic based on label length
- Edge label midpoint computation
- Default edge style: dark grey stroke, no fill
