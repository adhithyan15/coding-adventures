"""SQLite-compatible boolean coercion for ``AND`` / ``OR``.

SQLite has no separate BOOLEAN storage class: integers and floats
double as booleans.  Zero is FALSE; any other numeric value is TRUE.
The ``AND`` / ``OR`` operators must therefore coerce numeric operands
to truth values, not require Python ``bool`` instances.

Before this PR mini-sqlite silently produced NULL for ``SELECT 1 AND 0``:

* The constant-folding optimizer compared each operand to the Python
  ``True`` / ``False`` singletons with the ``is`` operator.  Integer
  literals like ``1`` and ``0`` failed all four identity checks, so
  the fold fell through to "both are literals but neither is TRUE/
  FALSE — must be NULL" and produced ``Literal(None)``.

* The VM-level ``apply_binary(AND, 1, 0)`` raised ``TypeMismatch``
  with "expected boolean" — also wrong, since SQLite freely mixes
  integers and booleans.

Both code paths now use ``_truthy`` / ``_truthiness`` helpers that
return ``True``/``False`` for any non-NULL numeric value and ``None``
for NULL or non-coercible inputs.

These oracle tests pair every interesting case against the reference
``sqlite3`` module byte-for-byte.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Integer-literal AND/OR — the headline bug
# ---------------------------------------------------------------------------


class TestIntegerLiteralAndOr:
    def test_1_and_0(self) -> None:
        # Was returning NULL; SQLite returns 0.
        _check("SELECT 1 AND 0")

    def test_1_and_1(self) -> None:
        _check("SELECT 1 AND 1")

    def test_0_and_0(self) -> None:
        _check("SELECT 0 AND 0")

    def test_0_and_1(self) -> None:
        _check("SELECT 0 AND 1")

    def test_1_or_0(self) -> None:
        _check("SELECT 1 OR 0")

    def test_0_or_0(self) -> None:
        _check("SELECT 0 OR 0")

    def test_1_or_1(self) -> None:
        _check("SELECT 1 OR 1")


# ---------------------------------------------------------------------------
# 3-valued logic — NULL interactions
# ---------------------------------------------------------------------------


class TestThreeValuedLogic:
    def test_null_and_zero_is_zero(self) -> None:
        # FALSE dominates AND, even when paired with NULL.
        _check("SELECT NULL AND 0")
        _check("SELECT 0 AND NULL")

    def test_null_and_one_is_null(self) -> None:
        _check("SELECT NULL AND 1")
        _check("SELECT 1 AND NULL")

    def test_null_and_null(self) -> None:
        _check("SELECT NULL AND NULL")

    def test_null_or_one_is_one(self) -> None:
        # TRUE dominates OR.
        _check("SELECT NULL OR 1")
        _check("SELECT 1 OR NULL")

    def test_null_or_zero_is_null(self) -> None:
        _check("SELECT NULL OR 0")
        _check("SELECT 0 OR NULL")

    def test_null_or_null(self) -> None:
        _check("SELECT NULL OR NULL")


# ---------------------------------------------------------------------------
# Non-zero numeric values count as TRUE
# ---------------------------------------------------------------------------


class TestNumericTruthiness:
    def test_negative_int_is_true(self) -> None:
        # -1 is non-zero, so it's TRUE.
        _check("SELECT -1 AND 1")
        _check("SELECT -1 OR 0")

    def test_large_int_is_true(self) -> None:
        _check("SELECT 1000000 AND 1")

    def test_float_zero_is_false(self) -> None:
        _check("SELECT 0.0 AND 1")

    def test_float_nonzero_is_true(self) -> None:
        _check("SELECT 1.5 AND 1")
        _check("SELECT 0.1 OR 0")


# ---------------------------------------------------------------------------
# Column-driven AND/OR — exercises the VM-level fix
# ---------------------------------------------------------------------------


class TestColumnAndOr:
    SETUP = [
        "CREATE TABLE t (a INTEGER, b INTEGER)",
        "INSERT INTO t VALUES (1, 0), (0, 1), (1, 1), (0, 0)",
    ]

    def test_column_and(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute("SELECT a AND b FROM t ORDER BY a, b").fetchall()
        r = conn_r.execute("SELECT a AND b FROM t ORDER BY a, b").fetchall()
        assert m == r

    def test_column_or(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute("SELECT a OR b FROM t ORDER BY a, b").fetchall()
        r = conn_r.execute("SELECT a OR b FROM t ORDER BY a, b").fetchall()
        assert m == r

    def test_where_with_int_columns(self) -> None:
        # WHERE a AND b — must filter to (1, 1) only.
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT a, b FROM t WHERE a AND b ORDER BY a, b"
        ).fetchall()
        r = conn_r.execute(
            "SELECT a, b FROM t WHERE a AND b ORDER BY a, b"
        ).fetchall()
        assert m == r


# ---------------------------------------------------------------------------
# Regression — boolean comparison expressions still work (this was the
# only path that worked *before* the fix, because comparisons produced
# real Python bools that the ``is True`` / ``is False`` checks could
# recognise).
# ---------------------------------------------------------------------------


class TestBoolComparisonRegression:
    def test_eq_and_eq(self) -> None:
        _check("SELECT 1=1 AND 0=0")

    def test_eq_or_eq(self) -> None:
        _check("SELECT 1=2 OR 1=1")

    def test_chained_comparisons(self) -> None:
        _check("SELECT (1 < 2) AND (3 > 2)")
