"""Oracle tests for newly added datetime modifiers and strftime specifiers.

This file pins behaviour for additions to ``sql_vm.scalar_functions``
against real ``sqlite3`` so we know we match SQLite's exact semantics:

1. **Timezone offset modifiers** — ``+HH:MM``, ``-HH:MM``,
   ``+HH:MM:SS`` shift the datetime by the given offset.

2. **``auto`` modifier no-op** — previously caused NULL propagation
   (unrecognised modifier).  Mini-sqlite dispatches numeric time values
   by Python type, so ``auto`` is a semantic no-op here; accepting it
   prevents NULL for SQL written against SQLite 3.46+.

3. **``%P`` strftime specifier** — lowercase ``am``/``pm``.  Python's
   macOS libc returns the literal ``'P'`` for ``strftime('%P')``, so we
   pre-process it ourselves.  Without that fix mini-sqlite would diverge
   from real SQLite on macOS CI runners.

All assertions compare against the real ``sqlite3`` module directly.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _one(sql: str):
    return mini_sqlite.connect(":memory:").execute(sql).fetchone()[0]


def _ref(sql: str):
    return sqlite3.connect(":memory:").execute(sql).fetchone()[0]


def _check(sql: str) -> None:
    m, r = _one(sql), _ref(sql)
    assert m == r, f"SQL: {sql!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Timezone offset modifiers
# ---------------------------------------------------------------------------


class TestTimezoneOffsetModifier:
    def test_positive_hours(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', '+02:00')")

    def test_negative_hours_minutes(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', '-05:30')")

    def test_with_seconds(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', '+02:30:45')")

    def test_zero_offset(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', '+00:00')")

    def test_day_rollback(self) -> None:
        _check("SELECT datetime('2024-03-15 02:00:00', '-05:00')")

    def test_day_rollforward(self) -> None:
        _check("SELECT datetime('2024-03-15 22:00:00', '+04:00')")

    def test_with_date_function(self) -> None:
        # Applying a timezone offset to a midnight time should not shift
        # the date (since 00:00 + 12:00 stays on the same day).
        _check("SELECT date('2024-03-15', '+12:00')")

    def test_chained_with_day_offset(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', '+02:00', '+1 day')")


# ---------------------------------------------------------------------------
# auto / julianday no-op modifiers
# ---------------------------------------------------------------------------


class TestAutoModifier:
    def test_auto_does_not_null(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', 'auto')")

    def test_auto_chained_with_offset(self) -> None:
        _check("SELECT datetime('2024-03-15 14:30:00', 'auto', '+1 day')")

    def test_unrecognised_modifier_still_null(self) -> None:
        # Make sure we didn't accidentally accept arbitrary garbage.
        # SQLite returns NULL for unknown modifiers.
        assert _one("SELECT datetime('2024-03-15 14:30:00', 'totally_bogus_x')") is None
        assert _ref("SELECT datetime('2024-03-15 14:30:00', 'totally_bogus_x')") is None


# ---------------------------------------------------------------------------
# strftime %P (lowercase am/pm)
# ---------------------------------------------------------------------------


class TestStrftimeLowerCaseAmPm:
    def test_pm_afternoon(self) -> None:
        _check("SELECT strftime('%P', '2024-03-15 14:30:00')")

    def test_am_morning(self) -> None:
        _check("SELECT strftime('%P', '2024-03-15 06:30:00')")

    def test_am_midnight(self) -> None:
        _check("SELECT strftime('%P', '2024-03-15 00:00:00')")

    def test_pm_noon(self) -> None:
        _check("SELECT strftime('%P', '2024-03-15 12:00:00')")

    def test_combined_format(self) -> None:
        _check("SELECT strftime('%I:%M %P', '2024-03-15 14:30:00')")

    def test_distinct_from_uppercase(self) -> None:
        # %P is am/pm; %p is AM/PM
        mini_lower = _one("SELECT strftime('%P', '2024-03-15 14:30:00')")
        mini_upper = _one("SELECT strftime('%p', '2024-03-15 14:30:00')")
        ref_lower = _ref("SELECT strftime('%P', '2024-03-15 14:30:00')")
        ref_upper = _ref("SELECT strftime('%p', '2024-03-15 14:30:00')")
        assert mini_lower == ref_lower == "pm"
        assert mini_upper == ref_upper == "PM"
