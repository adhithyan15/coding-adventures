# number-format-core

Pure-Rust **numeric format codes → display strings**. Turns a computed number
into the text a spreadsheet cell should *show*, given an Excel / Lotus 1-2-3-style
format code like `0.00`, `#,##0`, `0%`, or `$#,##0.00;(#,##0.00)`.

## Where it sits in the stack

A Layer-1 core (sibling of `text-core`, `statistics-core`, `datetime-core`, …):
pure, no-I/O, no-`unsafe`, WASM-compatible. The spreadsheet engine
(`spreadsheet-core`) computes a cell's *value*; this crate decides how that value
*reads*. It's frontend-agnostic, so the web/SwiftUI/Qt/Flutter/Compose/XAML
VisiCalc demos — and any chart axis renderer — can share one implementation of
cell formatting instead of each re-deriving thousands-grouping and rounding.

## What a format code means

A code is 1–3 `;`-separated **sections**, chosen by the value's sign:

| sections        | positive | negative          | zero   |
|-----------------|----------|-------------------|--------|
| `pos`           | `pos`    | `pos` with `-`    | `pos`  |
| `pos;neg`       | `pos`    | `neg` (abs value) | `pos`  |
| `pos;neg;zero`  | `pos`    | `neg` (abs value) | `zero` |

Within a section:

| token | meaning |
|-------|---------|
| `0`   | digit placeholder — always shown (zero-padded) |
| `#`   | digit placeholder — shown only if significant |
| `?`   | like `#` (space-padding approximated as `#` in v1) |
| `.`   | decimal point (at most one per section) |
| `,`   | *between digits* → thousands grouping; *trailing* → ÷1000 each |
| `%`   | scale ×100 and show a literal `%` |
| other | a literal (`$`, `(`, `)`, spaces…); `\x` and `"…"` escape |

## Usage

```rust
use number_format_core::{format_number, NumberFormat};

// One-shot:
assert_eq!(format_number(1234.5, "#,##0.00"), "1,234.50");
assert_eq!(format_number(0.0734, "0.0%"),     "7.3%");
assert_eq!(format_number(-12.0,  "$#,##0.00;($#,##0.00)"), "($12.00)");
assert_eq!(format_number(2_600_000.0, "#,##0,, \"M\""),    "3 M");
assert_eq!(format_number(42.0,   "General"), "42");

// Parse once, format many cells:
let fmt = NumberFormat::parse("#,##0.00").unwrap();
assert_eq!(fmt.apply(1.0),     "1.00");
assert_eq!(fmt.apply(-1234.5), "-1,234.50");
```

A malformed code makes `format_number` fall back to the shortest representation
(it never panics); `NumberFormat::parse` surfaces the error if you want it.

## Date/time codes

A code that uses date/time field letters (`y`/`m`/`d`/`h`/`s`, outside quotes)
is applied to the value as an **Excel-1900 serial date** — the integer part is
the day count, the fraction is the time of day. Serial decomposition is
delegated to `datetime-core`.

```rust
// `45292.0` is 2024-01-01; `+ 0.5` is noon.
assert_eq!(format_number(45292.0,  "yyyy-mm-dd"),  "2024-01-01");
assert_eq!(format_number(45292.0,  "d-mmm-yyyy"),  "1-Jan-2024");
assert_eq!(format_number(45292.5,  "h:mm AM/PM"),  "12:00 PM");
```

Tokens: `yyyy`/`yy`, `mmmm`/`mmm`/`mm`/`m` (month), `dddd`/`ddd`/`dd`/`d` (day &
weekday name), `hh`/`h`, `mm`/`m` (minute), `ss`/`s`, `AM/PM`, `A/P`. The `m`
overload resolves by context — minutes next to an hours/seconds token, month
otherwise. A serial outside the representable range renders `######`.

## Scope

Numeric and date/time formats. **Documented follow-ups:** scientific notation
(`0.00E+00`), fractions (`# ?/?`), `[Color]` prefixes, `[>100]`-style
conditional sections, and the text (4th) section.

Rounding uses the standard library's correctly-rounded decimal formatting
(ties-to-even); this differs from Excel's ties-away-from-zero only on exact
ties, which are rare with `f64`.

## Build & test

```bash
cargo test -p number-format-core
```

15 unit tests + a doctest cover General, fixed decimals + zero-padding, optional
`#` placeholders, thousands grouping, percent, trailing-comma scaling, currency
prefix + literal suffix, negative-with/without-section, three-section zero
routing, rounding carry through grouping, and the malformed-code fallback.
