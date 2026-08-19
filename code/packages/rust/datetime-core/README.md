# `datetime-core`

Excel / Lotus / R date and time functions, built on invariants chosen once.

Date math is the part of a spreadsheet's function catalog that lives and dies on
the representation you pick at the bottom. Get it wrong and every function above
inherits the mistake. This crate picks its invariants in `src/lib.rs` and
implements every public function in terms of them.

---

## The four invariants

1. **An instant** is a `wall_clock::Instant` — Unix-epoch `f64` seconds. Reading
   a clock is *dependency-injected*: no function here touches
   `std::time::SystemTime`. `now(clock)` and `today(clock)` take the clock as an
   argument, which is what makes `TODAY()`-dependent spreadsheet tests
   deterministic instead of "re-run it tomorrow and watch it fail."

2. **A calendar day** is a `Date` — `i32` days since `1970-01-01`. Negative
   values are dates before the epoch. This is the smallest representation that
   survives both the Excel quirks and the R/POSIXct edge cases.

3. **Civil ↔ serial conversion** uses Howard Hinnant's algorithm
   (`days_from_civil` / `civil_from_days`), exact for all Gregorian dates and
   tolerant of dates before year 1 — which Excel's own date model is not.

4. **Day-count conventions** for `yearfrac` follow the bond-market standard set:
   30/360-US (default), Actual/Actual, Actual/360, Actual/365, 30/360-European,
   selected by Excel's `basis` argument via `DayCount::from_basis`.

## Excel parity, not Excel mimicry

Both 1900 and 1904 epochs are supported (`from_excel_serial_1900` /
`from_excel_serial_1904` and their inverses) because real workbooks use both.
Where Excel has documented bugs, the crate's position is stated at the function
rather than silently reproduced or silently corrected — read the doc comment
before assuming which way a given edge case goes.

## The function catalog

| Area | Functions |
|------|-----------|
| Construction / parts | `Date::from_ymd`, `to_ymd`, `year`, `month`, `day`, `hour`, `minute`, `second`, `time` |
| Clock (injected) | `now`, `today`, `date_part_of`, `time_part_of` |
| Calendar facts | `is_leap_year`, `days_in_month`, `days_in_year`, `iso_weekday`, `weekday`, `isoweeknum` |
| Differences | `days`, `days360`, `datedif`, `yearfrac` |
| Arithmetic | `add_days`, `days_until`, `edate`, `eomonth` |
| Excel serials | `from_excel_serial_1900` / `to_excel_serial_1900`, and the 1904 pair |

Fallible operations return `Result<_, DateError>` rather than saturating or
panicking, so an out-of-range serial from a hostile or corrupt workbook is a
value the caller handles, not a crash.

## Where it sits

```
numeric-tower + r-vector + wall-clock
              │
        datetime-core   (this crate)
              │
   ┌──────────┼──────────────┬──────────────┐
financial-  number-      spreadsheet-    task-core
   core     format-core     core
```

## Testing

```sh
cargo test -p datetime-core -- --nocapture
```

## See also

- `wall-clock` — the injectable clock this crate reads time through.
- `spreadsheet-core` — the frontend that exposes these as worksheet functions.
