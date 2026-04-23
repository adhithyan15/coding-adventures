# @coding-adventures/diagram-layout-graph

Simple graph layout for the shared diagram IR.

This package is intentionally small and deterministic. It is not trying to
reimplement Graphviz layout. The goal of the first slice is to place nodes and
route edges well enough to exercise the full DOT -> Diagram IR -> PaintScene
pipeline.

Current behavior:

- DAGs get rank-based layered layout
- cyclic graphs fall back to stable insertion-order layout
- nodes are sized from label length
- edges are routed as straight segments, with explicit self-loop geometry

## Running tests

```bash
npx vitest run --coverage
```
