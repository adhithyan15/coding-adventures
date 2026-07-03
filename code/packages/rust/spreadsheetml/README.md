# coding-adventures-spreadsheetml

Read the bytes of an `.xlsx` file as a **typed cell grid** — workbook → sheets →
cells, with each populated cell carrying a decoded value and, where present, its
formula text plus cached result.

This is milestone **M3** of the OOXML effort. See the full spec at
[`code/specs/SML01-spreadsheetml.md`](../../../specs/SML01-spreadsheetml.md).

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

## Not yet (deferred to M4)

- Styles, number formats, date/time interpretation. Numbers are the bare stored
  `f64`, so a cell that *displays* as `$1,000.00` or a date returns the raw
  number.
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
| `Number(f64)`   | `t` absent / `t="n"` — bare stored number                    |
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
