# spreadsheet-io

**One spreadsheet core, many file formats.** This crate is the adapter layer
that lets the live [`spreadsheet-core`] engine — the model every VisiCalc
front-end computes on — read and write real spreadsheet files.

## Why it exists

The repo grew five separate spreadsheet models: the engine, plus a private
`Workbook`/`Cell` type inside each `.xlsx`/`.xls` reader and writer crate. None
could talk to another. `spreadsheet-io` makes `spreadsheet-core::Workbook` the
**single hub** and provides the one conversion layer between file bytes and a
live workbook:

```text
  .xlsx ──load_xlsx──┐                              ┌──save_xlsx──▶ .xlsx
                     ├─▶  spreadsheet-core::Workbook ─┤
  .xls  ──load_xls───┘     (THE model)               └──save_xls───▶ .xls
```

Both modern `.xlsx` (SSIO01) and legacy `.xls`/BIFF8 (SSIO02) are supported.
`.xlsx` preserves live formulas; `.xls` is lower-fidelity (values only — see
below).

It is the *only* crate that depends on both the engine and the file-format
codecs, so the engine never learns what a `.xlsx` is and each codec stays small.

## API

```rust
use spreadsheet_core::{CellAddress, CellValue, Workbook};
use spreadsheet_io::{load_xlsx, save_xlsx};

// Load a file into a live, editable workbook (formulas stay formulas):
let wb = load_xlsx(&bytes)?;

// Build/edit in the engine, then save back to .xlsx bytes:
let mut wb = Workbook::new();
let s = wb.add_sheet("Sheet1");
wb.set_value(s, CellAddress::new(1, 1), CellValue::Number(10.0));
wb.set_value(s, CellAddress::new(1, 2), CellValue::Number(20.0));
wb.set_formula(s, CellAddress::new(1, 3), "=SUM(A1:B1)").unwrap();
wb.recalc_all();
let bytes = save_xlsx(&wb); // C1 is written as =SUM(A1:B1) with cached 30
```

## Round-trip guarantee

`save_xlsx → load_xlsx` preserves every populated cell's computed value and
keeps formula cells as **live formulas** (they recompute when inputs change).
The write side is idempotent — repeated open/save cannot drift the file. The
crate's tests assert all of this, and an openpyxl cross-check proves the output
is a genuine `.xlsx` third-party tools accept.

## Fidelity limits (current)

The `.xlsx` writer's value model can't yet express everything the engine holds:

- **Formulas with non-numeric results** are written as their computed value (the
  value round-trips; the formula does not) — the writer's cache slot is `f64`.
- **Booleans** are written as `1`/`0` (no `t="b"`).
- **Number formats** are not yet emitted.

Numbers, text, and numeric-result formulas — a whole VisiCalc-authored sheet —
round-trip exactly. See `code/specs/SSIO01-spreadsheet-io.md`.

**Legacy `.xls` (SSIO02) is lower-fidelity:** its reader recovers cell values but
not formula *expressions*, and its writer stores only numbers and strings — so
`.xls` formulas become their computed value (or empty, if the producer didn't
cache one), booleans become `1`/`0`, and errors become their display text.
Numbers and text round-trip exactly. Prefer `.xlsx` when formulas matter. See
`code/specs/SSIO02-spreadsheet-io-xls.md`.

## Where it sits

```
spreadsheetml + xlsx-eval / xls   (read)  ─┐
                                           ├─▶ spreadsheet-io ◀─▶ spreadsheet-core
xlsx-writer / xls-writer          (write) ─┘                        (the engine)
```

Later milestones wire load/save into the `SpreadsheetSession` facade and surface
open/save buttons in the VisiCalc apps across every backend.
