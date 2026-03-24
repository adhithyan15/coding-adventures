# Lattice AST-to-CSS Compiler (Python)

**The core of the Lattice compiler** -- transforms a Lattice AST into plain CSS
text by expanding variables, mixins, functions, and control flow.

## What This Package Does

After the Lattice parser produces an AST, this package does the heavy lifting:
it walks the tree, resolves every Lattice construct, and produces a clean CSS
AST that can be emitted as CSS text. This is where variables get substituted,
mixins get inlined, `@if`/`@for`/`@each` get unrolled, and functions get
evaluated.

The package is split into five focused modules:

| Module | Responsibility |
|--------|---------------|
| `errors` | 10 structured error types with line/column info |
| `scope` | Lexical scope chain for variable/mixin/function lookup |
| `evaluator` | Compile-time expression evaluator for `@if`/`@for`/`@return` |
| `transformer` | Three-pass AST transformer (the core) |
| `emitter` | CSS text emitter with pretty-print and minified modes |

## How It Fits in the Stack

```
Lattice AST (from lattice-parser)
       |
       v
  +-----------+
  | Pass 1:   |  Collect variable, mixin, and function definitions
  | Symbols   |  into registries. Remove definition nodes from AST.
  +-----------+
       |
       v
  +-----------+
  | Pass 2:   |  Expand $variables, @include, @if/@for/@each,
  | Expansion |  function calls. Uses scope chain + evaluator.
  +-----------+
       |
       v
  +-----------+
  | Pass 3:   |  Remove empty blocks/rules left over from
  | Cleanup   |  expansion.
  +-----------+
       |
       v
  Clean CSS AST
       |
       v
  +-----------+
  | Emitter   |  Walk CSS AST, produce formatted text.
  +-----------+
       |
       v
  CSS Text
```

## Scope Chain

Lattice uses lexical (static) scoping. Each `{ }` block creates a child scope.
Variable lookup walks up the parent chain:

```
$color: red;              <-- global scope
.parent {
    $color: blue;         <-- shadows global
    color: $color;        --> blue
    .child {
        color: $color;    --> blue (inherited)
    }
}
.sibling {
    color: $color;        --> red (global, not affected by .parent)
}
```

Mixin expansion creates a child scope of the caller's scope. Function
evaluation creates an isolated scope parented to the global scope (preventing
accidental reliance on call-site variables).

## Expression Evaluator

The evaluator handles compile-time arithmetic and comparisons in `@if`, `@for`,
and `@return` contexts. It works with Lattice value types:

| Type | Example | Notes |
|------|---------|-------|
| LatticeNumber | `42`, `3.14` | Pure numbers |
| LatticeDimension | `16px`, `2em` | Number + CSS unit |
| LatticePercentage | `50%` | Number + percent |
| LatticeString | `"hello"` | Quoted strings |
| LatticeIdent | `red`, `bold` | Unquoted identifiers |
| LatticeColor | `#4a90d9` | Hash colors |
| LatticeBool | `true`, `false` | Boolean values |
| LatticeNull | (no literal) | Falsy null value |
| LatticeList | `1, 2, 3` | Comma-separated lists |

Operator precedence (tightest to loosest): unary `-`, `*`, `+`/`-`,
comparisons (`==`, `!=`, `>`, `>=`, `<=`), `and`, `or`.

## Error Handling

All errors inherit from `LatticeError` and carry line/column information:

```python
from lattice_ast_to_css import LatticeError

try:
    transformer.transform(ast)
except LatticeError as e:
    print(f"Error at line {e.line}, column {e.column}: {e}")
```

Error types include `UndefinedVariableError`, `UndefinedMixinError`,
`UndefinedFunctionError`, `WrongArityError`, `CircularReferenceError`,
`TypeErrorInExpression`, `UnitMismatchError`, and `MissingReturnError`.

## Usage

```python
from lattice_ast_to_css import LatticeTransformer, CSSEmitter
from lattice_parser import parse_lattice

# Parse
ast = parse_lattice("""
    $primary: #4a90d9;
    @mixin rounded($radius) {
        border-radius: $radius;
    }
    .card {
        color: $primary;
        @include rounded(8px);
    }
""")

# Transform (expand Lattice constructs)
transformer = LatticeTransformer()
css_ast = transformer.transform(ast)

# Emit CSS text
emitter = CSSEmitter(indent="  ")
print(emitter.emit(css_ast))
# .card {
#   color: #4a90d9;
#   border-radius: 8px;
# }
```

### Minified Output

```python
emitter = CSSEmitter(minified=True)
print(emitter.emit(css_ast))
# .card{color:#4a90d9;border-radius:8px}
```

## Installation

```bash
pip install coding-adventures-lattice-ast-to-css
```

## Dependencies

- `coding-adventures-lexer` -- provides the `Token` type
- `coding-adventures-parser` -- provides the `ASTNode` type
- `coding-adventures-lattice-lexer` -- Lattice token definitions
- `coding-adventures-lattice-parser` -- Lattice AST structure
