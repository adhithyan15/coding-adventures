# array-core

Canonical Rust dynamic-array helpers — Excel 365 array functions for any spreadsheet frontend.

## What this is

`array-core` is a Layer-1 Rust crate in the spreadsheet/statistics stack. It implements the shape-mutating, generating, and selecting array helpers that Excel 365 surfaces as dynamic-array functions (`SEQUENCE`, `TAKE`, `SORT`, `UNIQUE`, etc.) so that any frontend (VisiCalc faithful, modern reconstruction, a Sheets-style host) can share a single, well-tested implementation.

The crate is pure compute: no I/O, no platform code, no globals, WASM-compatible. The only dependencies are sibling Layer-0 crates `numeric-tower` and `r-vector`.

## Where it fits

See `code/specs/backend-crate-catalog.md` (Layer 1 row `array-core`) and `code/specs/statistics-core.md` for sibling conventions. Cell-level broadcast/spill logic lives in a higher layer (`spreadsheet-core`); this crate just provides the array math.

## Functions (Phase 1)

| Module      | Functions                                     |
|-------------|-----------------------------------------------|
| `generate`  | `sequence`                                    |
| `shape`     | `take`, `drop`, `expand`                      |
| `stack`     | `hstack`, `vstack`                            |
| `reshape`   | `to_row`, `to_col`, `wrap_rows`, `wrap_cols`  |
| `pick`      | `choose_rows`, `choose_cols`                  |
| `filter`    | `filter`                                      |
| `sort`      | `sort`, `sort_by`                             |
| `unique`    | `unique`                                      |

`RANDARRAY` is deferred — it requires a pluggable RNG (handled by a separate crate in the catalog).

## Conventions

- **1-based indexing at the public surface** (`CHOOSEROWS`, `SORT`'s `sort_index`). Negative indices count from the end, matching Excel.
- **NA via the `r-vector` bit pattern**. `na_real()` is re-exported. NA in sort keys lands at the end (ascending) / beginning (descending); UNIQUE collapses duplicate NAs; FILTER treats NA-in-mask as `FALSE`.
- **NA-padding for shape mismatches**: HSTACK / VSTACK / EXPAND / WRAPROWS / WRAPCOLS pad with NA when the inputs do not divide evenly.

## Example

```rust
use array_core::{generate::sequence, sort::sort, shape::take};

let a = sequence(5, None, Some(1.0), Some(1.0))?;  // 1,2,3,4,5 as a column
let top3 = take(&a, 3, None)?;                       // 1,2,3
let desc = sort(&a, None, Some(-1), None)?;          // 5,4,3,2,1
```

## Tests

```sh
cargo test -p array-core
```

Phase 1 ships ~50 integration tests covering happy paths, Excel-365 edge cases (negative indices, over-take/over-drop, NA-in-mask, NA-in-sort, exactly-once UNIQUE), and error paths.
