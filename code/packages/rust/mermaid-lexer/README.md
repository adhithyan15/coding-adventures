# mermaid-lexer

Grammar-driven Rust lexers for the shared Mermaid family grammars under
`code/grammars/mermaid/`.

The current entrypoints tokenize flowchart and Pie syntax for the native
diagram pipeline:

```text
Mermaid source
  -> mermaid-lexer
  -> mermaid-parser
  -> family Diagram IR
```

The flowchart lexer supports:

- `flowchart` / `graph`
- directions: `TB`, `TD`, `BT`, `LR`, `RL`
- node ids
- inline node shapes: `[rect]`, `(round)`, `((circle))`, `{diamond}`
- edge operators: `-->`, `---`
- edge labels: `|label|`
- statement separators: newline or `;`
- `%%` comments

The Pie lexer supports:

- `pie` and `showData`
- quoted labels
- numeric slice values
- `%%` comments
