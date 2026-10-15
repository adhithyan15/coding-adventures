# diagram-layout-geometric

Canvas resolver for geometric diagrams (DG04). Elements carry absolute coordinates;
this engine only computes the canvas bounding box.

## Position in pipeline

```
GeometricDiagram (diagram-ir)
  → diagram-layout-geometric
      → LayoutedGeometricDiagram (diagram-ir)
      → diagram-to-paint
      → PaintScene
```

## Usage

```rust
use diagram_layout_geometric::layout_geometric_diagram;
use diagram_ir::{GeoElement, GeometricDiagram};

let diagram = GeometricDiagram {
    title: None, width: None, height: None,
    elements: vec![GeoElement::Box { id: "a".into(), x: 10.0, y: 10.0, w: 100.0, h: 50.0,
                                     corner_radius: 4.0, label: None, fill: None, stroke: None }],
};
let layout = layout_geometric_diagram(&diagram);
// layout.width and layout.height are auto-computed from elements
```

## Bounded Venn canvas configuration

Venn carries width, height, padding, `useMaxWidth` and `useDebugLayout` through
owned semantic/layout types and five `venn.*` scene metadata entries. Defaults
are 800/450/8/true/false. The two booleans are transport metadata in this partial
implementation. Source dimensions admit finite(0,65536], padding finite[0,65536];
an invalid field uses its own default. A closed single-line init object's direct
`venn` object replaces front-matter config as a whole. Nested objects, quoted
values and body/style text cannot supply its fields.

`diagram_layout_geometric::try_layout_venn` checks caller-owned configuration
before arithmetic and refuses nonfinite derived circle/label geometry with a
typed `VennLayoutError`. The single-argument `layout_venn` wrapper panics for
unsupported IR. Use the checked API for untrusted typed input. Placement clamps
effective padding to each canvas quarter while retaining authored admitted
padding in config/metadata. Supported placement arithmetic is unchanged.

Exact two-set/union/text geometry and complete default/custom PaintScene,
glyph vectors, ordered font queries, primitives, styles and metadata controls
are portable tests. Finite geometry does not establish label/stroke enclosure,
area-proportional overlap, theme/CSS parity, arbitrary font/style safety or a
renderer pixel-area/resource budget. See
[`CV04a-venn-config-main-reconciliation.md`](../../../specs/CV04a-venn-config-main-reconciliation.md).
Full final-source Miri is **DEFERRED/PENDING**, never passed; compiler delivery
and the full Closure parity/provenance objective remain separate gates.
