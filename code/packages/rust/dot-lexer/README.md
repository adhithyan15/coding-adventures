# dot-lexer

Tokeniser for the [DOT graph description language](https://graphviz.org/doc/info/lang.html).

Transforms a DOT source string into a flat `Vec<Token>`. Whitespace, line
comments (`// …`), and block comments (`/* … */`) are consumed and discarded.

## Usage

```rust
use dot_lexer::{tokenise, TokenKind};

let result = tokenise(r#"
    digraph G {
        A -> B [label = "edge"]
    }
"#);

assert!(result.errors.is_empty());
assert_eq!(result.tokens[0].kind, TokenKind::Digraph);
```

## Token kinds

| Kind        | Example          | Notes                         |
|-------------|------------------|-------------------------------|
| `Strict`    | `strict`         | Case-insensitive keyword       |
| `Graph`     | `graph`          |                               |
| `Digraph`   | `digraph`        |                               |
| `Node`      | `node`           |                               |
| `Edge`      | `edge`           |                               |
| `Subgraph`  | `subgraph`       |                               |
| `LBrace`    | `{`              |                               |
| `RBrace`    | `}`              |                               |
| `LBracket`  | `[`              |                               |
| `RBracket`  | `]`              |                               |
| `Equals`    | `=`              |                               |
| `Semicolon` | `;`              |                               |
| `Comma`     | `,`              |                               |
| `Colon`     | `:`              |                               |
| `Arrow`     | `->`             | Directed edge operator        |
| `DashDash`  | `--`             | Undirected edge operator      |
| `Id`        | `foo`, `"bar"`, `3.14`, `<html>` | All ID flavours |
| `Eof`       | —                | Always last in the stream     |

## Spec

[DG01 — Rust DOT Diagram Pipeline](../../../specs/DG01-dot-pipeline-rust.md)
