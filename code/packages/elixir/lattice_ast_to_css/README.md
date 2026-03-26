# lattice-ast-to-css (Elixir)

Three-pass Lattice AST-to-CSS compiler. Transforms a Lattice AST (containing
variables, mixins, functions, control flow) into a clean CSS AST suitable for
emission.

## Architecture

The package contains seven modules:

- **Errors** -- structured error types (undefined variable, wrong arity, etc.)
- **Scope** -- lexical scope chain with `set_global` for `!global` flag support
- **Values** -- typed Lattice values (number, dimension, string, color, map, list)
- **Evaluator** -- compile-time expression evaluator
- **Builtins** -- 31 built-in functions (map, color, list, type, math)
- **Transformer** -- three-pass AST transformation (symbol collection, expansion, cleanup)
- **Emitter** -- CSS text generator from a clean CSS AST

## Lattice v2 Features

This package implements all Lattice v2 features:

- `@while` loops with max-iteration guard (1000 default)
- `!default` and `!global` variable flags
- `@content` blocks for mixins
- `@at-root` directive (hoists rules to stylesheet root)
- `@extend` and `%placeholder` selectors
- Property nesting (`font: { size: 14px; weight: bold; }`)
- Variables in selectors (`.col-$i`)
- Maps as first-class value type
- 31 built-in functions: map-get, lighten, darken, math.div, type-of, etc.

## Usage

```elixir
alias CodingAdventures.LatticeAstToCss

# Get a clean CSS AST
{:ok, css_ast} = LatticeAstToCss.transform(lattice_ast)

# Get CSS text directly
{:ok, css_text} = LatticeAstToCss.transform_to_css(lattice_ast)

# With options
{:ok, minified} = LatticeAstToCss.transform_to_css(ast, minified: true)
```

## Dependencies

- lattice-parser
- lattice-lexer
- grammar-tools
- parser
- lexer

## Development

```bash
# Run tests
mix test

# Run tests with coverage
mix test --cover

# Compile
mix compile
```
