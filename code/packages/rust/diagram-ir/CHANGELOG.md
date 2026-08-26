# Changelog — diagram-ir

## 0.80.0

- Represent multi-task Gantt `after` starts and `until` ends as typed dependency lists.

## 0.79.0

- Preserve explicit Gantt end dates and resolved top/bottom axis and styled today-marker geometry.

## 0.78.0

- Preserve Mermaid Gantt calendar exclusions, includes, weekend boundaries, and axis controls in semantic temporal IR.

## 0.68.0

- Preserve Mermaid XY x/y-axis label, title, and spine visibility, typography, padding, and width configuration.

## 0.67.0

- Preserve authored Mermaid XY-chart dimensions, title visibility and typography, bar data-label placement, and data-label color in semantic chart IR.

## 0.66.0

- Preserve optional Mermaid XY-chart labels with each semantic series data point and add resolved point-label geometry.

## 0.63.0

- Preserve resolved GitGraph commit IDs and parent topology and generalize cross-branch history arcs.

## 0.62.0

- Resolve temporal branch lanes to backend-neutral endpoints and label bounds for directional GitGraph layout.

## 0.61.0

- Add backend-neutral GitGraph commit symbols for normal, reverse, highlight, merge, and cherry-pick nodes.

## 0.60.0

- Preserve every ordered GitGraph tag on commits, merges, cherry-picks, and temporal commit nodes.

## 0.59.0

- Preserve GitGraph titles and accessibility metadata and add a backend-neutral temporal title layout item.

## 0.58.0

- Add semantic and resolved styles to structural nodes.

## 0.57.0

- Preserve structural diagram accessibility title and description metadata.

## 0.56.0

- Preserve the six SysML Requirement definition kinds as typed metadata.

## 0.55.0

- Add typed Requirement and element metadata to structural nodes.

## 0.54.0

- Preserve optional layout direction for structural diagrams.

## 0.53.0

- Add Requirement structural diagram and node kinds.

## 0.52.0

- Carry resolved Journey activity spines, task descenders, and score positions.

## 0.51.0

- Preserve Journey legend offset and label-width controls plus resolved actor bounds.

## 0.50.0

- Preserve Journey actor and section palettes through semantic and resolved IR.
- Box the expanded Journey temporal body to keep the shared enum compact.

## 0.49.0

- Preserve Journey title font size, family, and color through semantic and resolved IR.

## 0.48.0

- Preserve configured Journey task font size and family through semantic and resolved IR.

## 0.47.0

- Preserve Journey geometry configuration in semantic IR.

## 0.46.0

- Carry resolved Journey actor colors through temporal layout items.

## 0.45.0

- Preserve Journey accessibility metadata through semantic and resolved temporal IR.

## 0.44.0

- Add typed Journey sections, scored tasks, actors, and resolved layout items.

## 0.43.0

- Preserve all pinned quadrant theme-variable colors through semantic and layout IR.

## 0.42.0

- Preserve quadrant title, axis, region, and point-label typography and spacing through semantic and layout IR.

## 0.41.0

- Preserve quadrant padding and distinct internal/external border widths through semantic and layout IR.

## 0.40.0

- Preserve authored quadrant dimensions, axis positions, and default point radius in typed chart configuration.

## 0.39.0

- Preserve chart accessibility title and description through semantic and layout IR.

## 0.38.0

- Preserve Mermaid quadrant point radius, fill, stroke color, and stroke width through semantic and layout IR.

## 0.37.0

- Add semantic quadrant-chart labels and points plus backend-neutral layout regions and scatter points.

## 0.36.0

- Preserve optional and resolved graph text font families.

## 0.35.0

- Preserve optional and resolved graph text italic styling.

## 0.34.0

- Preserve optional and resolved graph text font weights.

## 0.33.0

- Preserve state empty-description visibility through graph semantic and layout IR.

## 0.32.0

- Preserve an optional requested graph canvas width through semantic and layout IR.

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
