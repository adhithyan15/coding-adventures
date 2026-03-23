# Lattice Transpiler (Python)

**Full pipeline from Lattice source to CSS output** -- a single
`transpile_lattice()` function that wires together the lexer, parser,
transformer, and emitter.

## What This Package Does

This is the top-level entry point for the Lattice transpiler. It is intentionally
thin: one function that pipelines three independent packages in sequence.

```
Lattice Source
     |
     v
+--------------+
| Lattice Lexer|  <-- lattice.tokens
+------+-------+
       | tokens
       v
+--------------+
|Lattice Parser|  <-- lattice.grammar
+------+-------+
       | AST (CSS + Lattice nodes)
       v
+--------------+
| Transformer  |  <-- scope chain, evaluator
+------+-------+
       | AST (CSS nodes only)
       v
+--------------+
| CSS Emitter  |
+------+-------+
       |
       v
  CSS Text
```

For finer control over individual stages, use the underlying packages directly:
`lattice-lexer`, `lattice-parser`, and `lattice-ast-to-css`.

## Usage

```python
from lattice_transpiler import transpile_lattice

css = transpile_lattice("""
    $primary: #4a90d9;
    $padding: 8px;

    @mixin button($bg) {
        background: $bg;
        padding: $padding 16px;
        border-radius: 4px;
    }

    .btn-primary {
        @include button($primary);
        color: white;
    }

    .btn-danger {
        @include button(#d94a4a);
        color: white;
    }
""")

print(css)
# .btn-primary {
#   background: #4a90d9;
#   padding: 8px 16px;
#   border-radius: 4px;
#   color: white;
# }
# .btn-danger {
#   background: #d94a4a;
#   padding: 8px 16px;
#   border-radius: 4px;
#   color: white;
# }
```

### Minified Output

```python
css = transpile_lattice("h1 { color: red; }", minified=True)
# "h1{color:red}"
```

### Custom Indentation

```python
css = transpile_lattice("h1 { color: red; }", indent="    ")
# Uses 4-space indentation instead of the default 2-space
```

## Error Handling

```python
from lattice_transpiler import transpile_lattice
from lattice_ast_to_css import LatticeError

try:
    css = transpile_lattice("h1 { color: $undefined; }")
except LatticeError as e:
    print(f"Error at line {e.line}: {e}")
```

## Installation

```bash
pip install coding-adventures-lattice-transpiler
```

## Dependencies

- `coding-adventures-lattice-lexer` -- tokenizes Lattice source
- `coding-adventures-lattice-parser` -- parses tokens into an AST
- `coding-adventures-lattice-ast-to-css` -- transforms Lattice AST to CSS and emits text
- `coding-adventures-grammar-tools` -- parses `.tokens` and `.grammar` files
- `coding-adventures-lexer` -- provides the `Token` type
- `coding-adventures-parser` -- provides the `GrammarParser` and `ASTNode` types
