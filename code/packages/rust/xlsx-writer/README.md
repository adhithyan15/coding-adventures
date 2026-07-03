# coding-adventures-xlsx-writer

Turns a **simple in-memory workbook model** into valid `.xlsx` bytes, by
generating the SpreadsheetML XML parts and packaging them via the generic
[`opc-writer`](../opc-writer). It is the write-side mirror of the
[`spreadsheetml`](../spreadsheetml) reader.

This is milestone **C1** of the OOXML effort. See
[`code/specs/XLSXW01-xlsx-writer.md`](../../../specs/XLSXW01-xlsx-writer.md) for
the full literate write-up.

## Where it fits

```text
  Workbook model ─► xlsx-writer ─► opc-writer ─► .xlsx bytes
                       │
                       ├─ xl/workbook.xml
                       ├─ xl/worksheets/sheetN.xml
                       ├─ xl/sharedStrings.xml
                       └─ *.rels
```

## Usage

```rust
use coding_adventures_xlsx_writer::{Workbook, write_xlsx};

let mut wb = Workbook::new();
let sheet = wb.add_sheet("Revenue");
sheet.set_string("A1", "Q1");
sheet.set_number("B1", 1000.0);
sheet.set_string("A2", "Total");
sheet.set_formula("B2", "SUM(B1:B1)", 1000.0); // formula text WITHOUT '='

let bytes: Vec<u8> = write_xlsx(&wb); // real .xlsx bytes
```

## API

| Item | Purpose |
|------|---------|
| `Workbook::new` / `add_sheet(name)` | Build the workbook; sheets keep insertion order. |
| `Sheet::set_number(a1, n)` | A numeric cell. |
| `Sheet::set_string(a1, s)` | A text cell (deduplicated into the shared-string table). |
| `Sheet::set_formula(a1, f, cached)` | A formula cell (`f` without `=`) plus its cached value. |
| `write_xlsx(&wb)` | Serialize to `.xlsx` bytes. |
| `parse_a1` / `col_to_letters` | A1 reference helpers (also public for reuse). |

A cell whose A1 reference does not parse is a silent no-op (never a panic).

## The round-trip proof

The test suite writes a "Revenue" sheet and re-opens the bytes with **this
repo's own readers**:

* `spreadsheetml::open_workbook` — structural read (sheet name, string cells,
  number cell, formula text).
* `xlsx-eval::open_and_evaluate` — recomputes `SUM(B1:B1)` from the formula text
  (ignoring the cached value) and confirms it equals `1000`.

Write → our reader → correct values (including a formula that *recomputes*) is
the milestone's core proof. Additional round-trips cover multiple sheets with
shared-string dedup, XML-special characters, Unicode, and a cross-sheet formula.

## Testing

```sh
cargo test -p coding-adventures-xlsx-writer
```

## Guarantees

* `#![forbid(unsafe_code)]`.
* No `unwrap`/`expect`/`panic!` on any input path; XML escaping is total.
* No filesystem, network, process, or environment access (see
  `required_capabilities.json`).
