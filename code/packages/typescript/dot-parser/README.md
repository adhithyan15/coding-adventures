# @coding-adventures/dot-parser

Parser for a focused Graphviz DOT subset.

This package is intentionally grammar-driven. The shared syntax source of truth
lives in:

- `code/grammars/dot.tokens`
- `code/grammars/dot.grammar`

The TypeScript package is a thin wrapper around the generic lexer/parser stack,
plus DOT-to-diagram lowering.

The first implementation slice does not aim to cover all of DOT. It exercises a
real end-to-end diagram pipeline with a subset that is rich enough to be useful
and stable:

- `digraph`
- optional graph name
- quoted and plain identifiers
- node statements
- edge chains (`A -> B -> C`)
- attribute lists (`[label="x", shape=diamond]`)
- graph-level assignments such as `rankdir=LR`

Unsupported DOT features such as subgraphs, ports, HTML labels, and undirected
graphs can layer on top later.

## Running tests

```bash
npx vitest run --coverage
```
