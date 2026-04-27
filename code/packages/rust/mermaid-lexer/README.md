# mermaid-lexer

Grammar-driven Rust lexer for the shared `code/grammars/mermaid.tokens`
definition.

This package tokenizes a focused Mermaid flowchart subset that is meant to feed
the native diagram pipeline:

```text
Mermaid source
  -> mermaid-lexer
  -> mermaid-parser
  -> GraphDiagram
```

Supported lexical constructs in v1:

- `flowchart` / `graph`
- directions: `TB`, `TD`, `BT`, `LR`, `RL`
- node ids
- inline node shapes: `[rect]`, `(round)`, `((circle))`, `{diamond}`
- edge operators: `-->`, `---`
- edge labels: `|label|`
- statement separators: newline or `;`
- `%%` comments
