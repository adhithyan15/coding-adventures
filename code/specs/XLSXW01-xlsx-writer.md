# XLSXW01 — `xlsx-writer`: emit a real `.xlsx` from a simple model

**Milestone C1 (write side), layer: SpreadsheetML.** This is the mirror of the
read-side [`spreadsheetml`](SML01-spreadsheetml.md) crate. It takes a small
in-memory workbook model and produces the bytes of a valid `.xlsx`, generating
the SpreadsheetML XML parts and packaging them via the generic
[`opc-writer`](OPCW01-opc-writer.md).

```text
  Rust model                xlsx-writer                    opc-writer         bytes
 ┌───────────┐   generate  ┌──────────────────┐  add_part ┌───────────┐  ZIP ┌────────┐
 │ Workbook  │  ─────────► │ workbook.xml      │ ────────►│ Package   │ ────►│ .xlsx  │
 │  Sheet    │             │ sheetN.xml        │           │ Writer    │      └────────┘
 │  cells    │             │ sharedStrings.xml │           └───────────┘
 └───────────┘             │ *.rels, [CT].xml  │
                           └──────────────────┘
```

## The `.xlsx` part tree we generate

```text
/                                    (the ZIP root)
├── [Content_Types].xml              content-type registry (opc-writer emits)
├── _rels/
│   └── .rels                        package → workbook (rId1)
└── xl/
    ├── workbook.xml                 <sheets>: name + sheetId + r:id per sheet
    ├── sharedStrings.xml            deduplicated text table (<sst>)
    ├── _rels/
    │   └── workbook.xml.rels        workbook → each sheetN.xml + sharedStrings
    └── worksheets/
        ├── sheet1.xml               <sheetData> rows/cells for sheet 1
        ├── sheet2.xml               …
        └── …
```

## The two normalizations, generated

The reader spec calls out two indirections that trip up newcomers. The writer
must *produce* both correctly.

### 1. Shared strings

Text is not stored inline in the cell; it is deduplicated into a single
**shared-string table**, and the cell stores an *index* with `t="s"`. The writer
builds the table as it walks cells: the first time a string appears it is
appended and gets the next index; repeats reuse that index. `<sst>` carries two
counts:

* `count` — total number of string-cell **references** (repeats included).
* `uniqueCount` — number of **distinct** strings (the table length).

```xml
<sst xmlns="…/spreadsheetml/2006/main" count="3" uniqueCount="2">
  <si><t>Q1</t></si>
  <si><t>Total</t></si>
</sst>
```

A cell then references index 0: `<c r="A1" t="s"><v>0</v></c>`.

### 2. `r:id` → part

`workbook.xml` names each sheet by a relationship id, and
`xl/_rels/workbook.xml.rels` maps that id to `worksheets/sheetN.xml`. The writer
assigns `rId1..rIdN` to the N sheets and `rId(N+1)` to `sharedStrings.xml`,
emitting the matching `.rels` entries.

## Cell XML by kind

| Model                     | Generated XML                                         |
|---------------------------|-------------------------------------------------------|
| `set_number("B1", 1000)`  | `<c r="B1"><v>1000</v></c>` (no `t` = number)         |
| `set_string("A1", "Q1")`  | `<c r="A1" t="s"><v>0</v></c>` (SST index)            |
| `set_formula("B2","SUM(B1:B1)",1000)` | `<c r="B2"><f>SUM(B1:B1)</f><v>1000</v></c>` |

The formula's `<f>` body is the formula text **without** a leading `=`; `<v>` is
the *cached* result (a courtesy for non-computing viewers). Our own evaluator
([`xlsx-eval`](SML03-formula-eval.md)) ignores the cached `<v>` and recomputes
from `<f>`, which is precisely what the round-trip test checks.

Cells within a `<row>` are emitted in column order; rows are emitted in row
order, each `<row r="N">` carrying its 1-based row number.

## Public API

```rust
pub struct Workbook { /* sheets in insertion order */ }
impl Workbook {
    pub fn new() -> Self;
    pub fn add_sheet(&mut self, name: &str) -> &mut Sheet;
}
pub struct Sheet { /* name + cells keyed by (row,col) */ }
impl Sheet {
    pub fn set_number(&mut self, a1: &str, n: f64);
    pub fn set_string(&mut self, a1: &str, s: &str);
    pub fn set_formula(&mut self, a1: &str, formula: &str, cached: f64);
}
pub fn write_xlsx(wb: &Workbook) -> Vec<u8>;   // -> .xlsx bytes
```

Setting a cell whose A1 reference does not parse is a silent no-op (the model is
caller-trusted; a bad ref is a caller bug, never a panic).

## Number formatting

Cell values are `f64`. Integers are written without a trailing `.0` (`1000`, not
`1000.0`) so the on-disk XML matches what Excel writes and what a human expects;
non-integers use Rust's shortest round-tripping `f64` formatting. `NaN`/`±∞` are
not representable in SpreadsheetML `<v>`; the writer emits `0` for them rather
than invalid XML (documented limitation — the model is trusted not to contain
them).

## The round-trip proof

The milestone's core proof writes a "Revenue" sheet — `A1="Q1"`, `B1=1000`,
`A2="Total"`, `B2=SUM(B1:B1)` (cached 1000) — to bytes, then reads them back
with **this repo's own readers**:

1. **Structural** — `spreadsheetml::open_workbook(&bytes)` reopens the sheet;
   we assert the sheet name, the string cells (`A1`→"Q1", `A2`→"Total"), the
   number cell (`B1`→1000), and that `B2` carries formula text `SUM(B1:B1)`.
2. **Recompute** — `xlsx-eval::open_and_evaluate(&bytes)` recomputes formulas
   from scratch (ignoring the cached `<v>`), and `computed_value(&core,
   "Revenue", "B2")` must equal `Number(1000.0)`.

Write → our own reader → correct values (including a formula that *recomputes*)
is the end-to-end demonstration that the bytes are a genuine `.xlsx`.

## Security / robustness

`#![forbid(unsafe_code)]`. All text (sheet names, string cells, formula text) is
XML-escaped totally. No `unwrap`/`expect`/`panic!` on paths reachable from an
empty workbook, empty sheet, unicode text, or special characters.
