# Changelog — diagram-ir

## 0.31.0

- Preserve optional local direction on composite graph groups and layout groups.

## 0.30.0

- Preserve ordered concurrent-region membership and resolved group dividers.

## 0.29.0

- Preserve composite graph-group styles through semantic and layout IR.

## 0.28.0

- Add nested graph groups for composite state containment and backend-neutral outlines.

## 0.27.0

- Preserve graph node links and optional tooltips through semantic and layout IR.

## 0.26.0

- Preserve graph-family accessibility titles and descriptions through semantic and layout IR.

## 0.25.0

- Add backend-neutral note nodes and note-association edges for annotated diagrams.

## 0.24.0

- Add a backend-neutral bar node shape for state fork and join pseudostates.

## 0.23.0

- Distinguish mirrored footer participant boxes in sequence layout IR.

## 0.22.0

- Preserve ordered sequence autonumber visibility and counter changes as semantic events.

## 0.21.0

- Preserve participant-group label wrap intent and resolved label height in sequence IR.

## 0.20.0

- Preserve participant-label wrap intent and resolved label height in sequence IR.

## 0.19.0

- Preserve explicit wrap intent and line-aware layout geometry for sequence control-block labels.

## 0.18.0

- Preserve default, forced-wrap, and forced-no-wrap sequence text intent.

## 0.17.0

- Added explicit sequence message-label height to layout IR.

## 0.16.0

- Added sequence accessibility title and description semantics.

## 0.15.0

- Added host document details references to sequence participant and layout IR.

## 0.14.0

- Added arbitrary JSON-valued properties to sequence participants and layout IR.

## 0.13.0

- Added labeled sequence participant links to semantic and layout IR.

## 0.12.0

- Added optional sequence block fills for background highlights.

## 0.11.0

- Added decimal sequence-number start and increment values.

## 0.10.0

- Added source, destination, and dual central-connection semantics.

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
