# SML02 — number formats, dates, merged cells, defined names

## Overview

This is milestone **M4** of the OOXML effort, extending the
`coding-adventures-spreadsheetml` crate built in [SML01](SML01-spreadsheetml.md).

M3 handed back the *bare stored value* of every cell. That is faithful to the
bytes but often useless to a human: on disk a date is the plain number `45292`,
a currency amount is `1234.5`, a percentage is `0.25`. M4 reads the **style**
attached to each cell — from `xl/styles.xml` — so the raw value can be
*interpreted* per its applied number format. The classic payoff: the number
`45292` is understood to be the date **2024-01-01**.

M4 is deliberately **backward-compatible**. `Value` and every M3 method
signature are unchanged; a date cell still holds `Value::Number(45292.0)`. The
number format is attached *alongside* the raw value (`Cell::number_format`),
never applied to it. So every M3 test passes verbatim, and a caller that ignores
formats sees exactly the M3 grid.

M4 also reads two small but frequently-needed structures the M3 reader skipped:
**merged cells** (on the worksheet) and **defined names** (on the workbook).

## The style-index chain (a third indirection)

SML01 documented two indirections (`r:id` → part, shared-string index → text).
Number formats add a third. A cell does **not** carry its format directly — it
carries a *style index* in an `s=` attribute:

```xml
<c r="A2" s="1"><v>45292</v></c>
```

That `s="1"` indexes into `<cellXfs>` ("cell eXtended Formats") in
`xl/styles.xml`. Each `<xf>` names a `numFmtId`:

```xml
<cellXfs count="4">
  <xf numFmtId="0"  xfId="0"/>   <!-- s=0 → General            -->
  <xf numFmtId="14" xfId="0"/>   <!-- s=1 → built-in date m/d/yyyy -->
  <xf numFmtId="164" xfId="0"/>  <!-- s=2 → custom "$"#,##0.00 -->
  <xf numFmtId="10" xfId="0"/>   <!-- s=3 → built-in 0.00% percent -->
</cellXfs>
```

So the full path a cell walks to learn its format is:

```text
<c s="1">  →  cellXfs[1]  →  numFmtId 14  →  format code "m/d/yyyy"  →  Date
```

The styles part itself is found the same way shared strings are: by scanning the
workbook's relationships for the `.../relationships/styles` type. A workbook with
no styles part is legal — every cell is then `General`.

## Built-in vs custom format ids

`numFmtId`s below **164** are *built-in*. Their meaning is fixed by ECMA-376 and
is **not** written into the file — everyone agrees id `14` is `m/d/yyyy`, id `9`
is `0%`, id `49` is `@` (text). Any reader must carry the table; ours is
`builtin_format_code(id)`. The commonly-seen ids:

| id | code       | id | code            | id | code           |
|----|------------|----|-----------------|----|----------------|
| 0  | `General`  | 14 | `m/d/yyyy`      | 21 | `h:mm:ss`      |
| 1  | `0`        | 15 | `d-mmm-yy`      | 22 | `m/d/yyyy h:mm`|
| 2  | `0.00`     | 16 | `d-mmm`         | 37-40 | accounting  |
| 3  | `#,##0`    | 17 | `mmm-yy`        | 45 | `mm:ss`        |
| 4  | `#,##0.00` | 18 | `h:mm AM/PM`    | 46 | `[h]:mm:ss`    |
| 9  | `0%`       | 19 | `h:mm:ss AM/PM` | 47 | `mmss.0`       |
| 10 | `0.00%`    | 20 | `h:mm`          | 49 | `@`            |

Ids **5-8, 23-36, 41-44, 50-58** are reserved / locale-defined; we return `None`
(no portable code) and classify them as `General`.

Ids **≥ 164** are *custom*: the producer defines them in `<numFmts>` with an
explicit `formatCode`:

```xml
<numFmts count="1">
  <numFmt numFmtId="164" formatCode="&quot;$&quot;#,##0.00"/>
</numFmts>
```

(The `&quot;` entities are decoded by the XML parser, so the code we store is
`"$"#,##0.00`.)

## Classification — `NumberFormatKind`

Each format code is bucketed into a coarse kind the caller usually wants:

```
General | Number | Date | Time | DateTime | Percent | Currency | Text | Other
```

For built-in ids the kind comes from the id (via its code). For custom codes we
**infer** it from the code's tokens, scanning left-to-right while honoring the
three OOXML "literal" contexts so literal text never triggers a false positive:

- `"..."` — a quoted literal (shown verbatim);
- `\x` — a single escaped literal char;
- `[...]` — a directive (`[Red]` colour, `[>100]` condition, `[$€-407]`
  currency/locale, `[h]` elapsed-time).

The rules, in priority order:

1. exactly `General` → `General`; exactly `@` → `Text`.
2. a date field (`y`, or a `d`/`m` read as a date) and/or a time field (`h`,
   `s`, or an `m` read as a minute) → `Date` / `Time` / `DateTime`.
3. else `%` → `Percent`.
4. else a currency signal (`$ € £ ¥ ₹ ₽ ₩ ¢`, or a `[$...]` directive) →
   `Currency`. A currency symbol inside a quoted literal (`"$"#,##0.00`) still
   counts — that is how the common currency format writes its sign.
5. else `@` anywhere → `Text`.
6. else digit placeholders (`0 # ?`) → `Number`.
7. else → `Other`.

### The `m` ambiguity

`m` is **month** in a date context but **minute** in a time context. Excel
disambiguates positionally. We approximate: an `m` is a *minute* only when the
code also contains a clock field (`h`/`s`) **and** no `y`/`d` date field has been
seen; otherwise it is a month. This matches every built-in code and the common
custom codes (`m/d/yyyy h:mm`, `h:mm:ss`, `[h]:mm:ss`, `mm:ss`).

## Dates — the 1900 date system and the leap-year bug

Excel stores a date as a **serial**: the count of days since an epoch. In the
default **1900 date system**, serial `1` is `1900-01-01`. The naive reading puts
serial `0` at `1899-12-31`, but Excel deliberately includes a **non-existent**
`1900-02-29`: 1900 was *not* a leap year (÷100 but not ÷400), yet Lotus 1-2-3
pretended it was and Excel copied the bug for compatibility. So for serials ≥ 60
the calendar is off by one day from reality.

We reproduce this **exactly** by anchoring serial `0` at the fictitious
`1899-12-30` and adding days on a real proleptic Gregorian calendar, mapping
serial 60 itself to the phantom `1900-02-29`:

| serial | renders as   | note                                   |
|--------|--------------|----------------------------------------|
| 1      | 1900-01-01   | the documented epoch                   |
| 59     | 1900-02-28   | last day before the phantom            |
| 60     | 1900-02-29   | **does not exist** — Excel's fake day  |
| 61     | 1900-03-01   | real calendar resumes                  |
| 25569  | 1970-01-01   | the classic Unix-epoch serial          |
| 45292  | 2024-01-01   | the headline example                   |

Rendering serial 60 as `1900-02-29` is intentional: it is what Excel shows, and
round-trip fidelity beats calendar correctness for a *reader*.

`serial_to_date` returns ISO `YYYY-MM-DD`; `serial_to_datetime` adds the
fractional time-of-day as `YYYY-MM-DDTHH:MM:SS` (fraction `0.5` = noon).

## Merged cells

A worksheet's `<mergeCells>` lists rectangular spans:

```xml
<mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells>
```

We parse each `ref` (via the M3 `parse_a1_ref`) into a `CellRange { start, end }`
of `(col, row)` pairs, both 1-based, and expose them via `Sheet::merged_ranges()`.

## Defined names

A workbook's `<definedNames>` lists named ranges / references:

```xml
<definedNames><definedName name="TaxRate">Report!$B$4</definedName></definedNames>
```

We expose them as `(name, reference)` pairs via `Workbook::defined_names()`,
keeping the reference text verbatim (`"Report!$B$4"`). We do **not** evaluate or
resolve the reference — that is a formula-engine concern.

## Public API (additions over SML01)

```rust
// On Cell (new field + methods; existing fields unchanged):
pub struct Cell {
    pub reference: String,
    pub value: Value,                       // UNCHANGED — a date is still Number(45292.0)
    pub formula: Option<String>,
    pub number_format: Option<NumberFormat>, // NEW: None if unstyled/General/out-of-range
}
impl Cell {
    pub fn format_kind(&self) -> NumberFormatKind; // General if no format
    pub fn as_date(&self) -> Option<String>;        // ISO YYYY-MM-DD for date cells
    pub fn as_datetime(&self) -> Option<String>;    // ISO YYYY-MM-DDTHH:MM:SS
    pub fn formatted(&self) -> String;              // pragmatic renderer (see below)
}

pub struct NumberFormat { pub id: u32, pub code: String, pub kind: NumberFormatKind }
pub enum NumberFormatKind {
    General, Number, Date, Time, DateTime, Percent, Currency, Text, Other,
}

pub fn builtin_format_code(id: u32) -> Option<&'static str>;
pub fn classify_id(id: u32) -> NumberFormatKind;
pub fn classify_format_code(code: &str) -> NumberFormatKind;
pub fn serial_to_date(serial: f64) -> Option<String>;
pub fn serial_to_datetime(serial: f64) -> Option<String>;
pub const FIRST_CUSTOM_FORMAT_ID: u32 = 164;

pub struct CellRange { pub start: (u32, u32), pub end: (u32, u32) }
impl CellRange { pub fn parse(range: &str) -> Option<CellRange>; }

// On Sheet:
impl Sheet { pub fn merged_ranges(&self) -> &[CellRange]; }
// On Workbook:
impl Workbook { pub fn defined_names(&self) -> &[(String, String)]; }

// StyleTable is exposed so tests (and advanced callers) can resolve manually:
pub struct StyleTable { /* custom-code map + ordered cellXfs */ }
impl StyleTable {
    pub fn empty() -> Self;
    pub fn from_root(root: &XmlElement) -> Self;
    pub fn format_for(&self, style_index: Option<u32>) -> Option<NumberFormat>;
}
```

### `formatted()` is pragmatic, not a full engine

A complete OOXML number-format renderer (grouping, decimal places, conditional
sections, colour, currency symbols) is **out of scope**. `formatted()` gives a
reasonable string:

- **Date / DateTime** — the exact ISO string. *This* is the part that must be
  exact, and it is.
- **Percent** — the stored fraction ×100 with a `%` (`0.25` → `"25%"`); the
  code's decimal count is not honoured.
- **Currency** — the raw number as a plain string; the symbol/grouping is **not**
  synthesized (documented limitation).
- **Everything else** — the value's natural string form.

## `NumberFormat` resolution — the `None` cases

`Cell::number_format` is `None` (and `format_kind()` is `General`) when:

- the cell has **no** `s=` attribute; or
- `s=` is **out of range** for `cellXfs` (a malformed style ref — we degrade
  gracefully, never panic or error); or
- the resolved format is **`General`** (id 0) — an unstyled number, kept `None`
  so M3 behaviour is preserved exactly.

## End-to-end payoff

Opening the M4 fixture (one sheet "Report": `A1`/`B1` headers "Date"/"Amount"
merged `A1:B1`; `A2` = `45292` styled numFmtId 14; `B2` = `1234.5` styled custom
numFmtId 164 `"$"#,##0.00`; `A4` = "Rate"; `B4` = `0.25` styled numFmtId 10;
definedName `TaxRate = Report!$B$4`) yields:

- `cell("A2").value == Number(45292.0)` **and** `cell("A2").as_date() == Some("2024-01-01")`, `format_kind() == Date`;
- `cell("B2").number_format` = id `164`, code `"$"#,##0.00`, kind `Currency`; value still `Number(1234.5)`;
- `cell("B4").format_kind() == Percent` (id 10); value `Number(0.25)`; `formatted() == "25%"`;
- `merged_ranges() == [A1:B1]`;
- `defined_names()` contains `("TaxRate", "Report!$B$4")`.

## Non-goals / deferred (still)

- A full number-format **renderer** (grouping, currency symbols, conditional
  sections, colours).
- Column widths / row heights, conditional formatting, cell borders/fills.
- Formula **evaluation** — formulas remain text + cached value.
- The **1904** date system (Mac legacy) — we implement the default 1900 system.
