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
owned semantic/layout types and eight `venn.*` scene metadata entries. Defaults
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

## Bounded Venn theme transport

Venn accepts root `themeVariables` front matter or a direct `themeVariables`
object in a closed single-line init directive after the leading YAML header.
The selected init object replaces front matter wholesale, including an empty
object. Only direct mapping fields supply values; nested maps/arrays, scalar
sections, sequence items and header literals cannot supply a theme. Specific
`vennTitleTextColor`/`vennSetTextColor` precede generic `titleColor`/`textColor`;
a present invalid specific value uses the default rather than the generic.
Active matching quotes preserve embedded other quotes and font-family lists;
only escaped active quotes/backslashes decode. Malformed present values default.

Defaults are `sans-serif`, `#0f172a`, `#0f172a`. Configured family replaces caller
option families, retaining size, weight, italic and stretch. Authored label
`color` overrides the configured set color. The checked geometric layout carries
all eight config fields; scenes expose all eight `venn.*` metadata entries.
Existing canvas/scalar/derived bounds and unrelated diagram readers remain intact.
Portable tests compare complete parsed config, geometry, paint primitives,
glyph positions, drawing order and actual ordered resolver queries in16 themed
source-to-scene cells, retaining the previous16 full-scene cells. This establishes
bounded transport, not physical-font fitting, rasterization or full CSS parity.
Actual affected14-package strict lint and925 unfiltered tests/40targets pass
at source05e61228. Miri **DEFERRED/PENDING**, never passed; hosted checks, current
main reconciliation, full Closure parity/lineage and delivery remain OPEN.
