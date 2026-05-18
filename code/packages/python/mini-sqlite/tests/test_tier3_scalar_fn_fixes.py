"""
Oracle tests for recently-fixed scalar function divergences.

This file locks in three SQLite-compatibility fixes:

1. **``time()`` accepts time-only strings** — ``time('12:34:56')`` and
   ``time('12:34')`` previously returned NULL because the parser only
   recognised full date-time strings.

2. **``date(t, 'weekday N')`` modifier** — advances to the next day-of-week N
   (0=Sun, 6=Sat).  Previously not implemented.

3. **``log(x)`` is base-10, not natural** — SQLite's ``log()`` takes a
   base-10 logarithm; ``ln()`` is the natural logarithm.  Mini-sqlite
   previously aliased ``log = ln``, which silently returned wrong values.

4. **``hex(N)`` operates on the decimal string** — SQLite hex-encodes the
   ASCII bytes of an integer's decimal representation, not its binary form.
   ``hex(255)`` returns ``"323535"`` (the bytes of ``"255"``), not the
   8-byte big-endian encoding.

Every test runs both engines and asserts byte-for-byte parity.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(sql: str):
    """Return ``(mini_result, ref_result)`` for *sql*."""
    mini = mini_sqlite.connect(":memory:").execute(sql).fetchone()
    ref = sqlite3.connect(":memory:").execute(sql).fetchone()
    return mini, ref


def _check(sql: str) -> None:
    m, r = _both(sql)
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# time() accepts time-only strings
# ---------------------------------------------------------------------------


def test_time_hh_mm_ss():
    _check("SELECT time('12:34:56')")


def test_time_hh_mm():
    _check("SELECT time('12:34')")


def test_time_fractional_seconds():
    _check("SELECT time('12:34:56.789')")


def test_time_with_hour_modifier():
    _check("SELECT time('12:00:00', '+1 hour')")


def test_time_zero():
    _check("SELECT time('00:00:00')")


def test_strftime_on_time_only_string():
    _check("SELECT strftime('%H:%M:%S', '12:34:56')")


def test_strftime_hour_minute_on_time_string():
    _check("SELECT strftime('%H', '23:45:00')")


# ---------------------------------------------------------------------------
# weekday N modifier
# ---------------------------------------------------------------------------


def test_weekday_advance_friday():
    # 2026-01-15 is Thursday.  Next Friday (weekday 5) → 2026-01-16.
    _check("SELECT date('2026-01-15', 'weekday 5')")


def test_weekday_advance_sunday():
    # Next Sunday (weekday 0).
    _check("SELECT date('2026-01-15', 'weekday 0')")


def test_weekday_same_day():
    # 2026-01-15 IS Thursday (weekday 4).  Same day → unchanged.
    _check("SELECT date('2026-01-15', 'weekday 4')")


def test_weekday_monday():
    # Next Monday (weekday 1).
    _check("SELECT date('2026-01-15', 'weekday 1')")


def test_weekday_with_offset():
    _check("SELECT date('2026-01-15', '+7 days', 'weekday 0')")


# ---------------------------------------------------------------------------
# log() is base-10 in SQLite
# ---------------------------------------------------------------------------


def test_log_is_base_10():
    _check("SELECT log(100)")  # → 2.0


def test_log_is_base_10_thousand():
    _check("SELECT log(1000)")  # → 3.0


def test_ln_is_natural():
    # LN(1) → 0.0 ; LN(e) → 1.0 — covered by an approximation here.
    _check("SELECT ln(1)")


def test_log_two_args_explicit_base():
    _check("SELECT log(2, 8)")  # log2(8) → 3.0


# ---------------------------------------------------------------------------
# hex() on integers/floats works on decimal-string bytes
# ---------------------------------------------------------------------------


def test_hex_integer_small():
    _check("SELECT hex(123)")  # → "313233"


def test_hex_integer_zero():
    _check("SELECT hex(0)")  # → "30"


def test_hex_integer_negative():
    _check("SELECT hex(-5)")  # → "2D35"  (the bytes of "-5")


def test_hex_text():
    _check("SELECT hex('AB')")  # → "4142"


def test_hex_null():
    _check("SELECT hex(NULL)")  # → "" (empty string)


def test_hex_blob():
    _check("SELECT hex(x'DEADBEEF')")  # → "DEADBEEF"
