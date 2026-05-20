"""Companion to the AND/OR integer-truthiness fix — same bug for ``NOT``.

PR #3737 fixed the binary ``AND`` / ``OR`` operators to coerce integer
operands to truth values, matching SQLite's "no separate BOOLEAN
class; zero is FALSE, anything else is TRUE" convention.  The unary
``NOT`` operator had the same bug at the VM level:
``apply_unary(NOT, 1)`` raised ``TypeMismatch`` instead of returning
``0``.

These oracle tests verify that ``NOT`` now coerces integer operands
through the same ``_truthiness`` helper that already handles
``AND``/``OR``, matching ``sqlite3`` byte-for-byte.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = mini_sqlite.connect(":memory:").execute(query).fetchall()
    r = sqlite3.connect(":memory:").execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# NOT on integer literals
# ---------------------------------------------------------------------------


class TestNotIntegerLiteral:
    def test_not_zero(self) -> None:
        _check("SELECT NOT 0")

    def test_not_one(self) -> None:
        _check("SELECT NOT 1")

    def test_not_large_int(self) -> None:
        _check("SELECT NOT 5")

    def test_not_negative_int(self) -> None:
        # -1 is non-zero, so NOT -1 → 0.
        _check("SELECT NOT -1")


class TestNotFloatLiteral:
    def test_not_zero_float(self) -> None:
        _check("SELECT NOT 0.0")

    def test_not_positive_float(self) -> None:
        _check("SELECT NOT 1.5")

    def test_not_negative_float(self) -> None:
        _check("SELECT NOT -0.1")


# ---------------------------------------------------------------------------
# NOT NULL — should still be NULL (3-valued logic)
# ---------------------------------------------------------------------------


class TestNotNull:
    def test_not_null(self) -> None:
        _check("SELECT NOT NULL")

    def test_not_null_aware(self) -> None:
        # NOT NULL must be NULL, never TRUE/FALSE.
        _check("SELECT NOT NULL IS NULL")  # NOT NULL → NULL; NULL IS NULL → 1


# ---------------------------------------------------------------------------
# NOT on column — exercises the VM path with non-constant values
# ---------------------------------------------------------------------------


class TestNotOnColumn:
    SETUP = [
        "CREATE TABLE t (b INTEGER)",
        "INSERT INTO t VALUES (1), (0), (NULL), (-1), (5)",
    ]

    def test_not_column(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT b, NOT b FROM t ORDER BY b"
        ).fetchall()
        r = conn_r.execute(
            "SELECT b, NOT b FROM t ORDER BY b"
        ).fetchall()
        assert m == r

    def test_where_not_column(self) -> None:
        conn_m = mini_sqlite.connect(":memory:")
        conn_r = sqlite3.connect(":memory:")
        for s in self.SETUP:
            conn_m.execute(s)
            conn_r.execute(s)
        m = conn_m.execute(
            "SELECT b FROM t WHERE NOT b ORDER BY b"
        ).fetchall()
        r = conn_r.execute(
            "SELECT b FROM t WHERE NOT b ORDER BY b"
        ).fetchall()
        assert m == r


# ---------------------------------------------------------------------------
# Compound expressions — NOT combined with AND/OR
# ---------------------------------------------------------------------------


class TestNotInCompound:
    def test_not_and(self) -> None:
        _check("SELECT NOT (1 AND 0)")
        _check("SELECT NOT 1 AND NOT 0")

    def test_not_or(self) -> None:
        _check("SELECT NOT (1 OR 0)")
        _check("SELECT NOT 1 OR NOT 0")

    def test_double_negation(self) -> None:
        _check("SELECT NOT NOT 1")
        _check("SELECT NOT NOT 0")
        _check("SELECT NOT NOT NULL")
