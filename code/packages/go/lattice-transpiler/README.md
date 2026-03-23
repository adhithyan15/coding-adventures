# lattice-transpiler

End-to-end Lattice source text to CSS text pipeline.

## What Is Lattice?

Lattice is a CSS superset language (similar to Sass/SCSS) that adds
compile-time variables, mixins, functions, control flow, and modules on top of
standard CSS. Because Lattice compiles entirely at build time, the output is
plain CSS that any browser understands — there is no runtime.

## Role in the Stack

This package is the single consumer-facing entry point. It wires together the
full pipeline:

```
Lattice source text (.lattice)
  ↓  lattice-lexer    — tokenise using lattice.tokens
  ↓  lattice-parser   — parse into Lattice AST using lattice.grammar
  ↓  lattice-ast-to-css — transform AST → CSS AST, then emit CSS text
CSS output text (.css)
```

If you only need part of the pipeline (e.g., you already have a parsed AST),
use the lower-level packages directly.

## Quick Start

```go
import latticetranspiler "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-transpiler"

// Pretty-print (2-space indent, newlines between rules)
css, err := latticetranspiler.Transpile(`
    $primary: #4a90d9;
    .btn { color: $primary; }
`)
// css = ".btn {\n  color: #4a90d9;\n}\n"

// Minified (no extra whitespace)
css, err := latticetranspiler.TranspileMinified("$x: 1px; .a { margin: $x; }")
// css = ".a{margin:1px;}"

// Custom options
css, err := latticetranspiler.TranspileWithOptions(source, latticetranspiler.Options{
    Indent: "\t",   // tab indentation
})

css, err := latticetranspiler.TranspileWithOptions(source, latticetranspiler.Options{
    Minify: true,   // compact output, Indent ignored
})
```

## API Reference

### `Transpile(source string) (string, error)`

Compiles Lattice source to pretty-printed CSS with 2-space indentation.
This is the most common entry point.

### `TranspileMinified(source string) (string, error)`

Compiles Lattice source to compact CSS with no extra whitespace. Suitable for
production deployments where file size matters.

### `TranspileWithOptions(source string, opts Options) (string, error)`

Compiles Lattice source with full control over output formatting.

### `Options`

```go
type Options struct {
    Minify bool    // produce compact CSS; when true, Indent is ignored
    Indent string  // indentation per level; "" defaults to "  " (2 spaces)
}
```

## Error Handling

All errors from the pipeline satisfy the standard `error` interface. Structured
Lattice errors can be extracted with `errors.As`:

```go
import latticeasttocss "github.com/adhithyan15/coding-adventures/code/packages/go/lattice-ast-to-css"

css, err := latticetranspiler.Transpile(source)
var uve *latticeasttocss.UndefinedVariableError
if errors.As(err, &uve) {
    fmt.Printf("Undefined variable %s at line %d\n", uve.Name, uve.Line)
}
```

Available error types (from `lattice-ast-to-css`):

| Type | When |
|------|------|
| `UndefinedVariableError` | Variable used but never declared |
| `UndefinedMixinError` | `@include` references unknown mixin |
| `UndefinedFunctionError` | Function call references unknown function |
| `WrongArityError` | Wrong number of arguments to mixin/function |
| `CircularReferenceError` | Mixin or function calls itself recursively |
| `TypeError` | Arithmetic on incompatible types |
| `MissingReturnError` | `@function` body has no `@return` |

## Supported Lattice Features

| Feature | Example |
|---------|---------|
| Variables | `$primary: #4a90d9;` / `color: $primary;` |
| Mixins | `@mixin btn($bg) { ... }` / `@include btn(red);` |
| Default params | `@mixin border($w: 1px) { ... }` |
| Functions | `@function double($n) { @return $n * 2; }` |
| `@if` / `@else if` / `@else` | Conditional rule inclusion |
| `@for` (inside mixin/function) | `@for $i from 1 through 3 { ... }` |
| `@each` (inside mixin/function) | `@each $c in red, blue { ... }` |
| Arithmetic | `$n * 2`, `10px + 5px`, `$x >= 0` |
| CSS passthrough | All standard CSS passes through unchanged |

**Not supported (by design):**
- `#{}` string interpolation — requires modal lexing (context-sensitive grammar)
- Division operator `/` — ambiguous with CSS `font` shorthand; use `calc()`
- `@use` from disk — module symbols are stubbed (no file I/O)

## Dependencies

```
lattice-transpiler
  └── lattice-ast-to-css
  └── lattice-parser
        └── lattice-lexer
              └── lexer, grammar-tools
        └── parser
```

## Development

```bash
cd code/packages/go/lattice-transpiler
go vet ./...
go test ./... -cover   # coverage: 100%
```
