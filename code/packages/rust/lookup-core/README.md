# lookup-core

Canonical Rust lookup/reference core for VisiCalc, spreadsheet, R, and S
frontends.  Implements the classic Excel lookup-family functions over a
frontend-agnostic value type.

## Where this fits in the stack

`lookup-core` is one of the Layer-1 backend crates described in
`code/specs/backend-crate-catalog.md`.  Like the rest of Layer 1 it is:

- Pure Rust, no platform code.
- WASM-compatible (no I/O, no globals, no `unsafe`).
- Depends only on `numeric-tower` and `r-vector`.

A future `spreadsheet-core` will own the canonical `CellValue` and call into
`lookup-core` by adapting `CellValue → LookupValue` at the dispatch
boundary.

## Functions implemented (Phase 1)

| Function                 | Notes                                              |
|--------------------------|----------------------------------------------------|
| `vlookup` / `hlookup`    | Exact + approximate (binary search) match.         |
| `index_1d` / `index_2d`  | 1-based; `row=0` or `col=0` selects whole strip.   |
| `match`                  | `Exact` / `LessOrEqual` / `GreaterOrEqual`.        |
| `xlookup` / `xmatch`     | All Excel `match_mode` × `search_mode` combos.     |
| `offset`                 | Sub-table extraction with bounds checking.         |
| `choose`                 | 1-based variadic pick.                             |
| `row` / `column` / `rows` / `columns` | Shape-introspection helpers.          |

## NA propagation

- NA in the *probe* → output is NA.
- NA in the *lookup array* is skipped — those cells never match.

This matches `code/specs/na-semantics.md` and Excel's observable behaviour.

## Example

```rust
use lookup_core::vlookup::vlookup;
use lookup_core::LookupValue;

let table = vec![
    vec![LookupValue::Text("apple".into()),  LookupValue::Number(1.0)],
    vec![LookupValue::Text("banana".into()), LookupValue::Number(2.0)],
];
let r = vlookup(&LookupValue::Text("banana".into()), &table, 2, false).unwrap();
assert_eq!(r, LookupValue::Number(2.0));
```
