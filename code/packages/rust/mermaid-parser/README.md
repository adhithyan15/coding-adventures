# mermaid-parser

Grammar-driven Rust parser for the shared Mermaid flowchart grammar.

The parser reads `code/grammars/mermaid.tokens` and
`code/grammars/mermaid.grammar`, validates the source with the generic lexer
and parser infrastructure, then lowers the result into `diagram-ir::GraphDiagram`.

The first supported subset is intentionally small but useful:

- `flowchart` / `graph`
- directions: `TB`, `TD`, `BT`, `LR`, `RL`
- node declarations
- edge chains
- edge labels
- inline Mermaid node shapes: `[]`, `()`, `(())`, `{}`

That is enough to prove:

```text
Mermaid -> GraphDiagram -> diagram-layout-graph -> diagram-to-paint -> paint-metal -> PNG
```
