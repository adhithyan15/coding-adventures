"""Oracle tests for ``ORDER BY ... NULLS FIRST | NULLS LAST`` (SQLite 3.30+).

Before this fix the parser rejected the explicit ``NULLS FIRST / NULLS
LAST`` clause with::

    Parse error at ...: Unexpected token: 'NULLS'

The planner already supported per-key NULL placement via
``SortKey.nulls_first``; this PR exposes that capability through the
grammar.  Without the explicit clause, SQLite's default applies:

- ASC ``ORDER BY x``       → NULLs first
- DESC ``ORDER BY x DESC`` → NULLs last

The new clause overrides those defaults.  Tests cover both override
directions, combined with ASC/DESC, multi-key ORDER BY, and
regression-guard cases where the clause is omitted.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(setup: list[str], query: str) -> None:
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# NULLS FIRST / LAST with default (no ASC/DESC)
# ---------------------------------------------------------------------------


class TestNullsPlacementOnImplicitAsc:
    SETUP = [
        "CREATE TABLE t (id INTEGER, val INTEGER)",
        "INSERT INTO t VALUES (1, 10), (2, NULL), (3, 20), (4, NULL)",
    ]

    def test_nulls_first(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val NULLS FIRST")

    def test_nulls_last(self) -> None:
        # SQLite default for ASC is NULLs first; this forces NULLs to the end.
        _check(self.SETUP, "SELECT * FROM t ORDER BY val NULLS LAST")


# ---------------------------------------------------------------------------
# NULLS FIRST / LAST combined with ASC / DESC
# ---------------------------------------------------------------------------


class TestNullsPlacementWithDirection:
    SETUP = [
        "CREATE TABLE t (id INTEGER, val INTEGER)",
        "INSERT INTO t VALUES (1, 10), (2, NULL), (3, 20), (4, NULL)",
    ]

    def test_asc_nulls_first(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val ASC NULLS FIRST")

    def test_asc_nulls_last(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val ASC NULLS LAST")

    def test_desc_nulls_first(self) -> None:
        # DESC default is NULLs last; this puts them first.
        _check(self.SETUP, "SELECT * FROM t ORDER BY val DESC NULLS FIRST")

    def test_desc_nulls_last(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val DESC NULLS LAST")


# ---------------------------------------------------------------------------
# Multi-key ORDER BY with per-key NULL placement
# ---------------------------------------------------------------------------


class TestMultiKeyNullsPlacement:
    def test_two_keys_independent_null_placement(self) -> None:
        _check(
            [
                "CREATE TABLE t (a INTEGER, b INTEGER, c INTEGER)",
                "INSERT INTO t VALUES "
                "(1, 1, NULL), "
                "(1, NULL, 100), "
                "(2, 2, 200), "
                "(NULL, 3, 300)",
            ],
            "SELECT * FROM t ORDER BY a NULLS LAST, b NULLS FIRST",
        )

    def test_three_keys_mixed_directions_and_nulls(self) -> None:
        _check(
            [
                "CREATE TABLE t (a INTEGER, b INTEGER, c INTEGER)",
                "INSERT INTO t VALUES "
                "(1, NULL, 1), "
                "(1, NULL, 2), "
                "(NULL, 5, 1), "
                "(2, 1, NULL)",
            ],
            "SELECT * FROM t "
            "ORDER BY a ASC NULLS LAST, b DESC NULLS FIRST, c NULLS LAST",
        )


# ---------------------------------------------------------------------------
# Regression — omitting the clause still works (no parser regression)
# ---------------------------------------------------------------------------


class TestImplicitNullsPlacement:
    SETUP = [
        "CREATE TABLE t (val INTEGER)",
        "INSERT INTO t VALUES (10), (NULL), (20), (NULL)",
    ]

    def test_default_asc(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val")

    def test_default_desc(self) -> None:
        _check(self.SETUP, "SELECT * FROM t ORDER BY val DESC")


# ---------------------------------------------------------------------------
# Combined with positional and SELECT *
# ---------------------------------------------------------------------------


class TestNullsPlacementCompositions:
    def test_positional_with_nulls_last(self) -> None:
        _check(
            [
                "CREATE TABLE u (a INTEGER, b INTEGER)",
                "INSERT INTO u VALUES (1, 10), (2, NULL), (3, 20), (4, NULL)",
            ],
            "SELECT * FROM u ORDER BY 2 NULLS LAST",
        )

    def test_text_column_nulls_last(self) -> None:
        _check(
            [
                "CREATE TABLE w (id INTEGER, name TEXT)",
                "INSERT INTO w VALUES (1, 'alice'), (2, NULL), (3, 'bob'), (4, NULL)",
            ],
            "SELECT * FROM w ORDER BY name NULLS LAST",
        )
