# lattice-ast-to-css

Three-pass compiler that transforms a Lattice AST (from `lattice_parser`)
into a clean CSS AST ready for text emission.

## Where It Fits

```
Lattice source text
        │
  lattice_lexer
        │ tokens
  lattice_parser
        │ Lattice AST
  lattice_ast_to_css      ← this package
        │ CSS AST
  lattice_transpiler
        │ CSS text
```

## Three-Pass Architecture

**Pass 1 — Symbol Collection** reads the AST to register all top-level
`@mixin`, `@function`, and `$variable` declarations into symbol tables
before expansion begins. This enables forward references (using a mixin
defined later in the file).

**Pass 2 — Expansion** walks the AST and:
- Substitutes `$variable` references with their bound values.
- Expands `@include` calls by cloning and evaluating mixin bodies.
- Evaluates `@if`/`@for`/`@each` at compile time and emits the selected nodes.
- Calls `@function` bodies and substitutes the `@return` value.
- Strips `@mixin`, `@function`, `$variable`, and `@use` nodes from output.

**Pass 3 — Cleanup** removes any nil nodes left by expansion.

## Value Types

Lattice expressions evaluate to typed values:

| Type                 | Example          | CSS output |
|----------------------|------------------|------------|
| `LatticeNumber`      | `42`, `3.14`     | `42`       |
| `LatticeDimension`   | `16px`, `2em`    | `16px`     |
| `LatticePercentage`  | `50%`            | `50%`      |
| `LatticeString`      | `"hello"`        | `hello`    |
| `LatticeIdent`       | `red`, `bold`    | `red`      |
| `LatticeColor`       | `#4a90d9`        | `#4a90d9`  |
| `LatticeBool`        | `true`, `false`  | `true`     |
| `LatticeNull`        | `null`           | (empty)    |
| `LatticeList`        | `red, blue`      | (iterable) |

## Error Types

| Error                          | Raised when                        |
|--------------------------------|------------------------------------|
| `LatticeUndefinedVariableError`| `$x` used but not declared         |
| `LatticeUndefinedMixinError`   | `@include` of unknown mixin        |
| `LatticeUndefinedFunctionError`| call to undeclared function        |
| `LatticeWrongArityError`       | wrong number of args               |
| `LatticeCircularReferenceError`| mixin/function calls itself        |
| `LatticeMissingReturnError`    | function has no `@return`          |
| `LatticeTypeErrorInExpression` | incompatible operand types         |
| `LatticeUnitMismatchError`     | `px + em` without conversion       |

## Usage

```ruby
require "coding_adventures_lattice_ast_to_css"
require "coding_adventures_lattice_parser"

ast     = CodingAdventures::LatticeParser.parse(source)
xformer = CodingAdventures::LatticeAstToCss::LatticeTransformer.new
css_ast = xformer.transform(ast)

emitter = CodingAdventures::LatticeAstToCss::CSSEmitter.new(indent: "  ")
css     = emitter.emit(css_ast)
```

## Dependencies

- `coding_adventures_lattice_parser` — AST input
- `coding_adventures_lattice_lexer` — used for tokenizing expression context

## Development

```bash
bundle exec rake test
```
