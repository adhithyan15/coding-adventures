# diagram-layout-grid

Deterministic backend-neutral grid layout for Mermaid block diagrams. It lowers
typed `GridDiagram` cells and connections into shared `LayoutedGraphDiagram`
geometry so every Paint VM backend uses the same rectangles, paths, and text.
