# diagram-layout-structural

Layout engine for structural diagrams (DG04): class, ER, and C4 diagrams.

## Position in pipeline

```
StructuralDiagram (diagram-ir)
  → diagram-layout-structural
      → LayoutedStructuralDiagram (diagram-ir)
      → diagram-to-paint
      → PaintScene
```

## Usage

```rust
use diagram_layout_structural::layout_structural_diagram;
use mermaid_parser::parse_class_diagram;

let diagram = parse_class_diagram("classDiagram\n  class Foo").unwrap();
let layout  = layout_structural_diagram(&diagram);
// layout.nodes each carry (x, y, width, height, compartments)
```

## Algorithm

Nodes are placed in a 3-column grid with:
- Node width from longest label/entry text (approx 8 px/char)
- Node height from compartment entry count (20 px/row)
- Relationships routed to closest-side midpoints
