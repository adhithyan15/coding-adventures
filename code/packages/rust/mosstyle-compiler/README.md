# mosstyle-compiler

Compiles `.msl` (Mosaic Style Language) files into scoped Lattice source.

## Role In The Mosaic Stack

```text
.msl -> mosstyle-compiler -> Lattice source
                         \-> resolved style map JSON
```

A `.msl` file answers exactly one question: what do the parts of a component
look like? It maps part names declared in the `.mll` layout file to Lattice
property declarations, with optional state selectors.

## Syntax

```msl
style Grid {

  part root {
    background-color: $color-surface ;
    border-radius: 6px ;
  }

  part cell-grid {
    width: 100% ;
    border-collapse: collapse ;
    color: $color-text-primary ;

    state hover {
      background-color: $color-surface-hover ;
    }
  }
}
```

Design tokens such as `$token-name` resolve to literal values using the UI15
dark-mode palette baked into the compiler. Full Lattice token file support is
planned for a later pass.

## Lattice Output

Class names follow the `.mos-{ComponentName}-{part-name}` convention:

```scss
.mos-Grid-root {
  background-color: #1e1e1e;
  border-radius: 6px;
}
.mos-Grid-cell-grid {
  width: 100%;
  border-collapse: collapse;
  color: #ffffff;
}
.mos-Grid-cell-grid:hover {
  background-color: #2e2e2e;
}
```

The current emitter writes CSS-compatible Lattice, which the repo's Lattice
transpiler accepts directly. Mosaic itself emits Lattice from the style stage;
web backends can compile that Lattice to CSS at their platform boundary.

## State Selector Mapping

| State | CSS selector |
| --- | --- |
| `hover` | `:hover` |
| `pressed` | `:active` |
| `focused` | `:focus-visible` |
| `disabled` | `.disabled` class |
| `selected` | `.selected` class |
| `editing` | `.editing` class |
| `error` | `.error` class |

## API

```rust
use mosstyle_compiler::compile;

let lattice = compile(src, None).expect("compile failed").lattice;
println!("{lattice}");
```

`compile(source, part_map_json)`:

- `source`: raw `.msl` source text.
- `part_map_json`: optional JSON from `moslayout-compiler`; when supplied,
  every `part` name in the style is validated against the layout's declared
  parts.

## Running Tests

```sh
cargo test -p mosstyle-compiler -- --nocapture
```
