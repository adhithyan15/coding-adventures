# lattice-lexer

Tokenizer for the Lattice CSS superset language — the first stage of the
Lattice compiler pipeline.

## What is Lattice?

Lattice is a CSS superset (similar to Sass/SCSS) that adds:
- **Variables**: `$primary: #4a90d9;`
- **Mixins**: `@mixin flex-center() { ... }` / `@include flex-center;`
- **Control flow**: `@if`, `@for`, `@each`
- **Functions**: `@function spacing($n) { @return $n * 8px; }`
- **Modules**: `@use "colors";`

## Where it fits in the pipeline

```
Lattice source text
      |
      v
lattice-lexer  ──→  Vec<Token>
      |              [VARIABLE("$primary"), COLON(":"), HASH("#4a90d9"), ...]
      v
lattice-parser  ──→  GrammarASTNode (AST tree)
      v
lattice-ast-to-css  ──→  GrammarASTNode (CSS-only AST)
      v
lattice-transpiler  ──→  CSS text
```

## Usage

```rust
use coding_adventures_lattice_lexer::{tokenize_lattice, create_lattice_lexer};

// Tokenize all at once
let tokens = tokenize_lattice("$color: red; h1 { color: $color; }");

// Or use the lexer directly
let mut lexer = create_lattice_lexer("$color: red;");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Key token types

| Token type      | Example          | Description                              |
|-----------------|------------------|------------------------------------------|
| `VARIABLE`      | `$primary`       | Variable reference (starts with `$`)     |
| `AT_KEYWORD`    | `@mixin`, `@if`  | At-keyword (starts with `@`)             |
| `FUNCTION`      | `rgb(`           | Function name (IDENT immediately before `(`) |
| `IDENT`         | `color`, `red`   | Identifier                               |
| `HASH`          | `#4a90d9`        | Hash token (hex colors, id selectors)    |
| `DIMENSION`     | `16px`, `1.5em`  | Number with unit                         |
| `PERCENTAGE`    | `50%`            | Number with `%`                          |
| `CUSTOM_PROPERTY` | `--color`      | CSS custom property name                 |
| `EQUALS_EQUALS` | `==`             | Equality comparison                      |
| `NOT_EQUALS`    | `!=`             | Inequality comparison                    |

## Grammar

Token rules live in `code/grammars/lattice.tokens`. The lexer reads this
file at runtime and uses the generic `GrammarLexer` from the `lexer` crate
to tokenize source text.

## Dependencies

- `grammar-tools` — grammar file parser (`parse_token_grammar`)
- `lexer` — generic `GrammarLexer` and `Token` types

## Development

```bash
cargo test -p coding-adventures-lattice-lexer
```
