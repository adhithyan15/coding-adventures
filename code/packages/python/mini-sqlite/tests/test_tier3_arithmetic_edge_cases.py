"""Arithmetic edge cases — divide-by-zero, modulo with negative operands.

Three Python-vs-SQLite semantic differences uncovered by a gap audit:

1. **``x / 0`` returns NULL** in SQLite (its broader "arithmetic errors
   produce NULL" policy).  Mini-sqlite was raising ``DivisionByZero``
   which surfaced as ``OperationalError`` to the caller — fine for a
   strict language, but mismatched for SQL-compatible code that
   expects ``COALESCE(a/b, 0)`` to work on rows where ``b = 0``.

2. **``%`` operator follows C-style ``fmod``** — result sign matches
   the *dividend*, not the divisor.  Python's ``%`` is the opposite:
   ``-7 % 3 == 2`` (positive, matching divisor) in Python but ``-1``
   in SQLite (matching dividend).

3. **``%`` operator truncates floats to integers first**, then
   computes integer modulo, then casts back to float.  So
   ``7.5 % 2.0`` is ``1.0`` in SQLite (``int(7.5) % int(2.0) =
   7 % 2 = 1``), not ``1.5`` like ``math.fmod`` would give.  Note
   this is **different from the ``mod()`` scalar function**, which
   uses true ``fmod`` and produces ``1.5``.

The fixes span both the runtime VM (``sql_vm.operators._arithmetic``)
and the constant-folding optimizer
(``sql_optimizer.constant_folding._apply_binary``) so literal-only
expressions are folded with SQLite semantics rather than Python's.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Divide by zero → NULL
# ---------------------------------------------------------------------------


class TestDivideByZero:
    def test_int_div_zero(self) -> None:
        # Was raising DivisionByZero (OperationalError).
        _check("SELECT 7 / 0")

    def test_float_div_zero(self) -> None:
        _check("SELECT 7.0 / 0")

    def test_float_div_zero_float(self) -> None:
        _check("SELECT 7.0 / 0.0")

    def test_div_zero_in_expression(self) -> None:
        # COALESCE(NULL, fallback) is a common pattern with divide-by-zero
        # since SQLite intentionally returns NULL so callers can do this.
        _check("SELECT coalesce(7 / 0, -1)")


# ---------------------------------------------------------------------------
# Integer division truncates toward zero (not Python's floor)
# ---------------------------------------------------------------------------


class TestIntegerDivisionTruncation:
    def test_negative_dividend(self) -> None:
        # Python: -7 // 2 = -4 (floor); SQLite: -7 / 2 = -3 (truncate).
        _check("SELECT -7 / 2")

    def test_negative_divisor(self) -> None:
        _check("SELECT 7 / -2")

    def test_both_negative(self) -> None:
        _check("SELECT -7 / -2")


# ---------------------------------------------------------------------------
# Modulo: C-style (sign follows dividend), integer-first for floats
# ---------------------------------------------------------------------------


class TestModuloOperator:
    def test_positive_operands(self) -> None:
        _check("SELECT 7 % 3")

    def test_negative_dividend(self) -> None:
        # Python: -7 % 3 = 2; SQLite: -7 % 3 = -1.
        _check("SELECT -7 % 3")

    def test_negative_divisor(self) -> None:
        # Python: 7 % -3 = -2; SQLite: 7 % -3 = 1.
        _check("SELECT 7 % -3")

    def test_both_negative(self) -> None:
        _check("SELECT -7 % -3")

    def test_mod_by_zero(self) -> None:
        _check("SELECT 7 % 0")

    def test_float_mod_truncates_to_int(self) -> None:
        # SQLite: 7.5 % 2.0 → int(7.5) % int(2.0) = 7 % 2 = 1 → 1.0
        _check("SELECT 7.5 % 2.0")

    def test_float_mod_truncates_15_5(self) -> None:
        # 15.5 % 4.5 → int(15.5) % int(4.5) = 15 % 4 = 3 → 3.0
        _check("SELECT 15.5 % 4.5")

    def test_mixed_int_float(self) -> None:
        _check("SELECT 7 % 2.5")
        _check("SELECT 7.5 % 2")


# ---------------------------------------------------------------------------
# mod() scalar function uses true fmod (different from % operator!)
# ---------------------------------------------------------------------------


class TestModScalarFunction:
    def test_mod_returns_float(self) -> None:
        # Always float, even for integer inputs.
        _check("SELECT mod(7, 3)")

    def test_mod_negative_dividend(self) -> None:
        _check("SELECT mod(-7, 3)")

    def test_mod_negative_divisor(self) -> None:
        _check("SELECT mod(7, -3)")

    def test_mod_true_fmod_for_floats(self) -> None:
        # mod() uses true fmod — fractional remainder preserved.
        _check("SELECT mod(7.5, 2.0)")

    def test_mod_by_zero_null(self) -> None:
        _check("SELECT mod(10, 0)")


# ---------------------------------------------------------------------------
# Column-driven arithmetic — exercises the VM path (not just constant fold)
# ---------------------------------------------------------------------------


class TestColumnArithmetic:
    SETUP = [
        "CREATE TABLE t (a INTEGER, b INTEGER)",
        "INSERT INTO t VALUES (7, 3), (-7, 3), (7, -3), (-7, -3), (7, 0)",
    ]

    def test_column_div(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT a, b, a / b, a % b FROM t ORDER BY a, b"
        ).fetchall()
        r = conn_r.execute(
            "SELECT a, b, a / b, a % b FROM t ORDER BY a, b"
        ).fetchall()
        assert m == r
