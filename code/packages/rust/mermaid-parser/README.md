# mermaid-parser

Versioned Mermaid compatibility dispatcher and grammar-driven Rust parsers.

Compatibility is pinned to Mermaid 11.16.1 in
`code/grammars/mermaid/compatibility.json`. The manifest distinguishes family
detection from syntax and native-render compatibility so progress can be
measured without treating a recognized header as a completed implementation.

The current native pipeline supports documented subsets of:

- `flowchart` / `graph`
- `classDiagram`
- `gantt`
- `pie`
- `xychart`

Each supported family lowers into the shared Diagram IR and can continue
through its family layout package:

```text
Mermaid
  -> family grammar and parser
  -> Diagram IR
  -> family layout
  -> diagram-to-paint
  -> PaintScene
  -> Metal / SVG / Direct2D / other Paint VM backends
```

All other Mermaid 11.16.1 family headers are recognized and return an explicit
`recognized but not implemented` error until their grammar, lowering, layout,
and native render fixtures are complete.
