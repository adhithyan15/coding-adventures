# @coding-adventures/diagram-ir

Shared diagram intermediate representation for the text-diagram pipeline.

This package provides the semantic and layouted graph types that sit between
source-specific frontends such as DOT or Mermaid and the generic paint layer.

For the first implementation slice, the package focuses on graph-style
diagrams:

- semantic graph diagrams: nodes, edges, labels, direction, style hints
- layouted graph diagrams: absolute node boxes and routed edge points

## Where it fits

```text
DOT / Mermaid / other source
  -> source parser
  -> graph diagram IR
  -> graph layout
  -> diagram-to-paint
  -> PaintScene
```

## Running tests

```bash
npx vitest run --coverage
```
