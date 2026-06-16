# Changelog

## 0.2.0

**Date/time format codes.** A code whose first section uses date/time field
letters (`y`/`m`/`d`/`h`/`s` outside quotes) is now parsed as a date/time format
and applied to the value as an Excel-1900 serial date (integer = days, fraction
= time of day). Numeric formats are unchanged.

- Tokens: `yyyy`/`yy`, `mmmm`/`mmm`/`mm`/`m` (month), `dddd`/`ddd`/`dd`/`d` (day &
  weekday name), `hh`/`h`, `mm`/`m` (minute), `ss`/`s`, `AM/PM`, `A/P`, plus
  separators and `\`/`"…"` literals.
- The `m`-overload is resolved by context: minutes when next to an hours or
  seconds token (`hh:mm`, `mm:ss`), month otherwise (`yyyy-mm`).
- 12-hour clock auto-selected when `AM/PM` or `A/P` is present.
- Serial decomposition (serial → Y/M/D, time components, weekday) delegates to
  `datetime-core` (new path dependency). A serial outside the representable
  range renders `######`, as a spreadsheet does.
- Internals: `NumberFormat` now holds a `FormatKind` (General / Numeric /
  DateTime); the numeric path is unchanged (all 16 numeric tests still green).
- 5 new tests (date fields, 24h/12h time, the `m` month-vs-minute rule, the
  out-of-range `######`, and a quoted-letter-stays-numeric case).

## 0.1.0

Initial release — a pure-Rust Layer-1 core that formats numbers per Excel /
Lotus 1-2-3-style format codes.

- `NumberFormat::parse(code) -> Result<NumberFormat, FormatError>`: parse a
  format code into a reusable formatter. Handles 1–3 `;`-separated sections
  (positive / negative / zero), digit placeholders (`0`, `#`, `?`), the decimal
  point, thousands grouping (`,` between digits), trailing-comma scaling (each
  trailing `,` ÷1000), percent (`%` scales ×100 and prints), and literal
  prefixes/suffixes including `\x` and `"…"` escapes. `General` / empty → the
  shortest round-tripping representation.
- `NumberFormat::apply(value) -> String` and the `format_number(value, code)`
  convenience (which falls back to the shortest representation on a malformed
  code rather than erroring, so a host render path never panics).
- `FormatError`: `MultipleDecimalPoints`, `TooManySections`.
- Non-finite guard: `NaN` → `"NaN"`, ±∞ → `"∞"` / `"-∞"` (never panics).
- Adversarial-input guard: the fractional precision handed to the std formatter
  is capped at `MAX_FRACTION_DIGITS` (340) — a format code with ≥ 65 536
  fractional placeholders would otherwise overflow the formatter's internal
  `u16` precision and panic. Required `0` placeholders still pad out to the
  demanded width. Keeps the documented "never panics on a bad format" promise.
- Rounding via the standard library's correctly-rounded decimal formatting
  (ties-to-even); documented as differing from Excel's ties-away only on exact
  ties.
- 15 unit tests + a doctest. `forbid(unsafe_code)`, no I/O, WASM-compatible.

### Scope

Numeric formats only. Out of scope (documented follow-ups): date/time codes,
scientific notation, fractions, `[Color]` prefixes, `[>condition]` conditional
sections, and the text (4th) section.
