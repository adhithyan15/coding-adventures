# SML01 — SpreadsheetML workbook reader

## Overview

This is milestone **M3** of the OOXML effort. It builds a Rust crate,
`coding-adventures-spreadsheetml`, that reads the bytes of an `.xlsx` file and
hands back a **typed cell grid**: a workbook → sheets → cells model where every
populated cell carries a decoded value (number, text, boolean, error, or
blank) and, where present, the formula text plus its cached result.

Where the layers below stop:

```text
raw bytes (.xlsx)
      |
      v
zip crate (M0)        → ZIP members: name → bytes
      |
      v
xml-parser (M1)       → XmlDocument (namespaces resolved, entities decoded)
      |
      v
opc crate (M2)        → Package  (parts, content types, relationships)
      |                     - main_document_part() → "/xl/workbook.xml"
      |                     - resolve(part, "rId1") → "/xl/worksheets/sheet1.xml"
      v
spreadsheetml (M3)    → Workbook / Sheet / Cell / Value   (THIS crate)
```

OPC knows "here is a bag of named parts and how they link." It does **not**
know that `/xl/workbook.xml` is a workbook with sheets, that a `<c t="s">` is a
*shared string* whose `<v>` is an index (not a value), or that a `<f>` is a
formula. Teaching the reader those SpreadsheetML-specific meanings is exactly
this milestone's job.

Deliberately **out of scope** (deferred to M4): styles, number formats, and
date/time interpretation. A cell that displays as `$1,000.00` or `2024-01-31`
is stored on disk as the bare number `1000` or the serial `45322`; this crate
returns the raw number and leaves formatting to M4. Formulas are returned as
text plus their *cached* value — we do **not** evaluate them.

## The two indirections a newcomer must understand

An `.xlsx` is deliberately *normalized* on disk, the way a relational database
is. Two indirections trip up everyone who reads their first spreadsheet.

### 1. The r:id → part indirection (which file is this sheet?)

The workbook does not say "sheet Revenue lives in `worksheets/sheet1.xml`."
It says:

```xml
<sheet name="Revenue" sheetId="1" r:id="rId1"/>
```

That `rId1` is a *relationship id*, not a path. To turn it into a part name you
must consult a **separate** `.rels` file (`/xl/_rels/workbook.xml.rels`) and
resolve `rId1` to its target. The OPC layer already does that dereference:

```rust
opc.resolve("/xl/workbook.xml", "rId1")  // → Some("/xl/worksheets/sheet1.xml")
```

So the sheet order and names come from `workbook.xml`, but the *bytes* of each
sheet come from a part you reach only by resolving its r:id. We read the name +
r:id from `<sheet>`, then ask OPC where the r:id points.

### 2. The shared-string indirection (why is this cell just "0"?)

Text is deduplicated across the whole workbook into one **shared string table**
(`/xl/sharedStrings.xml`). A text cell does **not** store its text inline; it
stores an *index* into that table and flags itself with `t="s"` ("the value is
a shared string"):

```xml
<c r="A1" t="s"><v>0</v></c>   <!-- value is sharedStrings[0], e.g. "Q1" -->
```

So `<v>0</v>` under `t="s"` means "shared string #0", not the number zero. We
find the shared-strings part by scanning the workbook's relationships for the
`sharedStrings` relationship type, parse it into a `Vec<String>`, and dereference
each `t="s"` cell's `<v>` index into it. A missing sharedStrings part is legal
(a workbook with no text cells) — we treat the table as empty.

A shared-string entry `<si>` is **either** a single `<t>text</t>` **or** rich
text made of multiple `<r><t>…</t></r>` runs (bold/italic runs sharing one
logical string). Either way its *string value* is the concatenation of all
descendant `<t>` text in document order, which is exactly what the xml-parser's
`text_content()` returns on the `<si>` element. So we never special-case rich
text — we just take `text_content()` of each `<si>`.

## Cell types (the `t` attribute)

Each `<c>` optionally carries a type attribute `t`. The decode table:

| `t`          | where the value is | meaning                                    |
|--------------|--------------------|--------------------------------------------|
| absent / `n` | `<v>` text         | **number** — parse `<v>` as `f64`          |
| `s`          | `<v>` = index      | **shared string** — `sharedStrings[index]` |
| `str`        | `<v>` text         | **formula string result** — literal string |
| `inlineStr`  | `<is>` child       | **inline string** — `text_content()` of `<is>` |
| `b`          | `<v>` = `1`/`0`    | **boolean**                                |
| `e`          | `<v>` text         | **error** — e.g. `#DIV/0!`                  |

Orthogonally, a `<c>` may contain an `<f>` child (the formula text). When it
does, the cell *has a formula*: we keep the formula text **and** the cached
value from `<v>` (or `<is>`). We never evaluate — evaluation is a later
milestone. A `<c>` with no `<v>`, no `<is>`, and no `<f>` is **empty/blank**.

Concretely, `B2` in the fixture:

```xml
<c r="B2"><f>SUM(B1:B1)</f><v>1000</v></c>
```

decodes to `Cell { reference: "B2", value: Number(1000.0),
formula: Some("SUM(B1:B1)") }`.

## A1 references

Cells and rows carry A1-style references: `<c r="B2">`. `B2` means column `B`
(the 2nd column) and row `2`. The column letters are **base-26 bijective**
(`A`=1 … `Z`=26, `AA`=27, `AB`=28 …) — note there is no "zero digit", so it is
*bijective* base-26, not ordinary base-26. We parse `"B2"` into `(col, row) =
(2, 2)`, both **1-based**. This helper is exposed (`parse_a1_ref`) because M4
and any CLI need it. Examples: `A1 → (1,1)`, `B2 → (2,2)`, `Z1 → (26,1)`,
`AA10 → (27,10)`.

## Public API

```rust
pub fn open_workbook(bytes: &[u8]) -> Result<Workbook, XlsxError>;

pub struct Workbook { /* sheets in workbook order */ }
impl Workbook {
    pub fn sheet_names(&self) -> Vec<String>;
    pub fn sheet_by_name(&self, name: &str) -> Option<&Sheet>;
    pub fn sheets(&self) -> &[Sheet];
}

pub struct Sheet { pub name: String, /* cells keyed by (col,row) */ }
impl Sheet {
    pub fn cell(&self, a1: &str) -> Option<&Cell>;
    pub fn cells(&self) -> impl Iterator<Item = &Cell>; // populated cells, row-major
}

pub struct Cell {
    pub reference: String,      // A1, e.g. "B2"
    pub value: Value,
    pub formula: Option<String>,
}

pub enum Value {
    Number(f64),
    Text(String),   // shared strings AND inline strings both surface here
    Bool(bool),
    Error(String),
    Empty,
}

pub fn parse_a1_ref(a1: &str) -> Option<(u32, u32)>; // (col, row), both 1-based

pub enum XlsxError {
    Opc(OpcError),          // the bytes were not a readable package
    MissingWorkbook,        // no /xl/workbook.xml main part
    NotUtf8(String),        // a part was not valid UTF-8
    MalformedXml(String),   // a part failed to parse as XML
    MissingSheetPart(String), // a <sheet r:id> did not resolve to a part
    BadSharedStringIndex(usize), // a t="s" cell's index was out of range
}
```

`Value` implements `PartialEq` so tests can assert `== Text("Q1")`, etc.
Shared strings and inline strings both surface as `Text` — the caller should
not have to care about the storage indirection.

## End-to-end payoff

Opening the fixture `.xlsx` (one sheet "Revenue": `A1`=shared "Q1", `B1`=1000,
`A2`=shared "Total", `B2`=`SUM(B1:B1)` cached 1000) yields:

- `sheet_names() == ["Revenue"]`
- `cell("A1").value == Text("Q1")`, `cell("B1").value == Number(1000.0)`
- `cell("A2").value == Text("Total")`
- `cell("B2").formula == Some("SUM(B1:B1)")` **and** `cell("B2").value == Number(1000.0)`

That is the whole point of M3: from raw compressed bytes to a typed grid, with
the r:id and shared-string indirections resolved for you.

## Non-goals / deferred

- **M4** — styles, number formats, date/time interpretation, merged cells, and
  defined names are implemented in milestone **M4**; see
  [`SML02-number-formats.md`](SML02-number-formats.md). Column widths remain
  deferred.
- Formula **evaluation** — we return formula text + cached value only.
- Writing `.xlsx` — this is a read-only reader.
