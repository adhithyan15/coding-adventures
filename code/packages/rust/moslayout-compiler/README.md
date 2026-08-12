# moslayout-compiler

Compiles `.mll` (Mosaic Layout Language) files into a typed layout IR and a part-map JSON.

## Role in the Mosaic stack

```
.mil  ──▶  mosmodel-compiler ──▶  interface descriptor JSON
                                       │
.mll  ──▶  moslayout-compiler ◀────────┘  ──▶  part-map JSON (→ mosstyle)
```

A `.mll` file answers exactly one question: *how are a component's primitives
arranged in space, and how do they wire to the component's slots?*

## Primitives

| Name     | Children? | Props                                                |
|----------|-----------|------------------------------------------------------|
| `Box`    | yes       | `direction`, `align`, `justify`, `wrap`, `grow`, …  |
| `Row`    | yes       | same (direction fixed to row)                        |
| `Column` | yes       | same (direction fixed to column)                     |
| `Text`   | no        | `slot: <name>` or `content: slot: <name>`            |
| `Image`  | no        | `slot: <name>` or `source: slot: <name>`             |
| `Spacer` | no        | optional `grow: <number>`                            |
| `Grid`   | no        | `headers: slot: <name>`, `rows: slot: <name>`        |

## Syntax

```mll
layout Grid {
  Column [ root ] {
    Grid [ cell-grid ] (
      headers: slot: column-headers ,
      rows:    slot: viewport-rows
    )
  }
}
```

**Prop shorthand** — For single-slot leaf nodes, the prop name can be omitted:

```mll
Text [ label ] ( slot: display-name )
// equivalent to:
Text [ label ] ( content: slot: display-name )
```

**Default authored children** — A reusable component with a `node` or
`list<node>` slot mounts caller-authored child subtrees in child position:

```mll
layout Surface {
  Column [ surface-root ] {
    slot: children
  }
}
```

The package resolver replaces this typed mount when a consumer writes
`Surface { ... }`. One default child mount is supported per layout; named child
regions are tracked separately by UI29-2.

## API

```rust
use moslayout_compiler::{compile, CompileOutput};

let src = r#"layout Button { Box [ root ] { Text [ label ] ( slot: label ) } }"#;
let out: CompileOutput = compile(src, None).expect("compile failed");

println!("{}", out.part_map_json);
// {"component":"Button","parts":[{"name":"root","primitive":"Box"},{"name":"label","primitive":"Text"}]}
```

`compile(source, interface_json)`:
- `source` — raw `.mll` source text
- `interface_json` — optional descriptor JSON from `mosmodel-compiler`; when
  supplied, all slot/emit references are validated against the declared interface.

## Running tests

```sh
cargo test -p moslayout-compiler -- --nocapture
```

After editing the canonical grammar, regenerate its embedded Rust form with:

```sh
cargo run -p moslayout-compiler --bin regen_grammar
```
