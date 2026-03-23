# lattice-transpiler

End-to-end pipeline: Lattice source text → CSS text. Combines
`lattice_parser`, `lattice_ast_to_css`, and the CSS emitter into a single
`CodingAdventures::LatticeTranspiler.transpile` call.

## Usage

```ruby
require "coding_adventures_lattice_transpiler"

TP = CodingAdventures::LatticeTranspiler

# Basic transpilation
css = TP.transpile(<<~LATTICE)
  $primary: #4a90d9;
  $pad: 8px;

  @mixin flex-center {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  @function double($n) {
    @return $n * 2;
  }

  .container {
    @include flex-center;
    padding: double($pad);
  }

  h1 { color: $primary; }
LATTICE

# Minified output
minified = TP.transpile("h1 { color: red; }", minified: true)
# => "h1{color:red;}"

# Custom indentation
indented = TP.transpile("h1 { color: red; }", indent: "    ")
```

## Supported Lattice Features

| Feature          | Syntax                                    |
|------------------|-------------------------------------------|
| Variables        | `$name: value;`                           |
| Mixins           | `@mixin name($p) { ... }`                 |
| Include          | `@include name(args);`                    |
| Conditionals     | `@if $cond { } @else if $c2 { } @else { }`|
| For loop         | `@for $i from 1 through 12 { }`           |
| Each loop        | `@each $c in red, green, blue { }`        |
| Functions        | `@function name($p) { @return expr; }`    |
| Modules          | `@use "file";`                            |
| CSS pass-through | All standard CSS3 syntax                  |

## Dependencies

- `coding_adventures_lattice_ast_to_css`
- `coding_adventures_lattice_parser`
- `coding_adventures_lattice_lexer`

## Development

```bash
bundle exec rake test
```
