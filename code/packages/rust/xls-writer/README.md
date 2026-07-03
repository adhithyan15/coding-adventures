# xls-writer

A from-scratch, **zero-third-party-dependency** writer for the legacy
**`.xls` / BIFF8** spreadsheet format ([MS-XLS]). You build a simple model —
sheets of string and number cells — and it produces the bytes of a real `.xls`
file: a stream of **BIFF records** wrapped in an **OLE2 Compound File** via the
sibling [`cfb-writer`](../cfb-writer) crate. This is milestone **C4** (legacy
write) of the OOXML/BIFF stack.

`#![forbid(unsafe_code)]`, pure `std` + `cfb-writer`, deterministic output (no
timestamps or randomness), never panics on the public API path.

## Where it fits in the stack

```
        deflate ── zip ── xml ── opc ── spreadsheetml ── xlsx-eval   (OOXML, .xlsx)

   cfb (reader) ◄──────── round-trip proof ────────► cfb-writer     (OLE2 container)
                                                          │
                                                     xls-writer      (this crate — BIFF8)
```

`xls-writer` sits on top of `cfb-writer`: it builds one `Workbook` BIFF stream
and hands it to `cfb-writer` to be wrapped into the final `.xls`. The round-trip
test then re-opens that `.xls` with the `cfb` reader and re-parses the BIFF
records, closing the loop `model → BIFF → CFB → cfb reader → BIFF → model`.

## What a `.xls` is (one paragraph)

A `.xls` is an OLE2 Compound File containing a single stream named `Workbook`,
which holds a flat sequence of **BIFF records** (`u16 type`, `u16 size`, body).
Records are grouped into substreams bracketed by `BOF`…`EOF`: one **globals**
substream (declaring sheets via `BOUNDSHEET` and holding the shared-string table
`SST`), then one **worksheet** substream per sheet (the cell records `LABELSST`
for strings and `NUMBER` for numbers). See `code/specs/XLSW01-xls-writer.md` for
the full literate walkthrough, including the `BOUNDSHEET.lbPlyPos` two-pass.

## Usage

```rust
use xls_writer::{Workbook, write_xls};

let mut wb = Workbook::new();
let sheet = wb.add_sheet("Revenue");
sheet.set_string(0, 0, "Q1");
sheet.set_number(0, 1, 1000.0);
sheet.set_string(1, 0, "Total");
sheet.set_number(1, 1, 1234.5);

let bytes: Vec<u8> = write_xls(&wb);
std::fs::write("revenue.xls", &bytes).unwrap();
```

All rows/cols are **0-based**. Identical string values are automatically shared
(de-duplicated) into a single SST entry, and each string cell references it by
index.

## Design notes

- **`NUMBER`, not `RK`.** Every numeric cell is a `NUMBER` record (an 8-byte
  `f64`). `RK` is only a space optimization for certain values; `NUMBER` is
  total and trivially verifiable. See the spec §2.1.
- **String encoding.** Strings that are entirely Latin-1 (every code unit
  ≤ `0xFF`) use the compact 8-bit form; anything else uses 16-bit UTF-16LE
  (the `fHighByte` flag).

## Limitations

- **No `CONTINUE` splitting.** The SST is emitted as a single record; a workbook
  with enough distinct strings to overflow one BIFF record (64 KiB body) would,
  in a full implementation, spill into `CONTINUE` records. We keep the SST small
  and clamp rather than emit a corrupt record; splitting is future work.
- **`u16` grid.** Cells beyond row/col `65535` cannot be addressed in BIFF and
  are skipped (documented, never wrapped into a wrong address).

## Testing

```
cargo test -p xls-writer -- --nocapture
```

The headline test writes a `.xls`, re-opens it with the `cfb` reader, and walks
the BIFF records to assert the sheet name, the `lbPlyPos` offsets, the SST
contents, and every cell's address/type/value.
