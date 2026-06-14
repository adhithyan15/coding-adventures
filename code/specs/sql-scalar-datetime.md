# SQL Date/Time Scalar Functions

**Spec ID**: sql-scalar-datetime  
**Status**: Implemented (v0.7.0)  
**Package**: `sql-vm` — `scalar_functions.py`  
**Depends on**: sql-vm v0.6.x (scalar function registry already exists)

---

## 1. Motivation

SQLite ships six built-in date/time functions that cover virtually all
calendar arithmetic found in real applications:

```
DATE(timevalue [, modifier...])
TIME(timevalue [, modifier...])
DATETIME(timevalue [, modifier...])
JULIANDAY(timevalue [, modifier...])
UNIXEPOCH(timevalue [, modifier...])
STRFTIME(format, timevalue [, modifier...])
```

Without these functions queries like the following are impossible to express:

```sql
SELECT * FROM events WHERE DATE(created_at) = DATE('now')
SELECT STRFTIME('%Y-%m', created_at) AS month, COUNT(*) FROM logs GROUP BY month
SELECT UNIXEPOCH('now') - UNIXEPOCH(last_login) AS seconds_since_login FROM users
```

Additionally, SQLite's `MAX(a, b)` and `MIN(a, b)` with exactly two arguments
behave as scalar functions (return the greater/lesser value), distinct from the
single-argument aggregate forms.  These are added here alongside the date/time
work since both changes are VM-only.

---

## 2. Architecture: VM-only change

All scalar functions dispatch through the `_REGISTRY` dict in
`scalar_functions.py`.  The `CallScalar` bytecode instruction already exists
and already routes calls through `call(name, args)`.  No changes are needed
to the grammar, parser, planner, codegen, or IR.

```
SQL text
  → parser (unchanged)
  → planner: FunctionCall(name="date", args=[...])  (unchanged)
  → codegen: CallScalar(name="date", n_args=N)      (unchanged)
  → vm: scalar_functions.call("date", evaluated_args)   ← NEW
```

---

## 3. Time value input format

SQLite accepts five forms for the *timevalue* argument:

| Form | Example | Description |
|------|---------|-------------|
| `'now'` | `'now'` | Current UTC date and time |
| ISO-8601 date | `'2024-03-15'` | Date only; time defaults to `00:00:00` |
| ISO-8601 datetime | `'2024-03-15 14:30:00'` | Date and time |
| Julian Day number | `2460384.5` | Float counting days from noon Jan 1, 4713 BC |
| Unix timestamp | `1710507600` | Integer seconds since 1970-01-01 00:00:00 UTC |

Our implementation parses each of these forms and converts to a Python
`datetime.datetime` object (UTC) for internal processing, then formats the
result as required by the specific function.

---

## 4. Modifier list

Modifiers are optional trailing string arguments that shift the time value
before formatting.  They are applied in order, left to right.

### Offset modifiers

```
'+N days'     / '-N days'
'+N hours'    / '-N hours'
'+N minutes'  / '-N minutes'
'+N seconds'  / '-N seconds'
'+N months'   / '-N months'
'+N years'    / '-N years'
```

*N* is an integer or decimal number.  Days, hours, minutes, and seconds
are added directly to the `timedelta`.  Months and years adjust the calendar
date component (clamping the day to the last valid day of the resulting month
when necessary — e.g. Jan 31 + 1 month → Feb 28).

### Snap-to-start modifiers

```
'start of day'    → set time to 00:00:00
'start of month'  → set day to 1 and time to 00:00:00
'start of year'   → set month and day to 1, time to 00:00:00
```

### `'localtime'` and `'utc'`

`'localtime'` converts from UTC to the local system timezone.
`'utc'` converts from local time back to UTC (rarely needed after `'now'`).

### Unknown modifier

An unrecognised modifier string causes the function to return `NULL`,
matching SQLite's error-propagation behaviour for invalid modifiers.

---

## 5. Function specifications

### `DATE(timevalue [, modifier...])`

Returns an ISO-8601 date string (`YYYY-MM-DD`).

```sql
DATE('now')                    → '2024-03-15'
DATE('now', 'start of month')  → '2024-03-01'
DATE('2024-01-31', '+1 month') → '2024-02-29'   -- leap year clamp
DATE(NULL)                     → NULL
```

### `TIME(timevalue [, modifier...])`

Returns a time string (`HH:MM:SS`).

```sql
TIME('now')                        → '14:30:00'
TIME('2024-03-15 14:30:45.123')    → '14:30:45'
TIME('now', '+1 hour')             → '15:30:00'
TIME(NULL)                         → NULL
```

### `DATETIME(timevalue [, modifier...])`

Returns a combined datetime string (`YYYY-MM-DD HH:MM:SS`).

```sql
DATETIME('now')                       → '2024-03-15 14:30:00'
DATETIME('now', '-1 day')             → '2024-03-14 14:30:00'
DATETIME('now', 'start of year')      → '2024-01-01 00:00:00'
DATETIME(NULL)                        → NULL
```

### `JULIANDAY(timevalue [, modifier...])`

Returns the Julian Day Number as a floating-point value.  The Julian Day
Number is the continuous count of days since noon on January 1, 4713 BC
(proleptic Julian calendar).

```sql
JULIANDAY('2000-01-01')   → 2451544.5
JULIANDAY('now')          → 2460384.6      -- (example; varies)
JULIANDAY(NULL)           → NULL
```

**Conversion formula** (from Gregorian date to JDN):

```
A = (14 - month) / 12
Y = year + 4800 - A
M = month + 12*A - 3
JDN = day + (153*M + 2)/5 + 365*Y + Y/4 - Y/100 + Y/400 - 32045
JD  = JDN - 0.5 + (hour*3600 + minute*60 + second) / 86400
```

### `UNIXEPOCH(timevalue [, modifier...])`

Returns the number of seconds since 1970-01-01 00:00:00 UTC as an integer.

```sql
UNIXEPOCH('1970-01-01')   → 0
UNIXEPOCH('now')          → 1710507600    -- (example; varies)
UNIXEPOCH(NULL)           → NULL
```

### `STRFTIME(format, timevalue [, modifier...])`

Returns a string formatted according to *format*, using the same specifiers
as the C `strftime` function.  The most commonly used subset:

| Specifier | Meaning |
|-----------|---------|
| `%Y` | 4-digit year |
| `%m` | 2-digit month (01–12) |
| `%d` | 2-digit day (01–31) |
| `%H` | 2-digit hour, 24h (00–23) |
| `%M` | 2-digit minute (00–59) |
| `%S` | 2-digit second (00–59) |
| `%f` | Fractional seconds: `SS.SSS` (SQLite-style, 3 decimal places) |
| `%j` | Day of year (001–366) |
| `%w` | Day of week (0=Sunday … 6=Saturday) |
| `%W` | Week of year (00–53, Monday first day) |
| `%s` | Unix epoch seconds (integer as string) |
| `%J` | Julian Day number |
| `%%` | Literal `%` |

```sql
STRFTIME('%Y-%m', 'now')               → '2024-03'
STRFTIME('%s', '2000-01-01')           → '946684800'
STRFTIME('%Y-%m-%d', 'now', '-7 days') → '2024-03-08'   -- a week ago
STRFTIME(NULL, 'now')                  → NULL
STRFTIME('%Y', NULL)                   → NULL
```

---

## 6. Scalar `MAX(a, b)` and `MIN(a, b)` (two-argument form)

In SQLite, `MAX` and `MIN` with **two or more arguments** are scalar functions
that return the maximum/minimum of their arguments using SQLite's comparison
ordering.  With **one argument** they behave as aggregate functions (handled
separately by the `InitAgg`/`FinalizeAgg` opcodes).

The codegen distinguishes these because aggregate function calls are represented
as `AggregateExpr` AST nodes (routed to `InitAgg/FinalizeAgg`) while
`FunctionCall(name="max", args=[a, b])` with two arguments is routed to
`CallScalar`.

```sql
MAX(3, 5)          → 5
MAX('apple', 'fig') → 'fig'   -- lexicographic
MAX(1, NULL)        → 1       -- NULL is "less than everything" in scalar max
MIN(3, 5)          → 3
MIN(NULL, NULL)    → NULL
```

**NULL handling**: unlike most functions, scalar `MAX`/`MIN` treat `NULL` as
less than any non-null value (SQLite semantics).  If all arguments are `NULL`,
the result is `NULL`.

---

## 7. Implementation notes

### Time value parser (`_parse_timevalue`)

Central helper function that converts any accepted time value form to a
`datetime.datetime` (UTC-aware).  Called by all six date/time functions before
applying modifiers.  Returns `None` on parse failure (propagates as SQL `NULL`).

### Modifier application (`_apply_modifier`)

Applies one modifier string to a `datetime.datetime`.  Returns `None` for
unrecognised modifiers.  Applied in sequence by the outer function loop.

### `%f` in STRFTIME

SQLite's `%f` outputs `SS.SSS` (seconds with 3 decimal places), not Python's
`%f` which outputs microseconds.  The implementation maps `%f` manually before
delegating to `datetime.strftime`.

### Leap-year clamping for `'+N months'`

When adding N months would produce an invalid date (e.g. `2024-01-31 + 1
month` → Feb 31), clamp to the last day of the resulting month.  This matches
SQLite's documented behaviour.

---

## 8. Testing strategy

Tests live in `sql-vm/tests/test_scalar_functions.py` in a new
`TestDateTimeFunctions` class and `TestScalarMinMax` class.

### Date/time tests
- `DATE('now')` returns a string matching `YYYY-MM-DD`
- `TIME('now')` returns a string matching `HH:MM:SS`
- `DATETIME('now')` returns a string matching `YYYY-MM-DD HH:MM:SS`
- `JULIANDAY('2000-01-01')` → `2451544.5` (known constant)
- `UNIXEPOCH('1970-01-01')` → `0`
- `STRFTIME('%Y-%m-%d', '2024-03-15')` → `'2024-03-15'`
- `STRFTIME('%s', '2000-01-01')` → `'946684800'`
- `DATE('2024-01-31', '+1 month')` → `'2024-02-29'` (leap-year clamp)
- `DATE('2024-03-15', 'start of month')` → `'2024-03-01'`
- `DATETIME('2024-03-15 12:00:00', '+1 day', '-2 hours')` → compound modifiers
- NULL propagation for all six functions
- ISO-8601 string input, Julian Day float input, Unix integer input

### Scalar MAX/MIN tests
- `MAX(3, 5)` → `5`
- `MIN(3, 5)` → `3`
- `MAX('apple', 'fig')` → `'fig'`
- `MAX(1, NULL)` → `1`
- `MIN(NULL, NULL)` → `NULL`

---

## 9. Non-goals

- Timezone database (`'America/New_York'` modifier) — SQLite 3.38+ feature,
  deferred.
- Subsecond precision in `DATE`, `TIME`, `DATETIME` output (SQLite truncates
  to seconds) — already matched by this implementation.
- `TIMEDIFF()` (SQLite 3.43+) — deferred.
- `UNIXEPOCH('now', 'subsec')` — deferred.
