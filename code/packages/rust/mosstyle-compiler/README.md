# mosstyle-compiler

Compiles `.msl` (Mosaic Style Language) files into scoped CSS.

## Role in the Mosaic stack

```
.msl  ──▶  mosstyle-compiler  ──▶  CSS string
```

A `.msl` file answers exactly one question: *what do the parts of a component
look like?*  It maps part names (declared in the `.mll` layout file) to CSS
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

**Design tokens** — `$token-name` references resolve to hex literals using the
UI15 dark-mode palette baked into the compiler.  No runtime CSS variable lookup
is needed for static rendering.  Full Lattice token file support is planned for v2.

## CSS output

Class names follow the `.mos-{ComponentName}-{part-name}` convention:

```css
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

**State → CSS selector mapping:**

| State      | CSS selector             |
|------------|--------------------------|
| `hover`    | `:hover`                 |
| `pressed`  | `:active`                |
| `focused`  | `:focus-visible`         |
| `disabled` | `.disabled` (class)      |
| `selected` | `.selected` (class)      |
| `editing`  | `.editing` (class)       |
| `error`    | `.error` (class)         |

## API

```rust
use mosstyle_compiler::{compile, CompileOutput};

let css = compile(src, None).expect("compile failed").css;
println!("{css}");
```

`compile(source, part_map_json)`:
- `source` — raw `.msl` source text
- `part_map_json` — optional JSON from `moslayout-compiler`; when supplied,
  every `part` name in the style is validated against the layout's declared parts.

## Running tests

```sh
cargo test -p mosstyle-compiler -- --nocapture
```
