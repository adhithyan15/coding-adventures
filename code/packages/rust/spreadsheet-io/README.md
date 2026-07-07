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
  .xlsx ──load_xlsx────┐                              ┌────save_xlsx──▶ .xlsx
  .xls  ──load_xls─────┤                              ├────save_xls───▶ .xls
  .csv/.tsv ─load_csv──┤─▶ spreadsheet-core::Workbook ─┤────save_csv───▶ .csv/.tsv
  .json ──load_json────┘        (THE model)            └────save_json──▶ .json
```

Modern `.xlsx` (SSIO01), legacy `.xls`/BIFF8 (SSIO02), delimited `.csv`/`.tsv`
(SSIOCSV01), and `.json` records (SSIOJSON01) are supported. `.xlsx` preserves
live formulas; `.xls`, CSV, and JSON are lower-fidelity (values only — see below).
Because everything lands in the one `Workbook`, any format loads and any format
saves: `load_csv` then `save_xlsx` converts a CSV to Excel, `load_json` then
`save_csv` flattens an API payload to a spreadsheet, and a loaded sheet is
queryable with SQL via `sql-spreadsheet-source`.

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

## JSON (records) — SSIOJSON01

JSON has no single canonical spreadsheet shape, so `load_json`/`save_json`
standardize on the one nearly every data API emits: a top-level **array of
objects** ("records"). Keys become the header row (the union of keys across
records, in first-seen order); each object becomes a data row; a missing key is
a blank cell:

```text
  [ {"region":"East","sales":200},        region | sales
    {"region":"West","sales":340} ]  ──▶    East  |  200
                                            West  |  340
```

`save_json` is the inverse (row 1 = keys, rows below = objects), so a records
file round-trips. `load_json` also accepts an array of arrays (positional grid),
an array of scalars (single column), a single object (header + one row), and a
top-level scalar (single cell); a nested object/array inside a value is stored as
its compact JSON text. Formulas export as their computed value and, like CSV,
only the first sheet is written. `load_json` parses **panic-free** (malformed
bytes are an `IoError::Json`, never a crash), since JSON is untrusted input. See
`code/specs/SSIOJSON01-spreadsheet-io-json.md`.

## Where it sits

```
spreadsheetml + xlsx-eval / xls / csv-parser / json-parser  (read)  ─┐
                                                                     ├─▶ spreadsheet-io ◀─▶ spreadsheet-core
xlsx-writer / xls-writer / json-serializer                  (write) ─┘                        (the engine)
```

Later milestones wire load/save into the `SpreadsheetSession` facade and surface
open/save buttons in the VisiCalc apps across every backend.
