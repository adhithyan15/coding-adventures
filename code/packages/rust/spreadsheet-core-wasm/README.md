# spreadsheet-core-wasm

A **string-in / JSON-out facade** over
[`spreadsheet-core`](../spreadsheet-core), shaped for a browser/WASM embedding.
It is the stable boundary a JavaScript host talks to: send an A1 address and a
raw cell string in, get JSON back. The engine stays typed and Rust-native;
this crate handles `A1 ⇄ CellAddress`, `raw string ⇄ CellValue`,
`CellValue ⇄ JSON`, and panic-safety.

It follows the repo's `macsyma-wasm` pattern: a pure, in-memory, panic-safe
facade. This crate is a normal workspace library — you build and test it with
`cargo test`, no WASM toolchain required. The thin `extern "C"` + linear-memory
ABI and the JavaScript loader that instantiates the compiled `.wasm` are a
separate layer on top.

## Where it sits

```text
  JS host (VisiCalc demo)
       │  set_cell("B6", "=SUM(B1:B5)")   ── strings in
       ▼
  spreadsheet-core-wasm   ← this crate (the JSON boundary)
       │
       ▼
  spreadsheet-core        ← cells, dependency graph, recalc, formulas
       │  delegates SUM/AVERAGE/… to
  statistics-core · math-core · financial-core · …
```

## API

`SpreadsheetSession` pins a single sheet (all the original VisiCalc, and the
web demos, need) and addresses cells by bare A1:

| method | in | out |
|---|---|---|
| `new()` | — | a session |
| `set_cell(a1, raw)` | `"B6"`, `"=SUM(B1:B5)"` | `{"ok":true}` / `{"ok":false,"error":…}` |
| `get_value(a1)` | `"B6"` | `{"kind":"number","value":46.0}` |
| `get_raw(a1)` | `"B6"` | `=SUM(B1:B5)` (the typed source) |
| `get_values()` | — | `{"B1":{…},"B6":{…}}` (all set cells) |

Raw strings are interpreted the way a spreadsheet does: a leading `=` is a
formula; `TRUE`/`FALSE` are booleans; a finite number is a number; anything
else is a text label; empty clears the cell. The **value JSON shape matches the
TypeScript engine's `CellValue` union exactly**, so the demo glue is identical
whichever engine backs it.

A cell has two faces — what you *typed* (`=SUM(B1:B5)`) and what it *shows*
(`46`). The engine owns the computed value; this facade keeps a small per-cell
`raw` map so the formula bar can be repopulated with the source.

### File open / save

The session also opens and saves real files, not just its own JSON snapshot —
via [`spreadsheet-io`](../spreadsheet-io):

| method | in | out |
|---|---|---|
| `load_xlsx_bytes` / `load_xls_bytes` / `load_csv_bytes` / `load_tsv_bytes` / `load_json_bytes` | file bytes | `bool` (opened?) |
| `save_xlsx_bytes` / `save_xls_bytes` / `save_csv_bytes` / `save_tsv_bytes` / `save_json_bytes` | — | file bytes |

An open reuses the snapshot path (load → serialize → `deserialize`), so it is
**undoable** and rebuilds the formula bar; a failed open (bad bytes / malformed
JSON) returns `false` and leaves the document untouched. `.xlsx` keeps live
formulas; `.xls`/CSV/TSV/JSON are lower-fidelity (one sheet, formulas flatten to
their value) per `spreadsheet-io`.

## Safety

- Text values are JSON-escaped (via `serde_json`), so a label like `a"b<c>`
  cannot break the JSON the host parses.
- Every mutating call is wrapped in `catch_unwind`: the engine is hardened
  against adversarial formulas, but a stray panic degrades to an error result
  instead of aborting the host / trapping the WASM module.
- An oversized range (`=SUM(A1:XFD1048576)`) surfaces as `#REF!`, inherited
  from the engine's range cap.

## Usage

```rust
use spreadsheet_core_wasm::SpreadsheetSession;

let mut s = SpreadsheetSession::new();
s.set_cell("B1", "15");
s.set_cell("B2", "8");
s.set_cell("B3", "=B1+B2");
assert_eq!(s.get_value("B3"), r#"{"kind":"number","value":23.0}"#);
assert_eq!(s.get_raw("B3"), "=B1+B2");
```
