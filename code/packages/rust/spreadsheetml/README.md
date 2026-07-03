# coding-adventures-spreadsheetml

Read the bytes of an `.xlsx` file as a **typed cell grid** — workbook → sheets →
cells, with each populated cell carrying a decoded value and, where present, its
formula text plus cached result.

This is milestones **M3–M4** of the OOXML effort. See the full specs at
[`code/specs/SML01-spreadsheetml.md`](../../../specs/SML01-spreadsheetml.md)
(M3, the base reader) and
[`code/specs/SML02-number-formats.md`](../../../specs/SML02-number-formats.md)
(M4, number formats / dates / merged cells / defined names).

## Where it fits in the stack

```text
bytes → zip (M0) → xml-parser (M1) → opc (M2) → spreadsheetml (M3, THIS crate)
```

* [`coding-adventures-opc`](../opc) opens the ZIP, exposes named parts, and
  resolves relationship ids (`r:id="rId1"`) to part names.
* [`coding-adventures-xml-parser`](../xml-parser) parses each part's UTF-8 XML
  into a namespaced tree with entities already decoded.

This crate adds the SpreadsheetML *meaning* on top: it knows a `<c t="s">` is a
shared string, a `<f>` is a formula, and how to turn `<sheet r:id="rId1">` into
the right worksheet file.

## What it does — the two indirections

An `.xlsx` is normalized like a database. This crate resolves both indirections
so you see a plain grid:

1. **`r:id` → part.** `workbook.xml` names sheets by relationship id, not path.
   We dereference each via OPC to find the sheet's actual bytes.
2. **shared string index → text.** Text is deduplicated into one shared-string
   table; a text cell stores an *index* (`<c t="s"><v>0</v></c>`). We build the
   table once and dereference each text cell into it.

Shared strings and inline strings both surface as `Value::Text` — the caller
never sees the storage indirection.

## M4 — number formats, dates, merged cells, defined names

M4 reads `xl/styles.xml` so a raw numeric value can be *interpreted* per its
applied format. The stored value is **unchanged** (a date cell still holds
`Value::Number(45292.0)`); a `NumberFormat` is attached alongside so you can
recover the human meaning:

```rust
use coding_adventures_spreadsheetml::{open_workbook, NumberFormatKind, Value};

let wb = open_workbook(bytes)?;
let sheet = wb.sheet_by_name("Report").unwrap();

// A2 stores the serial 45292 but is styled as a date:
let a2 = sheet.cell("A2").unwrap();
assert_eq!(a2.value, Value::Number(45292.0));        // raw value untouched
assert_eq!(a2.format_kind(), NumberFormatKind::Date);
assert_eq!(a2.as_date().as_deref(), Some("2024-01-01"));  // ← the headline

// Merged cells and defined names:
assert_eq!(sheet.merged_ranges().len(), 1);          // e.g. A1:B1
assert!(wb.defined_names().iter().any(|(n, _)| n == "TaxRate"));
# Ok::<(), coding_adventures_spreadsheetml::XlsxError>(())
```

**The style chain.** A cell carries a *style index* `s=`, not a format:
`<c s="1">` → `cellXfs[1]` → `numFmtId 14` → the built-in code `m/d/yyyy` →
`Date`. Ids `< 164` are spec-defined built-ins (hard-coded); ids `≥ 164` are
custom, defined in `<numFmts>` with an explicit `formatCode`.

**The 1900 date system.** Serial `0` is anchored at the fictitious `1899-12-30`,
which reproduces Excel's phantom `1900-02-29` leap-year bug for serials ≥ 60.
So `serial_to_date(1.0) == "1900-01-01"`, `serial_to_date(60.0) == "1900-02-29"`
(the bug), `serial_to_date(45292.0) == "2024-01-01"`.

`formatted()` is a *pragmatic* renderer (not a full number-format engine): exact
ISO strings for dates, `value ×100 + "%"` for percent, and the raw number for
currency (no symbol synthesis).

### Still deferred

- A full number-format renderer (grouping, decimal places, currency symbols).
- Formula **evaluation**. Formulas return their text plus the cached value.

## Usage

```rust
use coding_adventures_spreadsheetml::{open_workbook, Value};

let wb = open_workbook(bytes)?;              // bytes: &[u8] of an .xlsx
assert_eq!(wb.sheet_names(), vec!["Revenue".to_string()]);

let sheet = wb.sheet_by_name("Revenue").unwrap();
assert_eq!(sheet.cell("A1").unwrap().value, Value::Text("Q1".into()));
assert_eq!(sheet.cell("B1").unwrap().value, Value::Number(1000.0));

let b2 = sheet.cell("B2").unwrap();
assert_eq!(b2.formula, Some("SUM(B1:B1)".to_string()));
assert_eq!(b2.value, Value::Number(1000.0));   // cached result

// Iterate populated cells in reading order:
for cell in sheet.cells() {
    println!("{} = {:?}", cell.reference, cell.value);
}
# Ok::<(), coding_adventures_spreadsheetml::XlsxError>(())
```

### A1 references

`parse_a1_ref` turns an A1 reference into `(col, row)`, both **1-based**. The
column letters are *bijective* base-26 (`A`=1 … `Z`=26, `AA`=27):

```rust
use coding_adventures_spreadsheetml::parse_a1_ref;
assert_eq!(parse_a1_ref("A1"), Some((1, 1)));
assert_eq!(parse_a1_ref("AA10"), Some((27, 10)));
```

## Cell value types

| `Value`         | source                                                       |
|-----------------|--------------------------------------------------------------|
| `Number(f64)`   | `t` absent / `t="n"` — bare stored number (see M4 formats)    |
| `Text(String)`  | `t="s"` (shared), `t="str"` (formula result), `t="inlineStr"`|
| `Bool(bool)`    | `t="b"`                                                      |
| `Error(String)` | `t="e"`, e.g. `#DIV/0!`                                      |
| `Empty`         | a cell with no `<v>`/`<is>`/`<f>`                             |

## Testing

```sh
cargo test -p coding-adventures-spreadsheetml -- --nocapture
```

The end-to-end test opens a real DEFLATE-compressed `.xlsx` fixture (checked in
as bytes) and asserts the full Revenue-sheet grid.

## License

MIT.
