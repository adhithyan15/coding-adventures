# Changelog — diagram-layout-graph

## 0.10.0

- Reserve line-aware node geometry for repeated state descriptions.

## 0.9.0

- Route graph edges to composite-group boundary geometry.

## 0.8.0

- Stack concurrent state regions and resolve horizontal group dividers.

## 0.7.0

- Resolve composite graph-group styles into deterministic layout geometry.

## 0.6.0

- Compute padded nested group bounds around composite-state members.

## 0.5.0

- Forward graph node links and tooltips for backend hit-test metadata.

## 0.4.0

- Forward graph accessibility metadata into layout IR for backend export.

## 0.3.0

- Reserve line-aware geometry for backend-neutral note nodes.

## 0.2.0

- Give backend-neutral bar nodes compact fork/join geometry.

## 0.1.1 — TextMeasurer injection for real node sizing

### Changed
- `layout_graph_diagram` signature: `(diagram, options, measurer: Option<&dyn TextMeasurer>)`.
  Callers on Apple platforms can pass a `NativeMeasurer` from `layout-text-measure-native` for
  accurate CoreText-based node widths. Passing `None` retains the previous `char_width` heuristic.
- `node_width` now accepts `Option<&dyn TextMeasurer>` and delegates to the measurer when present:
  `width = max(min_node_width, h_padding × 2 + measured.width)`. When `None`, falls back to
  `h_padding × 2 + label.len() × char_width` as before.
- Added `label_font_spec()` helper returning the canonical label font (Helvetica 14 pt weight 400)
  used when calling the measurer — matches `diagram-to-paint`'s `label_font` default.
- Added `layout-ir` dependency for `TextMeasurer` and `FontSpec`.

### Tests — 12 pass (unchanged)

All existing tests pass `None` for the measurer and therefore exercise the heuristic path.

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
