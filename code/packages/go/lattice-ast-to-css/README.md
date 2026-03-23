# lattice-ast-to-css

Three-pass compiler from Lattice AST to clean CSS AST, plus a CSS emitter that
serialises the result to a string.

## What Is Lattice?

Lattice is a CSS superset (similar to Sass/SCSS) that adds compile-time
variables, mixins, functions, control flow, and modules on top of standard CSS.
Because Lattice compiles entirely at build time, the output is plain CSS that
any browser understands — there is no runtime.

## Role in the Pipeline

```
Lattice source text
  ↓ lattice-lexer   — tokenise
  ↓ lattice-parser  — parse into Lattice AST
  ↓ this package    — compile Lattice AST → CSS AST → CSS text
CSS output text
```

This package is the core compiler. It consumes the parsed AST from
`lattice-parser` and produces either a clean CSS AST (for inspection) or a
final CSS string.

## Quick Start

```go
import latticeasttocss "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css"

// All-in-one: source → CSS text
css, err := latticeasttocss.TranspileLatticeFull(source, false, "  ")

// Convenience wrappers
css, err := latticeasttocss.TranspileLattice(source)           // pretty-print
css, err := latticeasttocss.TranspileLatticeMinified(source)   // compact

// Step-by-step (if you already have a parsed AST)
cssAST, err := latticeasttocss.TransformLatticeAST(ast)
css := latticeasttocss.EmitCSS(cssAST)
css := latticeasttocss.EmitCSSMinified(cssAST)
```

## Three-Pass Transformation

### Pass 1: Module Resolution

Walks top-level rules for `@use` directives, collects their exported symbols
into a module registry, then removes the `@use` nodes from the AST. Cycle
detection prevents infinite loops from circular `@use` chains.

### Pass 2: Symbol Collection

Collects all `variable_declaration`, `mixin_definition`, and
`function_definition` nodes into symbol tables. Removes these definition nodes
from the AST (they produce no CSS output).

### Pass 3: Expansion

Recursively walks remaining AST nodes with a `ScopeChain`:

- **Variable substitution** — `VARIABLE` tokens replaced with their bound values
- **`@include` expansion** — mixin body deep-cloned, parameters bound, expanded in child scope
- **Control flow** — `@if`/`@else if`/`@else`, `@for`, `@each` evaluated at compile time
- **Function calls** — user-defined `@function` bodies evaluated, `@return` value substituted
- **CSS pass-through** — unknown at-rules, plain declarations, selectors emitted unchanged

## Supported Lattice Features

### v1 Features

| Feature | Example |
|---------|---------|
| Variables | `$primary: #4a90d9;` / `color: $primary;` |
| Mixins | `@mixin btn($bg) { ... }` / `@include btn(red);` |
| Default params | `@mixin border($w: 1px) { ... }` |
| Functions | `@function double($n) { @return $n * 2; }` |
| `@if` / `@else` | `@if $size > 10 { ... } @else { ... }` |
| `@for` (in mixin) | `@for $i from 1 through 3 { ... }` |
| `@each` (in mixin) | `@each $c in red, blue { ... }` |
| Arithmetic | `$n * 2`, `10px + 5px`, `$x >= 0` |
| CSS passthrough | All standard CSS passes through unchanged |

### v2 Features

| Feature | Example |
|---------|---------|
| `@while` loops | `@while $i <= 10 { ... }` |
| `!default` flag | `$color: red !default;` |
| `!global` flag | `$theme: dark !global;` |
| `@content` blocks | `@include wrapper { ... }` + `@content;` |
| `@at-root` | `@at-root .selector { ... }` |
| `@extend` / `%placeholder` | `@extend %message-shared;` |
| Variables in selectors | `.col-$i { ... }` |
| `@each` over maps | `@each $k, $v in $map { ... }` |
| Maps (`LatticeMap`) | `$theme: (primary: #4a90d9, secondary: #7b68ee)` |
| 37 built-in functions | `lighten()`, `map-get()`, `math.div()`, etc. |

### Built-in Functions

**Map:** `map-get`, `map-keys`, `map-values`, `map-has-key`, `map-merge`, `map-remove`

**Color:** `lighten`, `darken`, `saturate`, `desaturate`, `adjust-hue`, `complement`, `mix`, `rgba`, `red`, `green`, `blue`, `hue`, `saturation`, `lightness`

**List:** `nth`, `length`, `join`, `append`, `index`

**Type:** `type-of`, `unit`, `unitless`, `comparable`

**Math:** `math.div`, `math.floor`, `math.ceil`, `math.round`, `math.abs`, `math.min`, `math.max`

**Not supported:**
- `#{}` string interpolation (requires modal lexing — by design)
- Division operator `/` (ambiguous with CSS font shorthand — use `math.div()`)
- `@use` module loading from disk (stub: symbols collected but no file I/O)

## Emitter

`CSSEmitter` walks a clean CSS AST and reconstructs the CSS text. Two modes:

- **Pretty-print** — 2-space indentation, newlines between rules (default)
- **Minified** — no extra whitespace, suitable for production

## Error Handling

All errors satisfy the standard `error` interface and can be inspected with
`errors.As`:

```go
css, err := latticeasttocss.TranspileLatticeFull(source, false, "  ")
var uve *latticeasttocss.UndefinedVariableError
if errors.As(err, &uve) {
    fmt.Printf("Undefined variable %s at line %d\n", uve.Name, uve.Line)
}
```

Error types:

| Type | When |
|------|------|
| `UndefinedVariableError` | Variable used but never declared |
| `UndefinedMixinError` | `@include` references unknown mixin |
| `UndefinedFunctionError` | Function call references unknown function |
| `WrongArityError` | Wrong number of arguments to mixin/function |
| `CircularReferenceError` | Mixin or function calls itself recursively |
| `TypeErrorInExpression` | Arithmetic on incompatible types |
| `UnitMismatchError` | Adding incompatible CSS units |
| `MissingReturnError` | `@function` body has no `@return` |
| `MaxIterationError` | `@while` loop exceeds iteration limit (v2) |
| `ExtendTargetNotFoundError` | `@extend` references unknown selector (v2) |
| `RangeError` | Built-in function argument out of range (v2) |
| `ZeroDivisionInExpressionError` | `math.div()` divides by zero (v2) |

## Scope Model

Variables are block-scoped with lexical lookup. Inner scopes shadow outer
scopes; sibling scopes are isolated.

```lattice
$color: red;          /* global */
.outer {
  $color: blue;       /* shadows global */
  color: $color;      /* → blue */
}
.sibling {
  color: $color;      /* → red (global, not affected by .outer) */
}
```

## Dependencies

- `lattice-parser` — provides the Lattice AST input
- `lattice-lexer` — token type definitions used during AST walking
- `parser` — shared `ASTNode` type
- `lexer` — shared `Token` type

## Development

```bash
cd code/packages/go/lattice-ast-to-css
go vet ./...
go test ./... -cover   # coverage target: >80%
```
