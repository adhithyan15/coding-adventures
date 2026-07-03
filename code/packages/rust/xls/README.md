# xls — legacy `.xls` (BIFF8) reader

A from-scratch, **zero-third-party-dependency** Rust reader for legacy Excel
`.xls` workbooks (BIFF8 / [MS-XLS]). It decodes a workbook into a typed
`Workbook → Sheet → Cell` model. Its only dependency is our own zero-dependency
[`cfb`](../cfb) OLE2 container reader.

## Where it sits in the stack

```text
  zip → deflate → xml → opc → spreadsheetml → xlsx-eval   (modern .xlsx path)
  cfb → xls                                               (legacy .xls path)  ← this crate
```

A modern `.xlsx` is a ZIP of XML parts. A legacy `.xls` is an **OLE2 Compound
File** (a tiny FAT filesystem crammed into one file) whose `Workbook` stream is
a flat sequence of **BIFF records**. The `cfb` crate turns the outer container
into named byte streams; this crate parses the `Workbook` byte stream into
cells.

## What it does

- Opens the OLE2 container via `cfb`, reads the `Workbook` stream (falling back
  to `Book` for very old files).
- Walks the BIFF record stream, handling the substream framing (`BOF`/`EOF`),
  the **globals** substream (shared string table `SST` + sheet directory
  `BOUNDSHEET`), and each **worksheet** substream's cell records.
- Decodes cell records: `LABELSST`, `RK`, `MULRK`, `NUMBER`, `LABEL`,
  `BOOLERR`, `BLANK`, and `FORMULA` (exposing the cached result; the formula
  expression itself is not decoded).
- Correctly handles the BIFF8 **CONTINUE gotcha**: an `SST` larger than 8224
  bytes spills into `CONTINUE` records, and a single string's character data can
  split across the boundary — where the first byte of the continuation is a
  fresh `fHighByte` flag for the remainder.

## Usage

```rust
use xls::{open_xls, CellValue};

fn main() -> Result<(), xls::XlsError> {
    let bytes = std::fs::read("workbook.xls").expect("read file");
    let wb = open_xls(&bytes)?;

    for sheet in wb.sheets() {
        println!("sheet: {}", sheet.name);
        for cell in sheet.cells() {
            match &cell.value {
                CellValue::Number(n) => println!("  ({},{}) = {n}", cell.row, cell.col),
                CellValue::Text(t)   => println!("  ({},{}) = {t:?}", cell.row, cell.col),
                CellValue::Bool(b)   => println!("  ({},{}) = {b}", cell.row, cell.col),
                CellValue::Error(e)  => println!("  ({},{}) = error {e:#x}", cell.row, cell.col),
                CellValue::Formula { cached } => {
                    println!("  ({},{}) = formula, cached {cached:?}", cell.row, cell.col)
                }
                CellValue::Blank => {}
            }
        }
    }
    Ok(())
}
```

## Security

This crate treats its input as **untrusted, attacker-controlled** bytes (`.xls`
files arrive as email attachments):

- `#![forbid(unsafe_code)]`, no `unwrap`/`expect`/`panic!` on the parse path,
  no panicking index/slice, no unchecked arithmetic.
- Every declared record `size`, string `cch`, `cstUnique`, and `MULRK` column
  span is bounds-checked against the bytes actually remaining before any read or
  allocation. Lying counts yield clean typed errors (`Truncated` / `TooLarge`),
  never a huge allocation or a hang.
- A `MULRK` with `colLast < colFirst` yields zero cells (checked subtraction); a
  `BOUNDSHEET.lbPlyPos` pointing outside the stream simply produces an
  unmatched (empty) sheet; the `CONTINUE` chain is walked with a hard cap.

## Row/col indexing

Rows and columns are **0-based**, exactly as BIFF stores them.

## Spec

See [`code/specs/XLS01-biff-reader.md`](../../../specs/XLS01-biff-reader.md) for
the full literate walkthrough of BIFF8, the CFB layering, and this crate's
model.

## Tests

```sh
cargo test -p xls
```
