# lattice-parser

Parser for the Lattice CSS superset language — the second stage of the
Lattice compiler pipeline.

## What it does

Takes a Lattice source string, tokenizes it via `lattice-lexer`, and uses
the `lattice.grammar` grammar file with the generic `GrammarParser` to
produce a `GrammarASTNode` tree. The resulting AST is "mixed" — it contains
both CSS nodes and Lattice-specific nodes for the next stage to process.

## Where it fits in the pipeline

```
Lattice source text
      |
      v
lattice-lexer  ──→  Vec<Token>
      |
      v
lattice-parser  ──→  GrammarASTNode (mixed CSS + Lattice AST)
      |
      stylesheet
        rule
          lattice_rule
            variable_declaration
              VARIABLE("$primary")
              COLON(":")
              value_list
                value
                  HASH("#4a90d9")
              SEMICOLON(";")
      v
lattice-ast-to-css  ──→  GrammarASTNode (CSS-only AST)
      v
lattice-transpiler  ──→  CSS text
```

## Usage

```rust
use coding_adventures_lattice_parser::{parse_lattice, create_lattice_parser};
use coding_adventures_lattice_parser::ASTNodeOrToken;

// Parse all at once
let ast = parse_lattice("$color: red; h1 { color: $color; }");
assert_eq!(ast.rule_name, "stylesheet");

// Or use the parser directly
let mut parser = create_lattice_parser("h1 { color: red; }");
let ast = parser.parse().expect("parse failed");
```

## Grammar rules

The grammar covers the full Lattice language:

| Rule                   | Example                                      |
|------------------------|----------------------------------------------|
| `variable_declaration` | `$color: red;`                               |
| `mixin_definition`     | `@mixin flex-center() { ... }`               |
| `include_directive`    | `@include flex-center;`                      |
| `if_directive`         | `@if $x == 1 { ... } @else { ... }`          |
| `for_directive`        | `@for $i from 1 through 3 { ... }`           |
| `each_directive`       | `@each $c in red, blue { ... }`              |
| `function_definition`  | `@function double($n) { @return $n * 2; }`   |
| `return_directive`     | `@return $n * 2;`                            |
| `use_directive`        | `@use "colors";`                             |
| `qualified_rule`       | `h1 { color: red; }`                         |
| `at_rule`              | `@media screen { ... }`                      |

## Grammar file

Rules live in `code/grammars/lattice.grammar`. The parser reads this file
at runtime using `env!("CARGO_MANIFEST_DIR")` to find the project root.

## Dependencies

- `coding-adventures-lattice-lexer` — tokenizes the source
- `grammar-tools` — grammar file parser (`parse_parser_grammar`)
- `parser` — generic `GrammarParser`, `GrammarASTNode`, `ASTNodeOrToken`

## Development

```bash
cargo test -p coding-adventures-lattice-parser
```
