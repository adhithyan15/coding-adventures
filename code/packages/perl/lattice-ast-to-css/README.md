# CodingAdventures::LatticeAstToCss (Perl)

Lattice AST → CSS compiler. Walks a Lattice AST (produced by
`CodingAdventures::LatticeParser`) and emits plain CSS text.

## Synopsis

```perl
use CodingAdventures::LatticeParser;
use CodingAdventures::LatticeAstToCss;

my $ast = CodingAdventures::LatticeParser->parse(<<'LATTICE');
    $primary: #4a90d9;

    @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
        padding: 8px 16px;
    }

    .btn {
        @include button($primary);
    }
LATTICE

my $css = CodingAdventures::LatticeAstToCss->compile($ast);
```

Output:

```css
.btn {
  background: #4a90d9;
  color: white;
  padding: 8px 16px;
}
```

## API

### `compile($ast)` → `$css_string`

Compile a Lattice AST (root `stylesheet` ASTNode) to CSS text.

Returns a CSS string.  Returns `''` for empty input.  Always ends with `\n`.

## Features

- Variable declaration and `$var` reference expansion
- Nested rule flattening (`.parent { .child { } }` → `.parent .child { }`)
- `&` parent reference in selectors
- Mixin definition (`@mixin`) and `@include` expansion with parameter
  defaults
- `@if` / `@else if` / `@else` compile-time evaluation
- `@for $i from N through M` and `@for $i from N to M` loop unrolling
- `@each $var in list` loop iteration
- `@while` loop (capped at 1000 iterations)
- `@function` definition and call-site evaluation
- CSS built-in function passthrough (rgb, calc, etc.)
- Lexical scope chain for variable isolation
