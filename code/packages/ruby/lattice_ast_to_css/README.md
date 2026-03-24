# lattice-ast-to-css

Three-pass compiler that transforms a Lattice AST (from `lattice_parser`)
into a clean CSS AST ready for text emission. Supports Lattice v1 and v2.

## Where It Fits

```
Lattice source text
        |
  lattice_lexer
        | tokens
  lattice_parser
        | Lattice AST
  lattice_ast_to_css      <- this package
        | CSS AST
  lattice_transpiler
        | CSS text
```

## Three-Pass Architecture

**Pass 1 -- Symbol Collection** reads the AST to register all top-level
`@mixin`, `@function`, and `$variable` declarations into symbol tables
before expansion begins. This enables forward references (using a mixin
defined later in the file). v2 adds `!default` and `!global` flag handling.

**Pass 2 -- Expansion** walks the AST and:
- Substitutes `$variable` references with their bound values.
- Expands `@include` calls by cloning and evaluating mixin bodies.
- Evaluates `@if`/`@for`/`@each`/`@while` at compile time.
- Calls `@function` bodies and substitutes the `@return` value.
- Handles `@content` blocks inside mixins.
- Resolves `$var` in selector positions.
- Expands property nesting (`font: { size: 14px; }`).
- Collects `@at-root` rules for hoisting.
- Records `@extend` relationships.
- Evaluates built-in functions (map, color, list, type, math).
- Strips `@mixin`, `@function`, `$variable`, and `@use` nodes from output.

**Pass 3 -- Cleanup** removes nil nodes, applies @extend selector merging,
splices @at-root hoisted rules, and removes placeholder-only rules.

## Value Types

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
| `LatticeMap`         | `(a: 1, b: 2)`  | (via fns)  |

## Built-in Functions (v2)

| Category | Functions |
|----------|-----------|
| Map      | `map-get`, `map-keys`, `map-values`, `map-has-key`, `map-merge`, `map-remove` |
| Color    | `lighten`, `darken`, `saturate`, `desaturate`, `adjust-hue`, `complement`, `mix`, `rgba`, `red`, `green`, `blue`, `hue`, `saturation`, `lightness` |
| List     | `nth`, `length`, `join`, `append`, `index` |
| Type     | `type-of`, `unit`, `unitless`, `comparable` |
| Math     | `math.div`, `math.floor`, `math.ceil`, `math.round`, `math.abs`, `math.min`, `math.max` |

## Error Types

| Error                               | Raised when                        |
|-------------------------------------|------------------------------------|
| `LatticeUndefinedVariableError`     | `$x` used but not declared         |
| `LatticeUndefinedMixinError`        | `@include` of unknown mixin        |
| `LatticeUndefinedFunctionError`     | call to undeclared function        |
| `LatticeWrongArityError`            | wrong number of args               |
| `LatticeCircularReferenceError`     | mixin/function calls itself        |
| `LatticeMissingReturnError`         | function has no `@return`          |
| `LatticeTypeErrorInExpression`      | incompatible operand types         |
| `LatticeUnitMismatchError`          | `px + em` without conversion       |
| `LatticeMaxIterationError`          | `@while` exceeds iteration limit   |
| `LatticeExtendTargetNotFoundError`  | `@extend` target not in stylesheet |
| `LatticeRangeError`                 | value out of bounds (e.g., `nth`)  |
| `LatticeZeroDivisionError`          | `math.div` by zero                 |

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

- `coding_adventures_lattice_parser` -- AST input
- `coding_adventures_lattice_lexer` -- used for tokenizing expression context

## Development

```bash
bundle exec rake test
```
