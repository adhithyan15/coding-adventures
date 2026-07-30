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
    transition background-color $duration-fast $easing-out ;
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

Transitions may be declared at the part level, where they apply to every state
change, or inside a `state` block, where they apply when entering that state:

```msl
part root {
  opacity: 1 ;
  transition opacity $duration-normal ; // easing defaults to ease-out

  state disabled {
    opacity: $opacity-disabled ;
    transition opacity $duration-slow linear ;
  }
}
```

The transitioned property must have a base declaration in the same part. The
compiler resolves duration and easing tokens, validates the property reference,
emits a Lattice `transition` declaration, and preserves structured transition
entries in the backend-neutral style map JSON for native emitters.

## Lattice Output

Class names follow the `.mos-{ComponentName}-{part-name}` convention:

```scss
.mos-Grid-root {
  background-color: #1e1e1e;
  border-radius: 6px;
  transition: background-color 80ms ease-out;
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
