# read-xlsx

Open a real `.xlsx` spreadsheet and print its **evaluated** cell grid. This is the
runnable end-goal of the OOXML effort: it stitches the whole zero-third-party-
dependency stack together to turn spreadsheet bytes into a table of computed
values.

## The stack it exercises

```
.xlsx bytes
  → zip           unzip the OPC package (DEFLATE inflate)
  → xml-parser    namespace-aware parse of each XML part
  → opc           [Content_Types].xml + .rels → resolve parts by relationship
  → spreadsheetml workbook → sheets-by-r:id → cells + shared strings
  → styles.xml    number formats (serial 45292 → "2024-01-01", %, currency)
  → xlsx-eval     recompute <f> formulas via the spreadsheet-core engine
```

Every formula cell is **recomputed from scratch** — the cached value on disk is
ignored — so the output reflects what a live spreadsheet host would show.

## Usage

```
read-xlsx <file.xlsx>   # open a spreadsheet file and print its sheets
read-xlsx --demo        # run the two built-in fixtures (no file needed)
read-xlsx --help        # show help
```

## Example (`read-xlsx --demo`)

```
=== demo: minimal.xlsx (formulas) — 1929 bytes ===
Sheet "Revenue" — 4 cells
  cell  kind         display              formula            recomputed
  A1    General      Q1                   —                  Text("Q1")
  B1    General      1000                 —                  Number(1000.0)
  A2    General      Total                —                  Text("Total")
  B2    General      1000                 SUM(B1:B1)         Number(1000.0)

=== demo: styled.xlsx (number formats) — 2377 bytes ===
Sheet "Report" — 6 cells
  cell  kind         display              formula            recomputed
  A2    Date         2024-01-01           —                  Number(45292.0)
  B2    Currency     1234.5               —                  Number(1234.5)
  B4    Percent      25%                  —                  Number(0.25)
```

The `recomputed` column proves the `SUM(B1:B1)` formula was **evaluated**, not read
from a cache; the `display` column shows number formats applied (a raw serial
`45292` becomes the date `2024-01-01`, `0.25` becomes `25%`).

## As a library

`read_xlsx::render_xlsx(bytes) -> Result<Vec<RenderedSheet>, RenderError>` returns
the structured report; `format_report(&sheets)` renders the plain-text table.

## Tests

```
cargo test -p read-xlsx -- --nocapture
```
