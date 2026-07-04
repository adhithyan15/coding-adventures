# SSIOCSV01 — `spreadsheet-io`: delimited text (CSV / TSV) load & save

Extends `spreadsheet-io` (which unified `.xlsx`/`.xls` onto `spreadsheet-core`)
with **CSV / TSV**, so the same engine reads and writes plain delimited tables —
the first step of "read and process any tabular format, export any tabular
format" onto the one core. A CSV loaded this way is editable in VisiCalc,
queryable with SQL (`sql-spreadsheet-source`), and re-exportable as `.xlsx`.

## API (added)

```rust
pub fn load_csv(bytes: &[u8]) -> Result<Workbook, IoError>;
pub fn load_tsv(bytes: &[u8]) -> Result<Workbook, IoError>;
pub fn load_delimited(bytes: &[u8], delimiter: char) -> Result<Workbook, IoError>;

pub fn save_csv(wb: &Workbook) -> Vec<u8>;
pub fn save_tsv(wb: &Workbook) -> Vec<u8>;
pub fn save_delimited(wb: &Workbook, delimiter: char) -> Vec<u8>;
```

`IoError` gains a `Csv(String)` variant (invalid UTF-8 / malformed CSV).

## Model: a CSV is a one-sheet positional grid

- **Load** parses the bytes with `csv-parser`'s new `parse_records` (the
  grid-level primitive — rows of fields, in order, header row included), then
  writes each field to sheet `Sheet1` at `(r+1, c+1)`. A field that parses as a
  number becomes `Number` (so `42` is numeric and `007` becomes `7`, as a
  spreadsheet import would); anything else is `Text`; a blank field is an empty
  cell. There is no header, formula, or type notion — a CSV is just a grid.
- **Save** writes the **first** sheet's used range as lines of delimited fields.
  Each cell is rendered as text: a formula as its **computed value**, a number
  without a trailing `.0`, a boolean as `TRUE`/`FALSE`, an error as its display
  text. Fields containing the delimiter, a `"`, or a newline are quoted per
  **RFC 4180** (wrapped in `"`, internal `"` doubled). Rows join with `\n`.

## Round-trip & interop

Numbers and text round-trip exactly (`load_csv(save_csv(wb))` preserves values);
RFC-4180 quoting round-trips (a field with a comma/quote/newline survives). The
output is exactly what Python's `csv` module (same RFC 4180) reads, and a
`csv → .xlsx` bridge test proves any-format-in / any-format-out.

## Limitations (documented + tested)

- **One sheet.** A CSV holds a single table; `save_csv` writes the first sheet
  and drops the rest — use `.xlsx` for multi-sheet.
- **No types/formulas/booleans-as-such.** Formulas save as their value; booleans
  save as `TRUE`/`FALSE` and reload as text; a numeric-looking string loads as a
  number (leading zeros lost) — the standard spreadsheet-CSV bargain.
- **Size = used-range area.** A CSV is dense and positional, so a sheet with a
  far-flung cell yields a correspondingly large CSV (inherent to the format,
  unlike the sparse `.xlsx` writer). A caller exposing `save_csv` to untrusted
  workbooks should bound the used range first.

## Non-goals

- Delimiter/encoding sniffing (the caller picks `,`/`\t`/…; UTF-8 only).
- Header-aware typing of columns (that lives in `sql-spreadsheet-source`, which
  reads the header row as SQL column names).
