# SSIO01 — `spreadsheet-io`: one spreadsheet core, many file formats

## Why this crate exists

Before this crate, the repo had **five disconnected in-memory spreadsheet
models** that could not talk to each other:

| Crate | Role | Its own `Workbook`/`Cell` types |
|-------|------|---------------------------------|
| `spreadsheet-core` | the live **engine** VisiCalc computes on | `Workbook` + `CellValue`/`CellContent` (+ formula AST) |
| `coding-adventures-spreadsheetml` | `.xlsx` **reader** | `Workbook`/`Sheet`/`Cell`/`Value` |
| `xls` | `.xls` **reader** | `Workbook`/`Sheet`/`Cell`/`CellValue` |
| `coding-adventures-xlsx-writer` | `.xlsx` **writer** | `Workbook`/`Sheet`/`CellData` |
| `xls-writer` | `.xls` **writer** | `Workbook`/`Sheet` |

Every VisiCalc front-end (web/WASM, native/C-ABI, Android/JNI — 14 variants in
all) already computes on **one** of those models: `spreadsheet-core`. So the
unification is obvious: **make `spreadsheet-core::Workbook` the single hub**, and
give it one adapter layer that converts every file format to and from it.

```
  .xlsx ──load_xlsx──┐                              ┌──save_xlsx──▶ .xlsx
                     ├─▶  spreadsheet-core::Workbook ─┤
  .xls  ──load_xls───┘     (THE model; VisiCalc      └──save_xls───▶ .xls
                            computes here)
                                  ▲  ▲
                                  │  └── every VisiCalc front-end, via the
                                  │       SpreadsheetSession facade (later PRs)
                                  └───── formulas, values, formats all live here
```

`spreadsheet-io` is that adapter. It is the **only** crate that depends on both
the engine and the file-format crates; the engine stays pure (it does not know
`.xlsx` exists), and each reader/writer stays a focused, faithful codec.

## Public API

```rust
/// Load a .xlsx file's bytes into a live engine workbook. Formulas are
/// installed as formulas (not flattened to values) and recomputed, so the
/// result is immediately editable and a later save preserves them.
pub fn load_xlsx(bytes: &[u8]) -> Result<Workbook, IoError>;

/// Serialize a live engine workbook to .xlsx bytes. Formula cells are written
/// as `<f>` + cached `<v>`; literal cells as typed values.
pub fn save_xlsx(wb: &Workbook) -> Vec<u8>;
```

where `Workbook` is re-exported from `spreadsheet-core`, and

```rust
pub enum IoError {
    /// The .xlsx bytes could not be parsed / evaluated (bad zip, XML, refs…).
    Xlsx(String),
}
```

Later PRs add `load_xls` / `save_xls` (SSIO02) and the `SpreadsheetSession`
wiring + WASM export that puts open/save buttons in the VisiCalc apps.

## Load: `.xlsx` → engine

`load_xlsx` is a thin, honest wrapper over the existing read stack:

```
bytes ─▶ spreadsheetml::open_workbook  (zip → xml → OPC → SpreadsheetML cells)
      ─▶ xlsx-eval::evaluate_workbook  (installs formulas via set_formula,
                                        literals via set_value, recalc_all)
      ─▶ spreadsheet-core::Workbook
```

`xlsx-eval` already does exactly the right thing — it feeds each `<f>` body to
the engine's parser (so `=SUM(A1:D1)` is a live formula, not a frozen number)
and, on a parse failure, falls back to the cached value while recording a
diagnostic. We surface any open/parse failure as `IoError::Xlsx`.

## Save: engine → `.xlsx`

Driven entirely off the unified core model, using three read accessors:

- `used_range(sheet)` — the populated bounding box (skips empty cells).
- `cell_is_formula(sheet, addr)` — **new** in `spreadsheet-core` (SSIO01); the
  writer must distinguish a formula from a literal, and neither `get_value` nor
  `cell_source_text` alone can (a formula's text and a literal's canonical
  string are both just strings; the `=` prefix is unreliable).
- `cell_source_text(sheet, addr)` — the formula body; `get_value` — its cached
  result.

Per cell in the used range:

| Cell in engine | Written to `.xlsx` |
|----------------|--------------------|
| formula, cached `Number(n)` | `<f>body</f><v>n</v>` (leading `=` stripped) |
| formula, cached `Text`/`Bool`/`Error` | the **computed value** as a literal (see limitation) |
| literal `Number(n)` | numeric `<v>` |
| literal `Text(s)` | shared-string `<v>` |
| literal `Bool(b)` | numeric `1`/`0` (see limitation) |
| literal `Error(e)` | its display text as a string |
| empty | omitted |

### Known fidelity limitations (documented, not hidden)

The current `.xlsx` **writer**'s cell model (`Number | Text | Formula{formula,
cached: f64}`) cannot express everything the engine holds:

1. **Formulas with non-numeric results** — the writer's cached slot is an `f64`.
   A formula like `=IF(A1>0,"y","n")` is written as its computed *string* (a
   literal), losing the formula. The value round-trips; the formula does not.
2. **Booleans** — no `t="b"`; written as `1`/`0`. A `TRUE` reloads as `Number(1)`.
3. **Number formats** — not yet emitted (a later PR extends the writer + this
   bridge to carry `get_format` codes through `styles.xml`).

Numbers, text, and numeric-result formulas — the overwhelming common case, and
the whole of a VisiCalc-authored sheet — round-trip exactly. The round-trip test
below asserts that.

## Round-trip contract

The crate's headline test builds a workbook in the engine (numbers, strings, and
a live `=SUM` formula across two sheets), then asserts:

```
wb  ──save_xlsx──▶ bytes ──load_xlsx──▶ wb'
assert:  for every populated cell,  value(wb) == value(wb')
         and every formula cell is still a formula in wb'
         and re-saving wb' yields byte-identical output (idempotent)
```

An independent cross-check re-opens the same bytes with **openpyxl** (a real,
third-party spreadsheet library) to prove the output is not merely
self-consistent but a genuine `.xlsx` other tools accept — the same
belt-and-suspenders standard the writer crates hold themselves to.

## Non-goals (this PR)

- `.xls` load/save — SSIO02.
- Number-format / styles round-trip — later.
- CSV/ODS/other formats — the adapter shape generalizes; add codecs as needed.
- Front-end wiring (buttons in the apps) — SSIO03–05.
