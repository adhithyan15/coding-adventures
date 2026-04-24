# DG02 — Diagram Layout Graph: Rank-Based Graph Layout

## Overview

`diagram-layout-graph` takes a `GraphDiagram` (DG00) and assigns pixel
coordinates to every node and edge, producing a `LayoutedGraphDiagram` (DG00).

```
GraphDiagram  (topology only)
  → diagram-layout-graph
  → LayoutedGraphDiagram  (topology + pixel positions)
```

The algorithm is a simplified **rank-based layout** inspired by the Sugiyama
framework. It handles the common cases well — linear pipelines, trees,
simple cycles — without the full complexity of proper Sugiyama crossing
minimisation.

---

## 1. Algorithm Overview

```
1. Topological sort → assign rank to each node
2. Place nodes at rank × step offset
3. Route edges as polylines
4. Compute canvas dimensions
```

### 1.1 Rank assignment

Each node is assigned a rank — an integer representing its "layer" in the
direction of the layout axis.

- A node with no predecessors gets rank 0.
- A node's rank is `max(predecessor ranks) + 1`.

The `directed-graph` crate provides topological sort. If the graph has cycles,
all nodes are placed in individual ranks (one per node) as a safe fallback.

### 1.2 Node placement

Nodes at the same rank are stacked perpendicular to the layout axis.

For `direction = "lr"` or `"rl"`:

```
major axis = x  (column = rank × (max_node_width + rank_gap))
minor axis = y  (row    = item_index × (node_height + node_gap))
```

For `direction = "tb"` or `"bt"`:

```
major axis = y  (row    = rank × (node_height + rank_gap))
minor axis = x  (column = item_index × (node_width + node_gap))
```

For `"rl"` and `"bt"`, the rank order is reversed so that higher-rank nodes
appear on the left / bottom respectively.

### 1.3 Node sizing

When a `TextMeasurer` is supplied, node width is measured via real font metrics:

```
measured = measurer.measure(label, font_spec, max_width = None)
width = max(min_node_width, horizontal_padding * 2 + measured.width)
```

When no `TextMeasurer` is supplied (e.g. in tests without a font stack), the
heuristic fallback is used:

```
width = max(min_node_width, horizontal_padding * 2 + text_length * char_width)
```

All nodes in the same rank take the width of the widest node in that rank.
Node height is fixed at `node_height`.

### 1.4 Edge routing

Edges are encoded as two-point polylines (straight lines), except self-loops
which use a five-point waypoint path:

```
Self-loop on node N:
  point 0:  right edge midpoint of N
  point 1:  right edge midpoint + (28, 0)
  point 2:  right edge + (28, -28)
  point 3:  top edge midpoint + (0, -28)
  point 4:  top edge midpoint of N
```

Edge endpoints are determined by `direction`:

| Direction | Start point | End point |
|-----------|-------------|-----------|
| `lr` | right midpoint of `from` | left midpoint of `to` |
| `rl` | left midpoint of `from` | right midpoint of `to` |
| `tb` | bottom midpoint of `from` | top midpoint of `to` |
| `bt` | top midpoint of `from` | bottom midpoint of `to` |

Edge label position is the midpoint between start and end, shifted up by 8px.

---

## 2. Layout Options

All options have defaults and can be overridden:

| Option | Default | Description |
|--------|---------|-------------|
| `margin` | 24 | Outer margin in pixels |
| `rank_gap` | 96 | Gap between ranks (pixels) |
| `node_gap` | 56 | Gap between nodes within a rank (pixels) |
| `title_gap` | 48 | Extra top inset when title is present |
| `min_node_width` | 96 | Minimum node width in pixels |
| `node_height` | 52 | Node height in pixels (fixed) |
| `horizontal_padding` | 24 | Left+right padding inside a node |
| `char_width` | 8 | Approximate width of one character in pixels |

---

## 3. Public API

### `layout_graph_diagram(diagram, options?, measurer?) → LayoutedGraphDiagram`

Takes a `GraphDiagram` and returns a fully laid-out `LayoutedGraphDiagram`.
All style fields are resolved to `ResolvedDiagramStyle` (no `Option`).

The optional `measurer: Option<&dyn TextMeasurer>` (from `layout-ir`) enables
real glyph-advance–based node sizing. When `None`, the heuristic char-width
fallback is used. The measurer is called with the node label string and a
`FontSpec` matching the label font configured in `diagram-to-paint`
(default: Helvetica 14 pt).

Callers on Apple platforms should inject a `NativeMeasurer` from
`layout-text-measure-native`. Callers in test code may pass `None`.

---

## 4. Limitations

- No crossing minimisation — edges may cross in complex graphs.
- No curve routing — edges are straight lines (or the self-loop approximation).
- Fixed node height — text wrapping is not supported.
- No label overlap avoidance.
- Cycles are handled by a simple fallback (one node per rank), not by
  heuristic edge reversal.

These limitations are acceptable for the typical diagrams produced from DOT
files. Proper Sugiyama layout or force-directed layout can be added later.

---

## 5. Implementation Notes

- Uses `directed_graph::Graph` for topological sort and predecessor queries.
- The layout function is pure — no I/O, no global state, all inputs via args.
- Canvas dimensions are computed from the bounding box of all placed nodes
  plus the outer margin: `max_x + margin` × `max_y + margin`.
