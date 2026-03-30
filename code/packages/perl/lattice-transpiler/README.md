# CodingAdventures::LatticeTranspiler (Perl)

End-to-end Lattice → CSS transpiler. Takes a Lattice source string (or
file) and returns compiled CSS text. Wires together
`CodingAdventures::LatticeParser` and `CodingAdventures::LatticeAstToCss`
into a single convenience API.

## Synopsis

```perl
use CodingAdventures::LatticeTranspiler;

my ($css, $err) = CodingAdventures::LatticeTranspiler->transpile(<<'END');
    $primary: #4a90d9;

    @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
        padding: 8px 16px;
    }

    .btn {
        @include button($primary);
        &:hover { opacity: 0.9; }
    }
END

die "Error: $err\n" if $err;
print $css;
```

Output:

```css
.btn {
  background: #4a90d9;
  color: white;
  padding: 8px 16px;
}

.btn:hover {
  opacity: 0.9;
}
```

## API

### `transpile($source)` → `($css, $error)`

Transpile a Lattice source string. Returns `($css_string, undef)` on
success or `(undef, $error_message)` on failure.

### `transpile_file($path)` → `($css, $error)`

Read a file and transpile it. Returns `($css_string, undef)` on success
or `(undef, $error_message)` if the file cannot be opened or is invalid.

## Pipeline

```
Lattice source
    │
    ▼  LatticeParser->parse()
  AST
    │
    ▼  LatticeAstToCss->compile()
  CSS text
```
