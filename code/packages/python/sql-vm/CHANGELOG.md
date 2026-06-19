# Changelog

## 1.61.0 — 2026-06-19

### Added

- **`_do_create_trigger` honours `if_not_exists`** — when the backend raises
  `TriggerAlreadyExists` and the IR instruction carries
  `if_not_exists=True`, the VM now swallows the error silently (matching
  SQLite's semantics for `CREATE TRIGGER IF NOT EXISTS`).  Without the
  flag the error is still translated and re-raised as before.

## 1.60.0 — 2026-06-16

### Fixed

- **RANGE mode peer-group expansion** — cumulative window functions
  (``SUM``, ``COUNT``, ``AVG``, etc.) with ``ORDER BY`` now correctly
  expand ``CURRENT ROW`` to the full *peer group* when the frame mode
  is ``RANGE`` (the SQL standard default).

  Previously ``_frame_slice`` applied ``ROWS`` physical-position
  semantics even for the default ``RANGE BETWEEN UNBOUNDED PRECEDING
  AND CURRENT ROW`` frame, so tied ``ORDER BY`` values produced wrong
  cumulative totals::

      -- data: (1,2,2,3)
      SELECT a, COUNT(*) OVER (ORDER BY a) …
      -- was: (1,1),(2,2),(2,3),(3,4)   ← wrong for a=2 first row
      -- now: (1,1),(2,3),(2,3),(3,4)   ← correct (both a=2 rows in frame)

  Fix: ``_frame_slice`` now computes ``_peer_group_start`` /
  ``_peer_group_end`` helpers that scan backward/forward from position
  ``i`` to find all rows sharing the same ``ORDER BY`` key.  The
  helpers are used:

  * In the default-frame path (``frame is None``, ``order_cols``
    present): ``partition[:_peer_group_end()]`` replaces the old
    ``partition[:i+1]``.
  * In explicit ``RANGE`` frames (``frame.unit == "RANGE"``): the
    ``CURRENT_ROW`` bound in ``_start`` and ``_end`` delegates to the
    peer-group helpers instead of using the physical index ``i``.

  ``ROWS`` frames are completely unaffected — the ``is_range`` flag
  guards all peer-group expansions.

## 1.59.0 — 2026-05-24

### Fixed

- ``CAST(<blob> AS TEXT)`` now UTF-8-decodes the BLOB bytes instead
  of hex-encoding them, matching SQLite::

      CAST(x'48656c6c6f' AS TEXT)  ⟶  'Hello'   (was '48656c6c6f')
      CAST(x'31' AS TEXT)          ⟶  '1'       (was '31')
      CAST(x'3432' AS TEXT)        ⟶  '42'      (was '3432')

  Together with the 1.58.0 fix to ``CAST(<numeric> AS BLOB)``, this
  restores SQLite's documented round-trip identity::

      CAST(CAST(n AS BLOB) AS TEXT) == CAST(n AS TEXT)

  Fix in ``_cast_fn``'s TEXT-affinity branch: the ``bytes`` arm now
  calls ``x.decode("utf-8", errors="replace")`` instead of
  ``x.hex()``.  Invalid UTF-8 bytes are mapped to U+FFFD rather
  than raising — matches SQLite's "decode lazily, never error
  mid-query" stance and keeps the cast total.

## 1.58.0 — 2026-05-24

### Fixed

- ``CAST(<numeric> AS BLOB)`` now yields the UTF-8 encoding of the
  numeric's textual representation, matching SQLite::

      CAST(1 AS BLOB)     ⟶  b'1'
      CAST(42 AS BLOB)    ⟶  b'42'
      CAST(-7 AS BLOB)    ⟶  b'-7'
      CAST(1.5 AS BLOB)   ⟶  b'1.5'
      CAST(TRUE AS BLOB)  ⟶  b'1'

  Previously these used ``struct.pack(">q", x)`` for integers (and
  ``">d"`` for floats), producing 8-byte big-endian binary blobs
  that don't match SQLite's wire format and broke round-trip
  ``CAST(n AS BLOB)`` patterns that callers use for type-erased
  serialization.

  Fix in ``_cast_fn``'s BLOB-affinity branch: special-case ``bool``,
  ``int``, and ``float`` to encode via ``str(value).encode("utf-8")``.
  ``bool`` is checked before ``int`` because Python's ``bool`` is a
  subclass of ``int`` — without the explicit check, ``True`` would
  be encoded as ``b'True'`` (wrong) instead of ``b'1'`` (right).
  ``struct`` import removed (no longer used).

## 1.57.0 — 2026-05-24

### Fixed

- ``CAST(<bool> AS TEXT)`` now yields ``'1'`` / ``'0'`` instead of
  Python's ``'True'`` / ``'False'``.  SQLite has no native boolean
  type — TRUE and FALSE are aliases for the integers 1 and 0 — so
  the textual rendering of a cast boolean must match the integer
  string.  The previous implementation called ``str(x)`` directly,
  which leaked Python's ``bool`` repr into SQL output.

  Fix in ``_cast_fn``: the TEXT-affinity branch now special-cases
  ``isinstance(x, bool)`` before the generic ``str`` path
  (mirroring the existing INTEGER-affinity bool handling) and
  returns ``str(int(x))``.

## 1.56.0 — 2026-05-24

### Changed

- CHECK constraint violations now render as
  ``CHECK constraint failed: <expr_text>`` (matching SQLite) instead
  of ``CHECK constraint failed: <table>.<col>``.  The
  ``check_registry`` value shape grew from ``(col_name, instrs)`` to
  ``(col_name, expr_text, instrs)`` so the VM can quote the original
  predicate source.  The handler still accepts the legacy 2-tuple
  shape for backward compatibility with externally built fixtures
  (a fallback path emits the older ``<table>.<col>`` form when
  ``expr_text`` is empty).

## 1.55.0 — 2026-05-23

### Added

- ``execute()`` accepts a new ``fk_enabled: bool = True`` keyword
  forwarded to ``_VmState.fk_enabled``.  ``_check_fk_child`` and
  ``_check_fk_parent`` short-circuit to a no-op when False, mirroring
  SQLite's ``PRAGMA foreign_keys = OFF`` behaviour.  The default
  preserves existing call sites: omitting the kwarg keeps FK
  enforcement on.

## 1.54.0 — 2026-05-23

### Added

- ``_do_create_table`` now forwards the IR's ``autoincrement`` flag
  to ``BackendColumnDef.autoincrement``.  End-to-end this means
  ``CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT)`` flows
  through grammar → adapter → planner → codegen → VM → backend with
  the flag preserved, and ``sqlite_master.sql`` round-trips with
  ``AUTOINCREMENT`` in the reconstructed CREATE statement.

## 1.53.0 — 2026-05-23

### Added

- ``_do_create_table`` now forwards the IR's ``strict`` flag as a
  keyword arg to ``Backend.create_table(strict=...)``.  End-to-end this
  means ``CREATE TABLE t (x INTEGER) STRICT`` now triggers
  SQLite-compatible type enforcement on subsequent INSERT/UPDATE.

## 1.52.0 — 2026-05-23

### Added

- ``_do_alter_table`` now dispatches on the new IR ``AlterTable``
  optional fields: ``rename_to`` calls ``backend.rename_table``,
  ``rename_column`` calls ``backend.rename_column``, ``drop_column``
  calls ``backend.drop_column``.  The existing ``column`` (ADD COLUMN)
  branch is unchanged in shape but now also forwards the column's
  DEFAULT value to the backend (fixes a pre-existing bug where
  ``ALTER TABLE … ADD COLUMN x TEXT DEFAULT 'foo'`` backfilled NULL
  instead of ``'foo'``).
- 5 new tests in ``test_dml_ddl.py`` cover the new branches plus the
  error paths (RENAME on missing table, DROP unknown column).

## 1.51.0 — 2026-05-23

### Changed

- ``_do_create_table`` and ``_do_alter_table`` now forward the
  ``collation`` field from the IR ``ColumnDef`` to the backend's
  ``ColumnDef``.  This is the last hop on the journey from
  ``CREATE TABLE t(name TEXT COLLATE NOCASE)`` SQL through to the
  backend column metadata — the planner then reads it back via
  ``SchemaProvider.column_collation`` when resolving an ORDER BY
  clause.

## 1.50.0 — 2026-05-22

### Added

- ``_do_sort`` honours the new ``SortKey.collation`` field by
  transforming the value before the comparator sees it:
  - ``BINARY`` (or ``None``): pass-through
  - ``NOCASE``: ``str.lower()`` (ASCII case-insensitive)
  - ``RTRIM``:  ``str.rstrip(' ')``
  - Unknown name: pass-through (matches SQLite's lazy validation)
  Non-string values (ints, floats, blobs, NULL) pass through
  unchanged because SQLite's collations only affect TEXT
  comparison.

## 1.49.0 — 2026-05-21

### Added

- Runtime support for SQLite's bitwise operators (`&`, `|`, `<<`, `>>`,
  `~`).  All operate on 64-bit two's-complement signed integers, so
  results wrap exactly the way SQLite does:
  - `1 << 63` evaluates to `-9223372036854775808`, not the unbounded
    Python int `+9223372036854775808`.
  - Shift counts `≥ 64` saturate to `0` (or `-1` for arithmetic
    right-shift of a negative value, where the sign bit propagates).
  - Negative shift counts flip direction: `a << -k` ≡ `a >> k` and
    vice versa, matching SQLite's reinterpretation.
- Operand coercion follows the existing `_to_bitwise_int` helper:
  booleans become 0/1, floats truncate toward zero, strings raise
  `TypeMismatch` loudly so propagation bugs don't silently produce
  NULLs.
- `apply_unary` learned `UnaryOpCode.BIT_NOT`, with NULL propagation
  preserved (`~NULL` → NULL).

### Internal

- New `_to_i64` helper masks bitwise results to 64 bits and
  reinterprets the top bit as the sign — the same dance every CPU
  does on integer overflow.  Adding it in one place keeps the wrap-
  around behaviour consistent across all four binary bitwise ops.

## 1.48.0 — 2026-05-20

### Fixed

- **``CAST(... AS INTEGER)`` saturates at signed 64-bit bounds**
  matching SQLite's INTEGER affinity.  Previously the cast preserved
  arbitrary-precision Python bigints — so
  ``CAST(99999999999999999999 AS INTEGER)`` returned the full bigint
  instead of clamping to ``9223372036854775807``.  All four numeric
  paths (bool, float, string-prefix, native int) now flow through a
  new ``_clamp_int64`` helper.  Float overflow during ``int()`` is
  caught and saturated to the appropriate endpoint.

## 1.47.0 — 2026-05-20

### Fixed

- **``x / 0`` returns NULL** instead of raising ``DivisionByZero``.
  Matches SQLite's "arithmetic errors yield NULL" policy and lets
  callers wrap risky math in ``COALESCE(a/b, fallback)`` the way they
  would with real sqlite3.

- **``%`` operator follows C-style ``fmod`` semantics.**  Result sign
  matches the *dividend* (not the divisor like Python's ``%``):
  ``-7 % 3 → -1``, ``7 % -3 → 1``.

- **``%`` operator truncates floats to integers first** for the
  modulo computation, then casts back to float — so ``7.5 % 2.0 →
  1.0`` (because ``int(7.5) % int(2.0) == 7 % 2 == 1``).  This is
  *different* from the ``mod()`` scalar function, which still uses
  true ``math.fmod`` for backward compatibility with portable code
  written against the function spelling.

- **``mod()`` scalar function** now also uses ``math.fmod`` correctly
  for negative operands and always returns float (matches SQLite's
  documented ``mod()`` behaviour).

## 1.46.0 — 2026-05-20

### Fixed

- **``apply_unary(NOT, …)`` now coerces numeric operands to truth**
  via the same ``_truthiness`` helper used by ``AND``/``OR`` in 1.45.0.
  ``NOT 0`` was raising ``TypeMismatch`` (expected boolean, got
  INTEGER) and SQLite expects ``1``; same for ``NOT 5``, ``NOT 1.5``,
  etc.  Strings still raise TypeMismatch as before.

## 1.45.0 — 2026-05-20

### Fixed

- **``apply_binary(AND, …)`` and ``apply_binary(OR, …)`` now coerce
  numeric operands to truth values** instead of demanding Python
  ``bool``.  ``apply_binary(AND, 1, 0)`` used to raise ``TypeMismatch``
  because ``1`` isn't ``True`` (under ``is``).  SQLite has no separate
  BOOLEAN storage class — integers and floats double as booleans, with
  zero meaning FALSE and any other value meaning TRUE.

  A new ``_truthiness`` helper returns ``True``/``False`` for any
  non-NULL numeric, ``None`` for NULL, and (deliberately) ``None`` for
  strings.  The latter still raises TypeMismatch to keep ill-typed
  comparisons loud at runtime.

## 1.44.0 — 2026-05-20

### Added

- **``TIMEDIFF(A, B)`` scalar function** — SQLite 3.43+.  Returns the
  calendar-aware difference ``A − B`` as a string of the form
  ``±YYYY-MM-DD HH:MM:SS.sss``.  When ``A < B`` the sign is ``-`` and
  the magnitude is the time from ``A`` to ``B``.

  Implementation walks the seven fields (microseconds, seconds,
  minutes, hours, days, months, years) from low to high, borrowing
  from the next higher field whenever the current one is negative.
  The day-borrow step uses ``calendar.monthrange(…)`` of the month
  *preceding* ``A``'s month — that's what makes
  ``TIMEDIFF('2024-03-15', '2024-01-20')`` produce
  ``'+0000-01-24 …'`` (24 days because Feb 2024 has 29 days, hence
  ``15 + 29 − 20 = 24``).

  Microseconds are truncated to milliseconds (3 decimal places) to
  match SQLite's output precision.  NULL or unparseable inputs
  propagate NULL.

## 1.43.0 — 2026-05-20

### Fixed

- **``date(..., '+N year')`` no longer clamps Feb 29 — it rolls over
  to March 1** matching SQLite.  Previously
  ``date('2024-02-29', '+1 year')`` returned ``'2025-02-28'`` (clamp);
  SQLite returns ``'2025-03-01'`` because the naive ``2025-02-29``
  is invalid and the overflow day pushes into March.

  The fix ports the existing month-rollover algorithm to the year
  branch: try the literal ``(year + n, month, day)``; on ``ValueError``
  add the overflow as extra days starting from the month's last valid
  day.  The ``'+1 month'`` arithmetic was already correct because
  that's where the algorithm originated.

## 1.42.0 — 2026-05-20

### Fixed

- **``strftime('%f', …)`` preserves the millisecond fraction**.  The
  ISO-8601 datetime parser in ``_parse_timevalue`` had a fast path that
  truncated the input via slice arithmetic to fit the bare
  ``%Y-%m-%d %H:%M:%S`` strptime format, silently discarding the
  fractional-seconds suffix *before* the dedicated fractional-seconds
  branch could capture it.  Fix: try the fractional-seconds regex
  first, then fall through to the fixed-width formats only when the
  input has no fraction.  Result: ``strftime('%f', '2024-01-15
  12:30:45.123')`` now returns ``'45.123'`` (was ``'45.000'``).

- **``strftime('%W', …)`` matches SQLite byte-for-byte.**  The custom
  ``%W`` substitution used ``isocalendar()[1] - 1``, which produces
  ISO-week numbering shifted by one — different from POSIX week-of-
  year.  Python's own ``strftime('%W')`` already produces
  SQLite-compatible output, so the fix is to remove ``%W`` from the
  preprocessor and route it through the default Python path.

## 1.41.0 — 2026-05-20

### Fixed

- **``date()/datetime()/time()`` with the ``'unixepoch'`` modifier now
  forces numeric interpretation of the time value** — matching SQLite.

  Previously the modifier was a no-op: ``date('2024-01-15', 'unixepoch')``
  returned ``'2024-01-15'`` (SQLite returns NULL), and pure numeric
  strings like ``date('1704067200', 'unixepoch')`` returned NULL
  (SQLite returns ``'2024-01-01'``).

  The fix centralises ``unixepoch`` handling in ``_resolve_datetime``:
  when the modifier appears in the chain we coerce the time value to
  a number directly (via a ``re.fullmatch`` that requires the entire
  string be numeric — strings containing ``-`` like ISO dates have
  internal punctuation and fail the match), then strip the modifier
  from the chain so the downstream handler doesn't re-process it.

  Two existing test cases in ``test_scalar_functions.py`` were
  updated — they had pinned the previous no-op behaviour.

## 1.40.0 — 2026-05-20

### Fixed

- **``CAST(text AS REAL/INTEGER)`` uses SQLite's longest-prefix rule
  instead of Python's ``int()``/``float()`` semantics.**  Python rejects
  any string with trailing non-numeric characters; SQLite greedily
  takes the longest valid numeric prefix and discards the rest.

  Two specific bug classes fixed:

  * ``CAST('inf' AS REAL)`` (and ``'Inf'``, ``'infinity'``, ``'-inf'``,
    ``'nan'``, ``'NaN'``) used to surface Python's ``float('inf')`` /
    ``float('nan')`` to callers.  SQLite has no special-case for those
    keywords — they have no leading digit so the numeric prefix is
    empty, hence ``0.0``.

  * ``CAST('1.5abc' AS REAL)`` and ``CAST('123abc' AS INTEGER)`` used to
    return ``0.0`` / ``0`` because Python's ``float`` / ``int``
    rejected the whole string.  SQLite returns ``1.5`` / ``123`` —
    the valid prefix.

  Subtlety: ``CAST(string AS INTEGER)`` extracts only the *integer*
  prefix, not the float prefix.  So ``CAST('1.5abc' AS INTEGER)`` is
  ``1``, not ``1`` from truncating ``1.5`` (it never sees the decimal).
  ``CAST('1e5' AS INTEGER)`` is also ``1`` — the cast stops at the
  exponent marker.

  Two new helpers (``_sqlite_str_to_int``, ``_sqlite_str_to_real``)
  encode the rule via regex prefix matching.  48 unit tests in
  ``test_cast_numeric_prefix.py``.

## 1.39.0 — 2026-05-20

### Fixed

- **``REPLACE(x, "", y)`` is now a no-op** matching SQLite.  Python's
  ``str.replace("", X)`` inserts ``X`` between every character (so
  ``"hello".replace("", "X")`` becomes ``"XhXeXlXlXoX"``).  SQLite
  defines an empty needle as "match nothing" and returns the input
  unchanged.  Fix: short-circuit on empty ``old`` before delegating to
  Python's ``str.replace``.

- **``printf("%#o", val)`` now uses C's classic ``0`` octal prefix**
  instead of Python's modern ``0o`` prefix.  Mini-sqlite was emitting
  ``"0o10"`` for ``printf("%#o", 8)`` where SQLite emits ``"010"``.

  Subtleties also handled:
  * When ``val == 0`` the prefix is **omitted** entirely (the digit is
    already a zero; ``"00"`` would be wrong).  ``printf("%#o", 0) → "0"``.
  * With a width flag, the ``0`` prefix sits *after* leading spaces:
    ``printf("%#5o", 8) → "  010"``, not ``"   010"``.
  * Zero-padded widths grow by the prefix length:
    ``printf("%#05o", 8) → "000010"``.

  Implementation: strip ``#`` from the Python format, let Python compute
  the width/padding, then prepend ``0`` into the correct column.

  20 unit tests in ``test_replace_empty_and_octal.py``.

## 1.38.0 — 2026-05-20

### Fixed

- **``SUBSTR(x, y[, z])`` edge cases now match SQLite byte-for-byte.**
  The previous implementation accumulated per-branch fixups for
  negative ``y``, negative ``z``, and ``y = 0`` — and got several
  combinations wrong:

  * ``substr('hello', 0, 3)`` returned ``'hel'`` (wrong); SQLite
    returns ``'he'`` because ``y = 0`` means "one position before the
    string" so the span ``0, 1, 2`` intersects the string at
    positions ``1, 2``.
  * ``substr('hello', 2, -1)`` returned ``''`` (wrong); SQLite
    returns ``'h'`` — negative ``z`` asks for ``|z|`` characters
    *preceding* position ``y``.
  * ``substr('hello', -100, 5)`` returned ``'hello'`` (wrong); SQLite
    returns ``''`` because the resolved start (``-94``) plus length 5
    is still entirely to the left of position 1.

  The new algorithm models the requested character range as a closed
  1-indexed interval ``[lo, hi]``, clips to ``[1, N]``, and converts
  back to a Python slice — uniform handling that doesn't accumulate
  per-branch fixups.  Blob inputs use the same algorithm on bytes.

  33 unit tests in ``test_substr_edge_cases.py`` cover the full grid:
  positive/negative/zero ``y``, positive/negative ``z``, far-negative
  ``y``, empty strings, NULL, blob inputs, and the ``substring`` alias.

## 1.37.0 — 2026-05-20

### Fixed

- **``printf('%q', x)``** no longer wraps the result in single quotes.
  The ``%q`` conversion is the **escape-only** form — it doubles
  internal single quotes for safe interpolation inside a string
  literal the *caller* is writing.  Mini-sqlite was emitting
  ``'it''s'`` when SQLite emits ``it''s``; the caller had no way to
  embed the result in a larger literal.

- **``printf('%q', NULL)``** now returns the literal text ``"(NULL)"``
  (was empty string) — matches SQLite, which uses ``(NULL)`` so the
  caller cannot silently lose a NULL inside a generated SQL string.

### Added

- **``printf('%w', x)``** — new SQL identifier escape conversion.
  Doubles internal double quotes (the SQL identifier-quoting
  character).  NULL → ``"(NULL)"`` like ``%q``.  Designed for
  interpolation inside a ``"…"`` quoted identifier:
  ``printf('SELECT "%w" FROM t', col)``.

  36 unit tests in ``test_printf_q_w_correct.py`` cover the full
  ``%q``/``%Q``/``%w`` grid with parametric inputs, NULL handling,
  and composition with other conversions.  The single legacy
  ``test_sql_escape_q`` assertion in ``test_scalar_functions.py`` was
  updated — it had pinned the wrong (legacy) behaviour.

## 1.36.0 — 2026-05-20

### Fixed

- **``ROUND(x[, n])`` now rounds half away from zero** — Python's
  built-in ``round`` uses banker's rounding (round half to even), so
  ``round(0.5) == 0`` and ``round(2.5) == 2``.  SQLite uses the
  school-arithmetic convention: ``round(0.5) == 1.0``,
  ``round(2.5) == 3.0``, ``round(-2.5) == -3.0``.  The single-arg form
  now uses ``int64(x ± 0.5)``; the two-arg form quantises the exact
  IEEE 754 representation via ``Decimal(x).quantize(10**-n,
  ROUND_HALF_UP)``, which matches sqlite3's internal
  ``printf("%.*f", n, x)``-then-reparse path byte-for-byte.

- **``ROUND(x, n)`` clamps ``n`` to ``[0, 30]``** matching SQLite —
  negative ``n`` no longer rounds to the left of the decimal point,
  and excessively large ``n`` is capped at 30 (the maximum meaningful
  precision for a float64).

- **``ROUND(x, NULL)`` returns NULL** — SQLite short-circuits when
  either argument is NULL.  Mini-sqlite was previously coercing a
  NULL digits argument to the default ``0`` and returning a value.

  32 unit tests in ``test_round_half_away_from_zero.py`` cover one-arg
  and two-arg forms, NULL handling, and the [0, 30] clamping.

## 1.35.0 — 2026-05-19

### Added

Three SQLite 3.44+ string-family scalar functions
(``scalar_functions.py``):

- **``concat(...)``** — variadic, NULL args treated as empty string
  (NOT NULL-propagating).  Non-string args coerced via ``str()``.
  Requires ≥ 1 argument.  Example::

      CONCAT('a', NULL, 'b', 42)  → 'ab42'

- **``concat_ws(sep, ...)``** — concatenate with a separator.  Distinct
  NULL semantics from ``concat``:

  - NULL **separator** → NULL result (propagates).
  - NULL **value** → skipped (separator NOT doubled).

  Example::

      CONCAT_WS('-', 'a', NULL, 'b')   → 'a-b'
      CONCAT_WS(NULL, 'a', 'b')        → NULL

- **``octet_length(x)``** — byte length of a UTF-8-encoded string or
  BLOB.  Differs from ``length()`` for non-ASCII text::

      LENGTH('café')        → 4   (4 characters)
      OCTET_LENGTH('café')  → 5   ('é' = 2 bytes)
      LENGTH('🦀')          → 1   (1 character)
      OCTET_LENGTH('🦀')    → 4   (4-byte emoji)

  Numeric inputs coerced via decimal string representation
  (``OCTET_LENGTH(123) → 3``).

## 1.34.0 — 2026-05-19

### Fixed

- **``SELECT *`` now NULL-pads on LEFT JOIN unmatched rows**.  The
  previous behaviour of ``_do_scan_all_columns`` was to silently skip
  the cursor when it had no current row, which truncated the output
  row to fewer columns than real SQLite would produce.  Follow-up to
  the SELECT-star-cross-join fix in ``sql-codegen 1.31.0``.

  Wire-up:

  - ``_VmState.cursor_schema: dict[int, list[str]]`` — new cache
    holding the visible column names per open cursor.
  - ``_do_open`` (OpenScan handler) probes ``backend.columns(table)``
    and caches the resulting names at OpenScan time.  Backends that
    don't expose ``columns()`` fall back to the lazy path.
  - ``_do_advance`` lazily snapshots ``row.keys()`` the first time
    a cursor yields a row, covering subquery / derived-table /
    working-set cursors that bypass ``OpenScan``.
  - ``_do_scan_all_columns`` consults the cache when
    ``current_row[cursor_id]`` is missing and appends ``None`` per
    cached column name — matching SQLite's NULL-padded LEFT JOIN
    output.

  Example::

      Before:  SELECT * FROM a LEFT JOIN b ON … (no right match)
               → (a.id, a.name)                  -- wrong width
      Now:     → (a.id, a.name, None, None)     -- matches sqlite3

## 1.33.0 — 2026-05-19

### Added

- **Datetime timezone-offset modifiers** in ``_apply_modifier``: the
  ``±HH:MM``, ``±HH:MM:SS``, and ``±HH:MM:SS.SSS`` forms now shift the
  underlying datetime by the given offset, matching SQLite::

      datetime('2024-03-15 14:30:00', '+02:00')    → '2024-03-15 16:30:00'
      datetime('2024-03-15 14:30:00', '-05:30')    → '2024-03-15 09:00:00'
      datetime('2024-03-15 14:30:00', '+02:30:45') → '2024-03-15 17:00:45'

  Out-of-range components (e.g. ``+99:00``) return NULL.
- **``auto`` modifier** is now accepted as a no-op (SQLite 3.46+
  introduced it for forward-compat with future numeric encodings).
  Mini-sqlite's per-Python-type dispatch in ``_parse_timevalue`` already
  achieves what ``auto`` documents, so the modifier becomes a pass-
  through here; this matches real SQLite's behaviour on string inputs.

### Fixed

- **``%P`` strftime specifier** now returns ``'am'``/``'pm'`` on every
  platform.  Python's macOS libc returns the literal ``'P'`` for
  ``strftime('%P')``; we pre-process the specifier ourselves so output
  matches SQLite on Linux, macOS, and Windows CI runners.

## 1.32.0 — 2026-05-19

### Added

Eleven new built-in scalar functions, all oracle-verified against real
``sqlite3``:

- **Hyperbolic trig** — ``sinh``, ``cosh``, ``tanh``, ``asinh``,
  ``acosh``, ``atanh``.  Standard entries from SQLite's math function
  library (``--enable-math-functions``, default in Python's sqlite3).
  Out-of-domain inputs (e.g. ``acosh(0.5)``, ``atanh(1.0)``) return
  NULL via the existing ``_safe_math`` helper.
- **``trunc(X)``** — truncate toward zero.  Distinct from ``floor``
  for negative inputs: ``trunc(-3.7) = -3.0`` while ``floor(-3.7) =
  -4.0``.  Returns REAL to match SQLite.
- **Optimizer hints** — ``likely(X)``, ``unlikely(X)``, and
  ``likelihood(X, Y)``.  All three are identity functions in
  mini-sqlite (we have no cost-based optimizer), pinning portability
  for application SQL that sprinkles them in ``WHERE`` clauses.
- **Compile-option probes** — ``sqlite_compileoption_used(name)``
  returns ``0`` (mini-sqlite is not a SQLite build, so no compile
  options are defined) and ``sqlite_compileoption_get(N)`` returns
  ``NULL`` (no Nth option exists).  Safe responses for feature-
  detection probes in application code.

## 1.31.0 — 2026-05-19

### Added

- **SQLite conditional-upsert support** (`vm.py::_upsert_apply`).  Before
  evaluating the SET assignments, the VM now evaluates the
  pre-compiled `UpsertSpec.where_instructions` (if non-empty) with
  EXCLUDED and the existing row bound.  When the predicate evaluates
  falsy (False, 0, 0.0, NULL — same rules as `JumpIfFalse`), the upsert
  is silently skipped — semantically equivalent to ``DO NOTHING`` for
  that one row.  The fast paths for cursor cleanup are honoured on the
  early-exit branch.

## 1.30.0 — 2026-05-18

### Added

- **Connection-state scalar functions** (`scalar_functions.py`) — five new
  registered functions that SQLite apps commonly call:

  - `changes()` — rows affected by the most recent INSERT/UPDATE/DELETE.
  - `total_changes()` — cumulative rows affected since the connection opened.
  - `last_insert_rowid()` — rowid of the most recent successful INSERT.
  - `sqlite_version()` — dotted-integer version string ("3.45.0").
  - `sqlite_source_id()` — build identifier (mini-sqlite marker).

- **`set_connection_state(...)` helper** — module-level setter that the
  mini-sqlite engine calls after every statement to keep the three
  connection counters fresh.  Three module globals (`_LAST_INSERT_ROWID`,
  `_CHANGES`, `_TOTAL_CHANGES`) hold the per-process state.  Mini-sqlite
  is single-threaded so a single global is correct for the common
  single-connection use case; multi-connection programs see cross-talk
  (documented limitation).

- **`QueryResult.last_inserted_rowid`** (`result.py`) — new `int | None`
  field that propagates the rowid of the most recent INSERT through the
  result chain.  Populated by `_do_insert` based on the table's INTEGER
  PRIMARY KEY value, or a synthetic counter when no IPK is present.

- **`_do_insert` rowid tracking** (`vm.py`) — after a successful insert,
  looks up the table's INTEGER PRIMARY KEY value from the row dict;
  falls back to incrementing the previous rowid if the IPK isn't present.

### Changed

- Test `test_last_insert_rowid_returns_null` replaced by six tests that
  exercise the new `set_connection_state` plumbing.

## 1.29.0 — 2026-05-17

### Added

- **`__json_arrow(json, path)` scalar function** (`scalar_functions.py`) —
  implements `j -> path`.  Returns the value at the path as JSON text
  (numbers as quoted strings, objects/arrays as canonical JSON).  This
  keeps chained `j -> 'a' -> 'b'` expressions parsable as JSON at each
  step.
- **`__json_arrow_text(json, path)` scalar function** (`scalar_functions.py`) —
  implements `j ->> path`.  Returns the SQL-typed value (TEXT, INTEGER,
  REAL, or NULL).  Object/array results stay as JSON text (matching
  SQLite — `->>` does NOT unwrap composite values).
- **`_path_arg_to_jsonpath()` helper** — normalises the right-hand side of
  the arrow operator to a SQLite-style JSON path:
    integer N           → `$[N]`  (array index)
    string "a"          → `$.a`   (object key)
    string starting `$` → used verbatim

The functions are registered under `__`-prefixed names because they are
syntactic-sugar implementations rather than user-facing functions; the
adapter rewrites the operators into calls to these internal helpers.

## 1.28.0 — 2026-05-17

### Added

- **`like_match(value, pattern, escape=None)`** (`operators.py`) — the
  matcher gained an optional `escape` parameter.  Pattern characters that
  follow the escape character are treated as literal (wildcard meaning is
  disabled).  The implementation tokenises the pattern into `star`, `one`,
  and `lit` units before running a standard wildcard DP, collapsing each
  escape+char pair into a single literal token.  Consecutive `%` wildcards
  are collapsed to one star token for adversarial-pattern resilience.

- **`Like.has_escape` dispatch** (`vm.py`, `_do_like`) — when the IR
  instruction has `has_escape=True`, the handler pops a third stack value
  (the escape character) before the pattern and value.  A NULL escape
  yields a NULL result via three-valued logic; a non-single-character
  escape raises `TypeMismatch`.

## 1.27.0 — 2026-05-17

### Fixed

Four scalar-function divergences with the real `sqlite3` module:

- **`time()` accepts time-only strings** (`scalar_functions.py`) — `_parse_timevalue`
  was rejecting bare `HH:MM`, `HH:MM:SS`, and `HH:MM:SS.sss` strings, returning
  NULL.  SQLite anchors such inputs to year 2000-01-01 and accepts them as
  valid time values.  Added a regex match for time-only strings.

- **`weekday N` modifier** (`scalar_functions.py`) — `_apply_modifier` now
  recognises `weekday N` (0=Sunday, …, 6=Saturday) and advances the date to
  the next occurrence of that weekday.  Same-day matches leave the date
  unchanged.

- **`log(x)` is base-10 logarithm** (`scalar_functions.py`) — Mini-sqlite
  previously aliased `log` to `ln`, returning the *natural* logarithm.
  SQLite's `log()` is base-10; `ln()` is the natural log.  Split into two
  registrations.  `log(B, x)` (2-arg form) still computes log base B.

- **`hex(N)` uses decimal-string bytes** (`scalar_functions.py`) — SQLite's
  HEX() function operates on the SQL value's *string representation*, not its
  binary form.  `hex(123)` returns `"313233"` (the ASCII bytes of `"123"`),
  not the big-endian 8-byte encoding.

### Changed

- Test `test_log_natural` renamed to `test_log_base_10`; new
  `test_ln_natural` covers the natural-log case.
- Test `test_hex_integer` updated to expect the decimal-string-bytes form.

## 1.26.0 — 2026-05-17

### Fixed

- **NULL ordering bug in `_do_sort`** — the DESC branch of the sort comparator
  negated the NULL-placement rank (`-rank`), which inadvertently coupled NULL
  position to sort direction.  As a result `ORDER BY x DESC` with `NULLS LAST`
  put NULLs at the *start* of the result, not the end.

  NULL placement is now independent of direction: the rank (0/1/2 for
  FIRST/non-null/LAST) is kept positive in both ASC and DESC branches.  Only
  the value comparison is inverted (via the existing `_Rev` wrapper) for DESC.

  Truth table after the fix:

  | direction | nulls   | output                              |
  |-----------|---------|-------------------------------------|
  | ASC       | FIRST   | NULLs first, non-null ascending     |
  | ASC       | LAST    | non-null ascending, NULLs last      |
  | DESC      | FIRST   | NULLs first, non-null descending    |
  | DESC      | LAST    | non-null descending, NULLs last     |

  Combined with sql-codegen 1.28.0 (which now resolves the default to FIRST
  for ASC and LAST for DESC), this makes mini-sqlite byte-compatible with the
  real `sqlite3` module for NULL ordering.

## 1.25.0 — 2026-05-15

### Fixed

- **`pop_n(0)` stack corruption** (`vm.py`) — `pop_n(n)` with `n=0` previously
  deleted the entire stack because Python's `list[-0:]` equals `list[0:]` (the
  whole list) and `del self.stack[-0:]` empties it.  Added `if n == 0: return []`
  guard before the slice operation.

- **`IN ()` empty list** (`vm.py`) — `x IN ()` always returns `FALSE` per SQL
  semantics (there are no possible matches when the right-hand set is empty).
  The old handler would return `NULL` when the operand was NULL, or crash on
  `pop_n(0)`.  The fix adds an early `if ins.n == 0: st.push(False); return`
  before the NULL operand check.

- **`_do_sort` with `column_idx`** (`vm.py`) — The sort key dispatch now checks
  `k.column_idx is not None` before falling back to `columns.index(k.column)`.
  This supports the `ORDER BY N` positional sort keys emitted by the codegen
  (where `column_idx` is the 0-based SELECT-list position), avoiding the
  `ValueError: tuple.index("?")` that occurred when multiple computed columns
  shared the fallback display name `"?"`.

## 1.24.0 — 2026-05-15

### Added

- **`StripTrailingColumns`** (`vm.py`) — New post-processing instruction that
  removes the last `count` columns from `st.result.columns` and trims each row
  in `st.result.rows` by the same count.  Used by the codegen to erase hidden
  sort-key columns after `SortResult` has run.

  Implements `_do_strip_trailing(ins, st)`:

  1. Guards against `count ≤ 0` or `count ≥ len(columns)` (defensive noop).
  2. Slices `st.result.columns = st.result.columns[:-count]`.
  3. Rebuilds `st.result.rows` as `[row[:-count] for row in rows]` — an
     O(n·w) operation but unavoidable without lazy column hiding.

  The handler is placed between `SortResult` and `LimitResult` in the dispatch
  order so that stripping happens after sorting and before any LIMIT trim.

## 1.23.0 — 2026-05-14

### Changed

- **`_do_insert_from_result`** (`vm.py`) — Extended to support RETURNING on
  INSERT … SELECT.  When ``ins.returning_columns`` is non-empty the function:

  1. Snapshots the source rows from ``st.result.rows`` before clearing.
  2. Inserts each row as before, updating ``st.last_inserted_row`` per row.
  3. After each successful insert, reads the RETURNING column values from
     ``row_dict`` and accumulates them in a local list.
  4. After the loop, sets ``st.result.columns = ins.returning_columns``
     and extends ``st.result.rows`` with the accumulated tuples.
  5. Always updates ``rows_affected`` regardless of RETURNING.

  Rows skipped by ON CONFLICT IGNORE are not included in the RETURNING
  output, matching sqlite3 behaviour.  Column names absent from
  ``row_dict`` contribute ``None`` (covers unsupported complex RETURNING
  expressions).

## 1.22.0 — 2026-05-13

### Added

- **`AggFunc.JSON_GROUP_ARRAY`** (`vm.py`) — Accumulates non-NULL SQL values
  across a GROUP BY group into a JSON array.  Returns `'[]'` for an empty group
  (never NULL, unlike GROUP_CONCAT).  Uses `_sql_to_json_val` to convert each
  SQL scalar to the appropriate JSON type before accumulation.
- **`AggFunc.JSON_GROUP_OBJECT`** (`vm.py`) — Accumulates (key, val) pairs into
  a JSON object.  The codegen pushes the key expression *before* the value;
  `UpdateAgg` pops both.  Rows with NULL key or NULL value are silently skipped.
  Duplicate keys: last writer wins.  Returns `'{}'` for an empty group.

### Changed

- `_do_update_agg` now handles `JSON_GROUP_OBJECT`'s two-value stack protocol:
  when `value` is NULL the key is also popped to keep the stack balanced.
- `import json as _json` added to `vm.py` to serialise JSON group results.
- `_sql_to_json_val` imported from `scalar_functions` for JSON type conversion.

### Fixed

- **Non-finite float JSON safety** (`vm.py`): `json_group_array` and
  `json_group_object` finalize handlers now map `inf` and `nan` values to
  JSON `null` before serialising.  Python's `json.dumps` emits `Infinity` /
  `NaN` for non-finite floats by default, which is not valid per RFC 8259.
  SQLite maps such values to null; we now match that behaviour.
- **DISTINCT stack balance for `json_group_object`** (`vm.py`): the DISTINCT
  duplicate-skip early-return path now pops the stranded key from the operand
  stack when the aggregate function is `JSON_GROUP_OBJECT`.  Previously the
  key value was left on the stack, which would corrupt subsequent operand
  reads.  (The adapter never emits `DISTINCT json_group_object` today, so
  this was latent; the fix removes the time-bomb before it can fire.)

## 1.21.0 — 2026-05-13

### Added

- **JSON1 scalar functions** (`scalar_functions.py`) — 14 new functions
  matching the SQLite JSON1 extension, all implemented with Python's built-in
  `json` module:
  - `json(x)` — canonical (minified) JSON string.
  - `json_valid(x)` — 1 if *x* is valid JSON, 0 otherwise, NULL for NULL input.
  - `json_quote(x)` — SQL value → JSON text representation.
  - `json_array(v1, v2, …)` — build a JSON array from zero or more SQL values.
  - `json_object(k1, v1, k2, v2, …)` — build a JSON object from key/value pairs.
  - `json_extract(json, path [, path…])` — extract one or more values at JSON
    paths; multiple paths return a JSON array.
  - `json_type(json [, path])` — return the SQLite JSON type name
    ("null", "true", "false", "integer", "real", "text", "array", "object").
  - `json_array_length(json [, path])` — number of elements in a JSON array,
    or 0 for non-arrays (matching SQLite semantics).
  - `json_keys(json [, path])` — JSON array of the keys in a JSON object.
  - `json_patch(target, patch)` — RFC 7396 JSON Merge Patch.
  - `json_remove(json, path [, path…])` — remove one or more paths.
  - `json_set(json, path, val [, path, val…])` — insert or replace paths.
  - `json_insert(json, path, val [, …])` — insert only (no overwrite).
  - `json_replace(json, path, val [, …])` — replace only (no insert).
  - `json_group_array(v1, v2, …)` — scalar alias for `json_array`.

- **`TOTAL()` aggregate** (`vm.py`) — SQLite-specific aggregate that returns
  `0.0` (float) for empty groups or all-NULL input, never returning NULL.
  Added `AggFunc.TOTAL` to the `AggFunc` enum in `sql-codegen/ir.py` and
  handled in `_do_update_agg` (identical accumulation to SUM) and
  `_do_finalize_agg` (returns `0.0` instead of `None` on empty/all-null).

## 1.20.0 — 2026-05-13

### Added

- **`_frame_slice` helper** (`vm.py`) — new internal function that converts a
  `WinFuncSpec.frame` (or the SQL-standard default) to the list of partition
  rows visible at each row position `i`.  Supports `UNBOUNDED PRECEDING`,
  `CURRENT ROW`, `N PRECEDING`, `N FOLLOWING`, and `UNBOUNDED FOLLOWING`
  bounds for both `ROWS` and `RANGE` units.

### Fixed

- **Running / cumulative aggregates** (`vm.py`,
  `_do_compute_window`) — `SUM`, `COUNT`, `COUNT(*)`, `AVG`, `MIN`, and `MAX`
  window functions now respect the SQL-standard default frame: when `ORDER BY`
  is present in the window spec and no explicit frame is given, the frame is
  `RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW` (cumulative).
  Previously all aggregate window functions used the full-partition frame
  unconditionally, causing `SUM(x) OVER (ORDER BY x)` to return the global
  sum instead of a running sum.

- **`NTH_VALUE` per-row frame** (`vm.py`) — `NTH_VALUE(col, n)` now looks up
  the n-th value within the frame visible at each row position.  Rows whose
  frame does not yet contain n elements return `NULL` instead of broadcasting
  the partition-level n-th value to all rows.

- **`LAST_VALUE` per-row frame** (`vm.py`) — `LAST_VALUE(col)` now returns
  the last value within the current row's frame instead of always broadcasting
  the final partition value.  With the default cumulative frame the last value
  in the frame ending at the current row is the current row itself.

- **`FIRST_VALUE` explicit frames** (`vm.py`) — `FIRST_VALUE(col)` now
  correctly uses `_frame_slice` so that non-default start bounds (e.g.
  `1 PRECEDING`) are honoured.

## 1.19.0 — 2026-05-13

### Fixed

- **IS DISTINCT FROM / IS NOT DISTINCT FROM** (`operators.py`) — new
  NULL-safe comparison operators.  `IS DISTINCT FROM` returns `True` when
  the operands differ *or* exactly one is NULL; `IS NOT DISTINCT FROM`
  returns `True` when both are equal or both are NULL.  Neither operator
  ever returns NULL, unlike the regular `=` / `<>` operators.

- **Scalar MAX/MIN NULL propagation** (`scalar_functions.py`) — the
  two-or-more-argument scalar `MAX(…)` / `MIN(…)` now return `NULL`
  when any argument is `NULL`, matching SQLite's multi-argument max/min
  semantics.  The previous implementation treated `NULL` as less than
  all values (aggregate semantics); the scalar form propagates `NULL`.

- **ABS(text) → 0.0** (`scalar_functions.py`) — non-numeric text inputs
  now coerce to `0.0` via numeric-prefix regex rather than returning
  the original string.  Matches SQLite.

- **HEX(NULL) → ""** (`scalar_functions.py`) — returns an empty string
  instead of `NULL`, matching SQLite.

- **DATE +1 month overflow** (`scalar_functions.py`) — landing on a
  non-existent day (e.g. Jan 31 + 1 month → Feb 31) now overflows into
  the next month rather than clamping to the last valid day.  Matches
  SQLite: `date('2023-01-31', '+1 months')` → `'2023-03-03'`.

### Tests updated

- `test_mod_by_zero` — expects `None` instead of `DivisionByZero`.
- `test_abs_non_numeric_passthrough` — asserts `0.0` (not pass-through).
- `test_hex_null` — asserts `""` (not `None`).
- `test_date_plus_month_*_clamp` renamed to `*_overflow`; values updated.
- `test_max_with_null_returns_non_null` renamed; now asserts `None`.

## 1.18.0 — 2026-05-13

### Fixed

- **`x % 0` returns NULL** (`operators.py`) — the `BinaryOpCode.MOD` branch in
  `_arithmetic` now returns `None` (SQL NULL) when the divisor is zero instead of
  raising `DivisionByZero`.  This matches real SQLite's behaviour: `SELECT 5 % 0`
  → `NULL`.  Division by zero for `/` still raises `DivisionByZero` (SQLite
  behaviour).

### Added

- **DISTINCT deduplication for aggregate slots** (`vm.py`, `_AggState`) — the
  `_AggState` accumulator gains two new fields:

  - `distinct: bool = False` — set at `InitAgg` time; activates deduplication.
  - `seen: set | None = None` — lazily-populated set of already-accumulated values;
    only allocated when `distinct=True` to keep the common case lightweight.

  `_do_update_agg` now checks `agg.distinct` before accumulating a value.  If the
  value is already in `agg.seen` the row is silently skipped, implementing
  `COUNT(DISTINCT col)`, `SUM(DISTINCT col)`, `AVG(DISTINCT col)`, etc.

  `_do_init_agg` populates `distinct` and `seen` from the new `InitAgg.distinct`
  field added in `sql-codegen 1.19.0`.

## 1.17.0 — 2026-05-12

### Added

- **`LoadRowId` instruction dispatch** (`vm.py`) — the VM now handles
  `LoadRowId(cursor_id)` by looking up the cursor in `st.cursors`, calling
  `cursor.rowid()` via duck-typing (`getattr(cursor, "rowid", None)`), and
  pushing the result onto the operand stack.  Cursors that don't implement
  `rowid()` (subquery iterators, file-backed backends) push `None` without
  raising an exception.

- **Hidden-key filtering in `_do_scan_all_columns`** (`vm.py`) — when `SELECT *`
  scans all columns from a row dict, keys starting with `"\x00"` are excluded
  from both the value buffer and the result-column schema.  This ensures that
  the hidden `"\x00rowid"` stamp stamped by `InMemoryBackend` never appears in
  `SELECT *` output.

## 1.16.0 — 2026-05-05

### Added

- **`UPSERT` execution — `ON CONFLICT DO UPDATE` and `ON CONFLICT DO NOTHING`**
  (`vm.py`) — full upsert semantics for both VALUES-based and SELECT-based INSERTs.

  Key additions:

  - **`excluded_row: dict[str, SqlValue]`** on `_VmState` — stores the
    *would-be-inserted* row during upsert SET expression evaluation so that
    `LoadExcludedColumn` instructions can read from it.

  - **`LoadExcludedColumn` dispatch** — pushes `st.excluded_row.get(ins.col)` onto
    the operand stack.  This instruction is only executed inside `_upsert_apply`.

  - **`_do_upsert(table, row, upsert, st) -> bool`** — the main upsert decision
    function called from `_do_insert` and `_do_insert_from_result` when the backend
    raises `ConstraintViolation`.  Returns `True` if the conflict was handled (skip
    or update), `False` if no match was found (re-raises the original exception).
    - **DO NOTHING fast path**: immediately returns `True`, silently dropping the
      conflicting row.
    - **DO UPDATE**: opens a positioned cursor (`_open_cursor` if available, else
      `scan`), scans for the conflicting row using `conflict_target` columns (or
      schema-discovered PRIMARY KEY columns when `conflict_target` is empty), then
      calls `_upsert_apply`.

  - **`_upsert_apply(table, excluded, existing, upsert, cur, st)`** — evaluates
    each `UpsertAssignment.instructions` sequence via `_dispatch`, collecting the
    resulting value for each column, then calls `backend.update()` on the positioned
    cursor.  Before evaluation it temporarily parks `existing` in
    `st.current_row[0]` so that bare column references (e.g. `n + 1` where `n`
    refers to the existing row) resolve correctly via `LoadColumn(cursor_id=0, …)`.
    After evaluation it restores the previous `current_row[0]` entry.

  - **`_do_insert` and `_do_insert_from_result` updated** — catch
    `ConstraintViolation` and call `_do_upsert`.  If the upsert handler returns
    `False` (no match found), the original `ConstraintViolation` is re-translated
    and re-raised.

- **`test_upsert.py`** — 12 focused VM-level upsert tests covering DO NOTHING,
  DO UPDATE with EXCLUDED.col, bare column refs, arithmetic, multiple assignments,
  empty conflict_target, counter accumulation, and plain inserts with no conflict.

## 1.15.0 — 2026-05-05

### Added

- **DEFAULT column value passthrough in `_do_create_table`** (`vm.py`) —
  `CreateTable` now reads `c.default` from each `ColumnDef` in the instruction
  and passes it to `BackendColumnDef(default=...)`.  When `c.default` is the
  IR sentinel `NO_COLUMN_DEFAULT` the VM converts it to the backend sentinel
  `NO_DEFAULT`, preserving the existing "no default declared" semantics.  Any
  other value (integer, float, string, or `None` for DEFAULT NULL) is passed
  through verbatim so `InMemoryBackend._apply_defaults()` can fill the column
  on INSERT when it is omitted by the caller.

  This closes the final gap in the DEFAULT pipeline:
  `sql-parser → adapter → sql-backend ColumnDef → IR ColumnDef → VM → InMemoryBackend`.

## 1.14.0 — 2026-05-04

### Added

- **`INSERT OR REPLACE` / `REPLACE INTO`** — when `InsertRow.on_conflict ==
  "REPLACE"`, `_do_insert()` calls the new `_replace_delete_conflicts()`
  helper before inserting.  That helper scans the target table using a
  positioned cursor (`_open_cursor` where available, `scan` as fallback) and
  deletes every existing row that shares a value on any UNIQUE or PRIMARY KEY
  column with the incoming row.  The scan-delete is single-pass because the
  backend guarantees the cursor stays live after deletion and advances to the
  next row automatically.  The same logic applies to `_do_insert_from_result`
  for `INSERT OR REPLACE … SELECT`.

- **`INSERT OR IGNORE`** — when `InsertRow.on_conflict == "IGNORE"` or
  `InsertFromResult.on_conflict == "IGNORE"`, `ConstraintViolation` from the
  backend is caught silently and the row is skipped.  Other exceptions are
  still re-raised as `IntegrityError`.

- **`_replace_delete_conflicts` helper** — pre-scans a table and deletes all
  rows conflicting with a new row on any UNIQUE/PRIMARY KEY column.  Uses
  `getattr(backend, "_open_cursor", None)` to prefer positioned cursors
  (required by `InMemoryBackend.delete()`) over read-only `scan()` iterators.
  Only non-NULL column values are checked (NULL never conflicts in SQL).

### Fixed

- **`_do_create_table` now passes `unique=c.unique` to `BackendColumnDef`**
  — the VM handler for the `CreateTable` IR instruction was building
  `BackendColumnDef` without the `unique` keyword, causing every UNIQUE column
  constraint to be silently ignored by the backend.  Non-PK UNIQUE columns
  would accept duplicate values without raising `ConstraintViolation`, making
  `INSERT OR IGNORE` unable to detect non-PK UNIQUE conflicts.

## 1.13.0 — 2026-05-04

### Added

- **`glob(pattern, string)` scalar function** (`scalar_functions.py`) —
  registers the built-in `glob` function used by the `GLOB` operator.
  Case-sensitive Unix-style pattern matching via `fnmatch.fnmatchcase`.
  Returns a Python `bool` (coerced to `1`/`0` on output) so that
  `UnaryOp.NOT` and WHERE-clause `JumpIfFalse` both work correctly with it.
  NULL arguments propagate to NULL.

### Fixed

- **`JumpIfFalse` / `JumpIfTrue` now use proper SQL truthiness** (`vm.py`) —
  previously only Python `False` (identity) was treated as falsy; now any
  value for which `not v` is true (including integer `0`, float `0.0`) is
  treated as falsy. This fixes GLOB and any other scalar predicate that
  returns an integer rather than a Python bool, and correctly handles
  `WHERE 0` / `WHERE 1` literals.

- **`like_match` is now case-insensitive for ASCII** (`operators.py`) —
  ANSI SQL and SQLite both define LIKE as case-insensitive by default for
  ASCII characters. The DP table now normalises both value and pattern to
  lowercase before comparison, preserving `%` / `_` wildcard semantics.

## 1.12.0 — 2026-05-04

### Added

- **`GROUP_CONCAT` aggregate execution** (`vm.py`) — `_do_update_agg` and
  `_do_finalize_agg` now handle `AggFunc.GROUP_CONCAT`:
  - Per-row accumulation into `_AggState.items` (a `list[str]`); NULLs are
    silently ignored; integers and whole-number floats are rendered without a
    trailing `.0` to match SQLite output.
  - Finalisation joins the list with `agg.separator`; an empty list returns
    `None` (matching SQLite's NULL-for-empty-group behaviour).
- **`items` and `separator` fields on `_AggState`** (`vm.py`) — `items`
  accumulates strings for GROUP_CONCAT; `separator` is baked in at
  `InitAgg` time and carried through to `FinalizeAgg`.
- **Implicit-single-group synthesis in `AdvanceGroupKey` handler** (`vm.py`)
  — when `has_group_by=False` and the scan produced no rows (`group_order`
  is empty), the VM synthesises the implicit `()` group so that no-GROUP-BY
  aggregates over empty tables return exactly one row of NULL/zero values,
  matching the SQL standard.
- **Lazy slot initialisation in `_do_finalize_agg`** (`vm.py`) — if the
  slot list for the current group is shorter than the requested slot index
  (because `InitAgg` was never called on an empty table), the handler
  auto-grows the list with default `_AggState` entries using the `func` and
  `separator` baked into the `FinalizeAgg` instruction.  This eliminates
  the previous `InternalError` and produces the correct zero-state result.

### Security

- **NTILE DoS prevention** (`vm.py`) — `n_buckets` is clamped to
  `max(1, min(n_raw, total_rows))` before the modulo-distribution loop,
  preventing divide-by-zero and pathological O(N²) behaviour from
  caller-supplied values ≤ 0.
- **Defense-in-depth guards** (`vm.py`) — `LAG`, `LEAD`, `NTILE`, and
  `NTH_VALUE` handlers raise `RuntimeError` on non-integer extra-arg
  values, catching any `WinFuncSpec` objects that bypass codegen validation.

## 1.11.0 — 2026-05-04

### Added

- **LAG window function** (`vm.py`) — `_do_compute_window` now handles
  `WinFunc.LAG`: returns the value of `arg_col` from the row `offset`
  positions before the current row in the sorted partition.  Returns
  `default_val` (from `extra_args[1]`) when no prior row exists at that
  distance.  Offset and default are taken from `spec.extra_args = (offset,
  default)`, normalised to `(1, None)` by the codegen if omitted.
- **LEAD window function** (`vm.py`) — mirror of LAG, looks ahead by
  `offset` positions instead of behind.
- **NTILE window function** (`vm.py`) — `WinFunc.NTILE` divides the
  partition into `n` approximately equal numbered buckets (1..n).
  Distribution matches SQLite and PostgreSQL: `q, r = divmod(len, n)`;
  the first `r` buckets get `q+1` rows, the remaining `n-r` get `q` rows.
  `n` is taken from `spec.extra_args[0]`.
- **PERCENT_RANK window function** (`vm.py`) — `WinFunc.PERCENT_RANK`
  computes `(rank − 1) / (N − 1)` where rank is the SQL RANK() value and
  N is the partition size.  Returns `0.0` when `N == 1` (avoids division
  by zero).
- **CUME_DIST window function** (`vm.py`) — `WinFunc.CUME_DIST` computes
  the cumulative distribution as `(end-of-peer-group index + 1) / N`.
  Tied rows share the same peer-group endpoint so they all receive the
  same value.
- **NTH_VALUE window function** (`vm.py`) — `WinFunc.NTH_VALUE` returns
  the value of `arg_col` at the n-th row (1-indexed) of the partition.
  Rows beyond the partition size return `NULL`.  `n` is taken from
  `spec.extra_args[0]`.

## 1.10.0 — 2026-05-04

### Added

- **`last_inserted_row` field on `_VmState`** (`vm.py`) — a
  `dict[str, SqlValue]` that is overwritten with the full row dict every time
  `_do_insert` executes an `InsertRow`.  Provides the data source for
  `LoadLastInsertedColumn`.
- **`LoadLastInsertedColumn(col)` dispatch** (`vm.py`) — `_dispatch` now
  handles `LoadLastInsertedColumn` by pushing
  `st.last_inserted_row.get(ins.col)` onto the value stack, returning `None`
  (NULL) when the column is not present.  Powers INSERT … RETURNING without
  requiring an open cursor after the insert.

## 1.9.0 — 2026-05-04

### Added

- **`outer_current_row` parameter on `execute()`** (`vm.py`) — optional
  `dict[int, dict[str, SqlValue]]` mapping outer cursor IDs to their current
  row snapshots.  Defaults to `{}` (empty).  Stored in `_VmState` for use
  by the `LoadOuterColumn` handler.
- **`_VmState.outer_current_row` field** (`vm.py`) — the outer row snapshot
  from the enclosing query; populated at construction time from `execute()`'s
  parameter.
- **`LoadOuterColumn` dispatch** (`vm.py`) — `_dispatch` routes
  `LoadOuterColumn(cursor_id, col)` to the new `_load_outer_column()` helper,
  which reads `col` from `outer_current_row[cursor_id]` and pushes the value
  (or `None` if the cursor or column is absent).
- **Correlated outer-row threading** (`vm.py`) — `_do_run_exists_subquery`,
  `_do_run_scalar_subquery`, and `_do_run_in_subquery` now call
  `execute(sub_program, backend, outer_current_row=st.current_row)` so that
  inner programs can resolve `LoadOuterColumn` against the outer scan's
  snapshot.  Each outer row gets a fresh inner execution — no caching.
- **11 new VM tests** in `tests/test_correlated_subquery.py`:
  `LoadOuterColumn` unit tests (basic, missing cursor, missing column, no
  `outer_current_row`), and end-to-end planner→codegen→VM tests for
  correlated IN, NOT IN, EXISTS, NOT EXISTS, scalar subquery, and per-row
  re-execution.

## 1.8.0 — 2026-05-04

### Added

- **`RunInSubquery` handler** (`vm.py`) — executes the embedded
  `sub_program` via a recursive `execute()` call, materializes the
  first column of all result rows into a `set`, and pushes a `bool` or
  `None` onto the value stack.  SQL three-valued NULL logic:
  - test value is `NULL` → push `None`
  - test value in non-null set → push `True` (or `False` when `negate=True`)
  - set contains `NULL` and value not found → push `None` (UNKNOWN)
  - value not found, no NULLs in set → push `False` (or `True` when `negate=True`)

## 1.7.0 — 2026-05-04

### Added

- **FULL OUTER JOIN execution** — no new VM instructions needed.  FULL JOIN
  is compiled to two passes by `sql-codegen`: Pass 1 emits left rows via
  the existing LEFT JOIN machinery; Pass 2 is a right-anti-join that emits
  only unmatched right rows.  The null-padding mechanism is identical to
  LEFT/RIGHT JOIN: a closed inner cursor returns `None` from `_load_column`.
- **4 new outer-join VM tests** in `tests/test_outer_join.py`:
  `test_full_join_all_rows_appear`, `test_full_join_left_empty`,
  `test_full_join_right_empty`, `test_full_join_no_overlap`.

## 1.6.0 — 2026-05-04

### Added

- **`join_match_stack: list[bool]`** added to `_VmState` — a stack that
  tracks, per active left row, whether any right row satisfied the JOIN
  ON condition. Supports arbitrarily nested LEFT OUTER JOINs.
- **`JoinBeginRow` handler** — appends `False` to `join_match_stack`.
- **`JoinSetMatched` handler** — sets `join_match_stack[-1] = True`.
- **`JoinIfMatched(label)` handler** — pops the stack; conditionally
  jumps to *label* if the popped value is `True`. When the stack is
  empty (defensive), pops as `False` and falls through.
- **LEFT OUTER JOIN null-padding** — no new instruction required; when
  the right scan's `CloseScan` removes the cursor from `current_row`,
  any subsequent `LoadColumn` for right-side columns returns `None`
  automatically (existing `_load_column` semantics).

## 1.5.0 — 2026-04-28

### Added

- **User-defined functions (UDFs)** — `execute()` accepts `user_functions`
  dict; `_do_call_scalar` checks user registry before built-ins. nargs=-1
  for variadic functions.
- **`RunScalarSubquery` handler** — `_do_run_scalar_subquery` executes the
  embedded sub-program, pushes the single result value, or NULL when empty.
- **`CardinalityError`** (`errors.py`) — raised when a scalar subquery
  returns more than one row; exported from `sql_vm.__init__`.
- **`primary_key` passed to `BackendColumnDef`** in `_do_create_table` —
  threads the primary-key flag through to the backend so PRAGMA table_info
  correctly reports pk=1 for primary-key columns.

## 1.4.0 — 2026-04-28

### Added — Phase 9: SQL Triggers

- **`TriggerDepthError`** (`errors.py`) — raised when trigger recursion exceeds
  depth 16; exported from `sql_vm.__init__`.
- **`_VmState.trigger_executor` / `.trigger_depth`** — optional callback and
  nesting depth injected by the façade layer; the VM calls the executor for
  each trigger that should fire without importing parsing/planning code itself.
- **`execute()` new kwargs** — `trigger_executor` and `trigger_depth` wired
  into `_VmState` construction.
- **`_fire_trigger()`** — checks depth limit, then delegates to the executor.
- **`_do_insert` / `_do_update` / `_do_delete`** — fire BEFORE and AFTER
  triggers around the actual DML call.
- **`_do_create_trigger` / `_do_drop_trigger`** — new dispatch handlers for
  `CreateTriggerDef` / `DropTriggerDef` IR instructions.

### Fixed

- **`_do_update` old-row snapshot** — `current_row[cursor_id]` was captured as
  a mutable reference; subsequent in-place `update(assignments)` mutated
  `old_row` before AFTER triggers fired, causing OLD.col to return the
  post-update value.  Fixed by calling `dict(...)` to take a shallow copy.

## 1.3.0 — 2026-04-27

### Added — Phase 8: Window Functions (OVER / PARTITION BY)

- **`_do_compute_window()` handler** — dispatched when the VM encounters a
  `ComputeWindowFunctions` instruction.  Two-pass algorithm:
  1. Converts the result buffer rows to dicts keyed by `result.columns`.
  2. Groups rows into partitions by `partition_cols` (empty key = global window).
  3. Sorts each partition by `order_cols` using a NULL-first `_win_sort_key()`.
  4. Evaluates each `WinFuncSpec` in order:
     - Ranking: `ROW_NUMBER`, `RANK`, `DENSE_RANK`
     - Aggregate: `SUM`, `COUNT` (skips NULLs), `COUNT_STAR`, `AVG`, `MIN`, `MAX`
     - Value: `FIRST_VALUE`, `LAST_VALUE`
  5. Projects rows to `output_cols` and updates `result.columns`.
- **`_win_sort_key()` / `_Descending` helpers** — NULL-first sort key; wraps
  non-NULL values in `_Descending` for DESC columns.
- **`_order_vals()` helper** — extracts ordered column values from a row dict.

## 1.2.0 — 2026-04-27

### Added — Phase 5b: Recursive CTEs

- **`_VmState.working_set_data: list[dict[str, SqlValue]]`** — stores the
  current working-set rows for the recursive iteration; populated by
  `_execute_with_cursors` before each recursive step.
- **`_execute_with_cursors(program, backend, working_set_rows)`** — private
  helper that runs a sub-program with a pre-loaded working set.  Sets
  `state.working_set_data` rather than directly populating cursor 0, so
  `OpenWorkingSetScan` can re-create a fresh cursor on each inner-loop
  entry (crucial for correctness when the self-reference appears inside a JOIN).
- **`RunRecursiveCTE` dispatch** — `_do_run_recursive_cte` implements the
  fixed-point algorithm:
  1. Execute anchor program via `execute()`; collect anchor rows as the initial
     working set.
  2. Repeat: run `recursive_program` via `_execute_with_cursors(working_rows)`;
     collect new rows; if `union_all=False` deduplicate against a `seen` set.
  3. Terminate when the working set is empty.
  4. Populate `st.cursors[cursor_id]` with a `_SubqueryCursor` over all
     accumulated rows.
- **`OpenWorkingSetScan` dispatch** — handler creates a fresh
  `_SubqueryCursor(rows=st.working_set_data)` bound to `cursor_id`.
  Each call produces an independent cursor so JOIN outer loops can exhaust
  and reopen without interfering with each other.
- **Column name normalisation** — output column names always come from the
  anchor's `result.columns`, matching the SQL standard rule that UNION output
  names are taken from the leftmost SELECT.

## 1.1.0 — 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`fk_child` / `fk_parent` parameters on `execute()`** — mutable dicts passed
  from `Connection` so FK registrations from `CREATE TABLE` persist across calls.
  `fk_child`: child_table → [(child_col, parent_table, parent_col_or_None)].
  `fk_parent`: parent_table → [(child_table, child_col, parent_col_or_None)].
- **`_VmState.fk_child` / `fk_parent`** — two new `field(default_factory=dict)`
  fields carrying both directions of the FK graph.
- **`_do_create_table` populates both registries** — for every column with a
  non-None `foreign_key` tuple, writes forward (child→parent) and reverse
  (parent→child) entries using `dict.setdefault`.
- **`_check_fk_child()`** — scans the parent table and raises `ConstraintViolation`
  when a non-NULL FK value has no matching row.  NULL passes unconditionally
  (SQL standard: NULL reference is not an error).
- **`_check_fk_parent()`** — scans the child table and raises `ConstraintViolation`
  (RESTRICT) when deleting a parent row that is still referenced.
- **`_fk_find_pk()` / `_fk_row_exists()`** — helpers: PK column discovery and
  O(n) scan predicate.
- **INSERT, UPDATE, DELETE enforcement** — `_do_insert` and `_do_update` call
  `_check_fk_child` after CHECK; `_do_delete` calls `_check_fk_parent` before
  the backend write.
- **6 new VM-level tests** in `test_dml_ddl.py`.

## 1.0.0 — 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`check_registry` parameter on `execute()`** — a mutable `dict` passed in from
  `Connection` so CHECK state registered by `CREATE TABLE` persists across calls.
  The dict maps `table_name → list[(col_name, check_instrs)]`.
- **`_do_create_table` populates `check_registry`** — for each column whose IR
  `ColumnDef.check_instrs` is non-empty, an entry is written into the registry so
  subsequent INSERT/UPDATE calls can enforce it.
- **`_check_constraints()` helper** — iterates over the registry entry for the
  target table, temporarily sets `st.current_row[CHECK_CURSOR_ID] = row`, runs the
  pre-compiled instruction sequence, pops the result, and raises `ConstraintViolation`
  when the result is `False`.  NULL results pass (SQL three-valued-logic).
- **`ConstraintViolation` exports `table` and `column`** — the raised exception
  carries enough detail for the mini-sqlite layer to produce an informative error.
- **INSERT and UPDATE enforcement** — `_do_insert` validates the to-be-inserted row
  before writing; `_do_update` merges pending assignments with the current row and
  validates the merged dict before writing, preserving transactional rollback on
  violation.
- **Tests** — 4 new tests in `test_dml_ddl.py` covering valid INSERT, violating
  INSERT, violating UPDATE, and NULL passthrough.

## 0.9.0 — 2026-04-27

### Added
- `ColumnAlreadyExists` VM error — raised (and exported) when ALTER TABLE tries to
  add a column that already exists.
- `AlterTable` IR instruction dispatch — `_do_alter_table` handler calls
  `backend.add_column` and translates any `BackendError`.
- `_translate_backend_error` extended to map `be.ColumnAlreadyExists` to
  `ColumnAlreadyExists`.

## 0.8.0 — 2026-04-27

### Added — Phase 2: EXISTS / NOT EXISTS subquery expressions

- **`RunExistsSubquery` dispatch** — the VM's main dispatch loop now handles
  `RunExistsSubquery` instructions.  The handler calls `execute(ins.sub_program,
  st.backend)` in a sub-state, then pushes `True` onto the value stack if the
  result set contains at least one row, `False` otherwise.  Because `NOT
  EXISTS` is represented as `UnaryExpr(NOT, ExistsSubquery(...))`, the
  existing `NOT` unary instruction handles inversion without any extra VM
  logic.

## 0.7.0 — 2026-04-27

### Added — Date/time scalar functions + scalar MAX/MIN

- **`DATE(timevalue [, modifier...])`** — returns ISO-8601 date string
  (`YYYY-MM-DD`).  Accepts `'now'`, ISO-8601 strings, Julian Day floats,
  and Unix epoch integers as time values.

- **`TIME(timevalue [, modifier...])`** — returns time string (`HH:MM:SS`).

- **`DATETIME(timevalue [, modifier...])`** — returns combined datetime string
  (`YYYY-MM-DD HH:MM:SS`).

- **`JULIANDAY(timevalue [, modifier...])`** — returns Julian Day Number as
  float.  `JULIANDAY('2000-01-01')` → `2451544.5` (well-known constant).

- **`UNIXEPOCH(timevalue [, modifier...])`** — returns Unix epoch seconds as
  integer.  `UNIXEPOCH('1970-01-01')` → `0`.

- **`STRFTIME(format, timevalue [, modifier...])`** — formats a time value
  using C-style format specifiers.  Supports all standard `%Y`, `%m`, `%d`,
  `%H`, `%M`, `%S` plus SQLite extensions `%f` (SS.SSS), `%s` (epoch
  integer), `%J` (Julian Day), `%j` (day of year), `%W` (week number).

- **Modifiers supported** for all six functions:
  `+N days/hours/minutes/seconds/months/years`,
  `-N days/…`, `start of day/month/year`, `localtime`, `utc`.
  Leap-year clamping applied when adding months (`2024-01-31 + 1 month` →
  `2024-02-29`).

- **`MAX(a, b)`** (scalar form) — returns the greater of two arguments using
  SQLite type ordering.  NULL is treated as "less than everything":
  `MAX(1, NULL)` → `1`, `MAX(NULL, NULL)` → `NULL`.

- **`MIN(a, b)`** (scalar form) — returns the lesser of two arguments.
  `MIN(1, NULL)` → `NULL`, `MIN(NULL, NULL)` → `NULL`.

  The scalar two-argument forms are dispatched via `CallScalar` and do not
  conflict with the single-argument aggregate forms handled by
  `InitAgg`/`FinalizeAgg` opcodes.

- **`tests/test_scalar_functions.py`** — 69 new tests in `TestScalarMinMax`
  and `TestDateTimeFunctions` classes covering: format correctness, NULL
  propagation, known constants (`JULIANDAY('2000-01-01')` → `2451544.5`,
  `UNIXEPOCH('1970-01-01')` → `0`), all six modifier types, leap-year
  clamping, compound modifiers, and `STRFTIME` specifiers including `%f`,
  `%s`, `%j`.

## 0.6.0 — 2026-04-23

### Changed — Phase 9.7: Composite (multi-column) automatic index support (IX-8)

- **`_do_open_index_scan` tuple-unpack fix** — index scan bounds (`lo`, `hi`)
  are now tuples (`tuple[object, ...] | None`) instead of scalars.  The handler
  previously wrapped scalar bounds with `[ins.lo]`; it now calls `list(ins.lo)`
  directly.  This is the minimal change needed to support composite multi-column
  scans: the backend's `scan_index` receives a list of values, one per leading
  index column, rather than always a 1-element list.

## 0.5.0 — 2026-04-23

### Added

- **`QueryEvent` dataclass** — emitted by `execute()` after each SELECT scan
  via the new `event_cb` callback.  Fields:
  - `table` — the table that was scanned.
  - `filtered_columns` — column names in the WHERE predicate (pre-populated
    by the caller).
  - `rows_scanned` — total rows advanced through during the scan.
  - `rows_returned` — rows emitted to the result set via `EmitRow`.
  - `used_index` — the index name used for an index scan, or `None` for a
    full-table scan.
  - `duration_us` — wall-clock execution time in microseconds.
- **`execute()` new keyword parameters**:
  - `event_cb: Callable[[QueryEvent], None] | None` — callback invoked once
    after execution when a scan table was observed.  Replaces the global
    `set_event_listener` hook for per-execution callbacks.
  - `filtered_columns: list[str] | None` — caller-supplied column names
    forwarded into the emitted `QueryEvent`.
- **`QueryEvent` exported** from `sql_vm.__init__` and included in
  `__all__`.
- **Scan telemetry in `_VmState`** — four new fields (`scan_table`,
  `scan_index`, `rows_scanned`, `rows_returned`) accumulate metrics during
  execution.  Updated by `_do_open`, `_do_open_index_scan`, `_do_advance`,
  and the `EmitRow` handler.

## 0.4.0 — 2026-04-21

### Added

- **`RunSubquery` instruction dispatch** — new `_do_run_subquery` handler
  executes a derived-table sub-program against the same backend as the outer
  query and materialises its result rows.

- **`_SubqueryCursor` class** — an in-memory `RowIterator` backed by pre-
  materialised rows from a `RunSubquery` execution.  Stored under the derived
  table's `cursor_id` in `_VmState.cursors` so the outer scan loop's
  `AdvanceCursor` / `LoadColumn` / `CloseScan` instructions work transparently
  without any special-casing in those paths.

### Fixed

- **`row_buffer` changed from `dict[str, SqlValue]` to `list[SqlValue]`** —
  the previous dict-based buffer assigned each emitted column by name, causing
  duplicate column names (e.g. two columns both called `v` in a CROSS JOIN of
  two subqueries) to silently overwrite each other.  The new list-based buffer
  appends values positionally; `EmitRow` converts it directly to a tuple so
  column positions always match the declared result schema.  `_do_scan_all_columns`
  similarly appends values rather than keying by name.

- **`cursors` field type widened** to `dict[int, RowIterator]` (was `dict[int,
  Cursor]`) to accommodate `_SubqueryCursor` alongside normal backend cursors.

### Tests

- `tests/test_tier2_features.py` — 34 new end-to-end integration tests covering
  derived tables (`RunSubquery`), CROSS JOINs, CASE expressions (searched and
  simple), chained UNION/INTERSECT/EXCEPT, explicit transaction control, and
  subqueries in WHERE (scalar subqueries and IN subqueries).

## 0.3.0 — 2026-04-21

### Added

- **`UNION` / `INTERSECT` / `EXCEPT` execution** — full support for all six
  set-operation variants:
  - `Union ALL` — both sides are appended directly to `result_buffer`.
  - `Union DISTINCT` — `DistinctResult` deduplicates the merged buffer.
  - `CaptureLeftResult` instruction — saves `result_buffer.rows` to a new
    `left_result` field on `_VmState` and clears the buffer, allowing the right
    side to fill the buffer independently.
  - `IntersectResult(all)` — set semantics (distinct rows in both sides) when
    `all=False`; bag semantics with `min(left_count, right_count)` copies when
    `all=True`.
  - `ExceptResult(all)` — set semantics (rows in left but not right) when
    `all=False`; bag semantics with `max(0, left_count − right_count)` copies
    when `all=True`.

- **`INSERT … SELECT` execution** — `InsertFromResult` instruction drains every
  row from `result_buffer` into `backend.insert()`, clears the buffer, and
  records the count in `rows_affected`.

- **Explicit transaction support** via three new VM instructions:
  - `BeginTransaction` — calls `backend.begin_transaction()`, stores the handle
    in `_VmState.transaction_handle`.  Raises `TransactionError` if a
    transaction is already active (detected via both `_VmState.transaction_handle`
    and `backend.current_transaction()`).
  - `CommitTransaction` — resolves the handle from `_VmState.transaction_handle`
    or `backend.current_transaction()`, calls `backend.commit_transaction()`.
    Raises `TransactionError` if no active transaction exists.
  - `RollbackTransaction` — same handle-resolution strategy as commit, calls
    `backend.rollback_transaction()`.

- **`TransactionError(message)`** — new `VmError` subclass raised for nested
  `BEGIN`, `COMMIT`/`ROLLBACK` without `BEGIN`, etc.

- **`TransactionError` exported** from `sql_vm.__init__`.

### Tests

- `tests/test_tier1_features.py` — 41 new integration tests in seven classes:
  `TestUnion` (7), `TestIntersect` (7), `TestExcept` (9),
  `TestInsertSelect` (5), `TestTransactions` (5),
  `TestTransactionErrors` (4), `TestSetOpEdgeCases` (3).
- VM total: **305 tests, 83.38% coverage**.

## 0.2.0 — 2026-04-20

### Added

- **Built-in scalar functions** — new `scalar_functions` module with 40+ SQLite-compatible
  functions organised into categories:
  - *NULL-handling*: `COALESCE`, `IFNULL`, `NULLIF`, `IIF`
  - *Type inspection/casting*: `TYPEOF`, `CAST` (all SQLite affinity targets)
  - *Numeric*: `ABS`, `ROUND`, `CEIL`/`CEILING`, `FLOOR`, `SIGN`, `MOD`
  - *Math (SQLite 3.35+)*: `SQRT`, `POW`/`POWER`, `LOG`/`LN`, `LOG2`, `LOG10`, `EXP`,
    `PI`, `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`, `DEGREES`, `RADIANS`
  - *String*: `UPPER`, `LOWER`, `LENGTH`/`LEN`, `TRIM`, `LTRIM`, `RTRIM`,
    `SUBSTR`/`SUBSTRING`, `REPLACE`, `INSTR`, `HEX`, `UNHEX`, `QUOTE`, `CHAR`, `UNICODE`,
    `ZEROBLOB`, `SOUNDEX`
  - *Formatting*: `PRINTF`/`FORMAT` (SQLite subset: `%d`, `%f`, `%e`, `%g`, `%s`, `%q`,
    `%Q`, `%%`)
  - *Utility*: `RANDOM`, `RANDOMBLOB`, `LAST_INSERT_ROWID`

- **`CallScalar` dispatch in VM** — new `_do_call_scalar` handler in `_dispatch`
  dispatches any `CallScalar` IR instruction to the scalar function registry.  Arguments
  are popped left-to-right from the stack; the result is pushed back.

- **New error classes** (`sql_vm.errors`):
  - `UnsupportedFunction(name)` — unknown function name at runtime
  - `WrongNumberOfArguments(name, expected, got)` — arity mismatch

- **Public API additions** (`sql_vm.__init__`): `UnsupportedFunction`,
  `WrongNumberOfArguments`, `call_scalar`

- **`[tool.uv.sources]`** in `pyproject.toml` — all four local transitive dependencies
  (`sql-backend`, `sql-codegen`, `sql-planner`, `sql-optimizer`) declared as editable
  path sources so `uv run` and `uv sync` resolve correctly without PyPI.

- **200 new tests** in `tests/test_scalar_functions.py` covering every function category,
  NULL propagation, edge cases, and VM end-to-end integration via `CallScalar`.

## 0.1.0 — 2026-04-19

Initial release.

- Dispatch-loop VM `execute(program, backend)` returning a `QueryResult`
- Stack machine with separate row_buffer, cursors, and agg_table state
- Full arithmetic, logic, and comparison semantics with SQL three-valued
  NULL logic (AND/OR truth tables, NULL propagation through arithmetic
  and comparisons)
- Scan, AdvanceCursor, CloseScan — paired with label-driven loop exit
- BeginRow / EmitColumn / EmitRow for result assembly
- InitAgg / UpdateAgg / FinalizeAgg / SaveGroupKey / LoadGroupKey for
  GROUP BY and HAVING
- SortResult / LimitResult / DistinctResult post-processing
- DML: InsertRow, UpdateRows, DeleteRows
- DDL: CreateTable, DropTable
- Typed error hierarchy rooted at `VmError`

