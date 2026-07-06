# SSIOJSON01 — `spreadsheet-io`: JSON (array-of-objects records) load & save

Extends `spreadsheet-io` (which unified `.xlsx`/`.xls`/CSV/TSV onto
`spreadsheet-core`) with **JSON**, so the same engine reads and writes the shape
almost every web API and data tool emits — a top-level array of record objects.
A JSON payload loaded this way is editable in VisiCalc, queryable with SQL
(`sql-spreadsheet-source`), and re-exportable as `.xlsx`/CSV. This continues
"read and process any tabular format, export any tabular format" onto the one
core.

## API (added)

```rust
pub fn load_json(bytes: &[u8]) -> Result<Workbook, IoError>;
pub fn save_json(wb: &Workbook) -> Vec<u8>;
```

`IoError` gains a `Json(String)` variant (invalid UTF-8 / malformed JSON / an
AST that can't be lowered to a value).

## Model: JSON has no single spreadsheet shape, so pick the common one

The canonical, round-tripping shape is a top-level **array of objects**
("records") — one object per row, one key per column:

```text
  [ {"region":"East","sales":200},        region | sales
    {"region":"West","sales":340} ]  ──▶    East  |  200
                                            West  |  340
```

- **Load** parses the bytes (panic-free — see below), then interprets the value:
  - **array of objects** → records: row 1 is the header (the **union** of keys
    across all records, in first-seen order, so a later record's new key still
    gets a column); each object is a data row, a field placed under its key's
    column; a missing key is a blank cell (records need not be uniform).
  - **array of arrays** → a positional grid `(r+1, c+1)`, no header row.
  - **array of scalars** → a single column.
  - **single object** → header + exactly one record row.
  - **top-level scalar** → a single cell.
  - A value that is itself an **object or array** (a nested subtree) has no flat
    cell, so it is stored as its **compact JSON text** — lossy for editing but
    faithful for display and re-export.
  - Scalar → cell mapping: JSON integer/float → `Number`, string → `Text`,
    `true`/`false` → `Boolean`, `null` → empty cell.
- **Save** is the inverse of the records shape: the **first** sheet's header row
  (`used_range.min_row`) supplies the keys, and each populated row below becomes
  one `{key: value}` object. Cell → JSON mapping: a whole-number `Number` → JSON
  integer (`200`, not `200.0`), a fractional/non-finite `Number` → float, `Text`
  → string, `Boolean` → `true`/`false`, empty → `null`, error → its display text
  (there is no JSON error literal). An empty or sheetless workbook yields `[]`.

## Panic-free parsing (untrusted input)

JSON here is **untrusted** (a file, a paste, eventually a browser upload), so
`load_json` uses `json-parser`'s new `try_parse_json` (which uses `json-lexer`'s
new `try_tokenize_json`): malformed bytes return `IoError::Json`, never a panic.
The pre-existing `parse_json`/`tokenize_json` (which panic) remain for callers
with pre-validated input.

## Sparse save (DoS-safe)

Unlike the dense CSV writer, `save_json` walks `populated_cells` **sparsely**, so
a sheet with one far-flung cell costs one record, not a used-range-area blow-up.

## Round-trip & interop

A records file round-trips (`load_json(save_json(wb))` preserves the table). The
output is standard JSON any parser accepts, and a `JSON → .xlsx` bridge test
proves any-format-in / any-format-out.

## Limitations (documented + tested)

- **One sheet.** A JSON records array is a single table; `save_json` writes the
  first sheet and drops the rest — use `.xlsx` for multi-sheet.
- **No formulas/types-as-such.** Formulas save as their computed value; a
  nested object/array in a cell round-trips as JSON *text*, not as structure.
- **Records shape on save.** `save_json` always emits the array-of-objects
  records shape; it does not attempt to reproduce a grid/scalar input shape.

## Non-goals

- JSON Lines / NDJSON, streaming, or schema inference beyond the header row.
- Header-aware column typing (that lives in `sql-spreadsheet-source`).
