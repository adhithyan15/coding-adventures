# Lattice Parser (Python)

Parses Lattice source text into ASTs using the grammar-driven parser -- a thin
wrapper that loads `lattice.grammar` and feeds it to the generic `GrammarParser`.

## What This Package Does

This is the second stage of the Lattice compiler pipeline. It takes Lattice
source text, tokenizes it with the Lattice lexer, then parses the token stream
using the EBNF rules defined in `lattice.grammar`. The result is a generic
`ASTNode` tree.

The AST contains both CSS nodes (`qualified_rule`, `declaration`,
`selector_list`) and Lattice-specific nodes (`variable_declaration`,
`mixin_definition`, `if_directive`, `for_directive`, `each_directive`,
`function_definition`, `include_directive`). The downstream AST-to-CSS
compiler expands the Lattice nodes into pure CSS.

## How It Fits in the Stack

```
lattice.tokens          lattice.grammar
    |                        |
    v                        v
lattice_lexer          grammar_tools
(tokenize_lattice)     (parse_parser_grammar)
    |                        |
    +----------+-------------+
               |
               v
        GrammarParser
               |
               v
    lattice_parser.parse_lattice()
               |
               v
         ASTNode tree
               |
               v
    lattice-ast-to-css (next stage)
```

## Usage

```python
from lattice_parser import parse_lattice

ast = parse_lattice("$color: red; h1 { color: $color; }")
print(ast.rule_name)  # "stylesheet"
```

### Lattice Features

```python
from lattice_parser import parse_lattice

ast = parse_lattice("""
    $primary: #4a90d9;

    @mixin button($bg) {
        background: $bg;
        padding: 8px 16px;
    }

    .btn {
        @include button($primary);
        color: white;
    }
""")
```

### Lower-Level Access

```python
from lattice_parser import create_lattice_parser

parser = create_lattice_parser("h1 { color: red; }")
ast = parser.parse()
```

## Installation

```bash
pip install coding-adventures-lattice-parser
```

## Dependencies

- `coding-adventures-lattice-lexer` -- tokenizes Lattice source text
- `coding-adventures-grammar-tools` -- parses the `.grammar` file
- `coding-adventures-lexer` -- provides the `Token` type
- `coding-adventures-parser` -- provides the `GrammarParser` engine and `ASTNode` type
