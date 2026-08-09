# Changelog — diagram-ir

## 0.9.0

- Added normal and reverse filled/stick half-arrow semantics for sequence messages.

## 0.8.0

- Added Mermaid sequence participant stereotype kinds.

## 0.7.0

- Added semantic and layout IR primitives for sequence participant groups.

## 0.6.0

- Added sequence participant creation/destruction events and destruction geometry.

## 0.5.0

- Added nested sequence block start, branch, and end events.
- Added layouted sequence frames and branch dividers.

## 0.4.0

- Added semantic sequence participants, messages, notes, and activation events.
- Added layouted participant boxes, lifelines, message routes, notes, and activation bars.

## 0.1.0

Initial release.

- `GraphDiagram`, `GraphNode`, `GraphEdge`, `EdgeKind` — pre-layout semantic IR
- `DiagramDirection` (`Tb`, `Lr`, `Rl`, `Bt`)
- `DiagramShape` (`Rect`, `RoundedRect`, `Ellipse`, `Diamond`)
- `DiagramLabel`, `DiagramStyle`, `ResolvedDiagramStyle`
- `resolve_style` / `resolve_style_with_base` — apply defaults
- `LayoutedGraphDiagram`, `LayoutedGraphNode`, `LayoutedGraphEdge`, `Point` — post-layout IR
