# spreadsheet-core

The headless **spreadsheet engine** in Rust — the essential machinery of a
spreadsheet with no UI of its own. It owns the cell model, the formula AST,
the dependency graph between cells, and the recalculation engine that keeps
everything consistent. Modern VisiCalc (and the cross-backend demos) sit on
top of it; so will a WASM build that drives the browser demos and a C-ABI
build that drives the native apps.

## The boundary it draws

> `spreadsheet-core` owns the **cell, the formula, the graph, and the
> dispatch**. It does *not* own statistics, finance, or any specific
> mathematical operation — those live in their Layer-1 cores. When
> `=AVERAGE(A1:A10)` evaluates, this crate parses the formula, resolves
> `A1:A10` to a vector, looks up `AVERAGE` in the dispatch table, and calls
> `statistics_core`.

It inlines only the function families with no life outside a spreadsheet —
logical (`IF`, `AND`, `OR`, `NOT`, `IFERROR`, `IFNA`) and information
(`ISBLANK`, `ISERROR`, `ISNA`, `ISNUMBER`, `ISTEXT`, …) — and delegates
everything else (`SUM`, `AVERAGE`, `MIN`/`MAX`, `STDEV`, trig, `ROUND`, …) to
`statistics-core`, `math-core`, `financial-core`, `lookup-core`, `text-core`,
and `datetime-core`.

## Where it sits in the stack

```
                 UI (Mosaic-generated FormulaBar + Grid; demos)
                              │  drives
   ┌──────────────────────────────────────────────────────────┐
   │  spreadsheet-core   cells · formula AST · dependency DAG · │  ← this crate
   │                     topological recalc · dispatch table     │
   └──────────────────────────────────────────────────────────┘
        │ delegates `SUM`/`AVERAGE`/`STDEV`/`ROUND`/… to
   statistics-core · math-core · financial-core · lookup-core ·
   text-core · datetime-core · numeric-tower · r-vector
```

The same crate is intended to compile to **WASM** (zero-dep `extern "C"`
linear-memory wrapper, per the repo's `grammar-wasm-support` / `iir-to-wasm`
precedent — no `wasm-bindgen`) for the HTML/WebComponent demos, and to a
**C-ABI** shared library for the native (Qt/SwiftUI/Compose/Flutter/XAML)
demos. One engine, every frontend.

## Quick start

```rust
use spreadsheet_core::{Workbook, CellAddress, CellValue};

let mut wb = Workbook::new();
let sheet = wb.add_sheet("Sheet1");

wb.set_value(sheet, CellAddress::new(1, 1), CellValue::Number(2.0)); // A1
wb.set_value(sheet, CellAddress::new(1, 2), CellValue::Number(3.0)); // B1
wb.set_formula(sheet, CellAddress::new(1, 3), "=A1+B1").unwrap();    // C1

wb.recalc_all();
assert_eq!(
    wb.get_value(sheet, CellAddress::new(1, 3)),
    Some(CellValue::Number(5.0)),
);

// Change an input — dependents recompute on the next recalc.
wb.set_value(sheet, CellAddress::new(1, 1), CellValue::Number(10.0));
wb.recalc_all();
assert_eq!(
    wb.get_value(sheet, CellAddress::new(1, 3)),
    Some(CellValue::Number(13.0)),
);
```

## Design notes

- **Portability bar**: `#![forbid(unsafe_code)]`, no I/O, no global state — so
  it lints clean and is safe to compile to WASM and link into any host.
- **Dependency DAG + recalc**: cells form a directed acyclic graph of
  references; `recalc_all` walks them in topological order. Cycles are
  detected and surfaced as a `#REF!`-style error rather than looping forever.
- **Errors propagate**: a `#DIV/0!` (or any error) in a precedent flows into
  every cell that depends on it, exactly like a real spreadsheet.

## Tests

`cargo test -p spreadsheet-core` — 80+ unit tests across the address grammar,
AST, dependency DAG, recalc ordering, dispatch table, and workbook API, plus
the doctest above.
