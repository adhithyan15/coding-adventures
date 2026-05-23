"""``COLLATE name`` as a column constraint in ``CREATE TABLE``.

This is the third part of the COLLATE work, building on PRs #3985
(ORDER BY ... COLLATE) and #4002 (COLLATE postfix in comparison
operands).  Here we let the user declare a column's *default*
collation at table-creation time::

    CREATE TABLE users (
        email TEXT COLLATE NOCASE,
        …
    );

After this, ``ORDER BY email`` against this table sorts case-
insensitively even without an explicit ``COLLATE NOCASE`` on the
ORDER BY clause — matching SQLite.  Explicit ``COLLATE`` on the
ORDER BY overrides the column's declaration.

These oracle tests pin the behaviour byte-for-byte against stdlib
``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check_with_setup(setup: list[str], query: str) -> None:
    mc = mini_sqlite.connect(":memory:")
    rc = sqlite3.connect(":memory:")
    for stmt in setup:
        mc.execute(stmt)
        rc.execute(stmt)
    m = list(mc.execute(query))
    r = list(rc.execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# CREATE TABLE parsing — the column-constraint COLLATE clause must parse
# alongside every combination of other constraints.
# ---------------------------------------------------------------------------


class TestCollateParses:
    def test_collate_only(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT COLLATE NOCASE)"
        )

    def test_collate_with_not_null(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT NOT NULL COLLATE NOCASE)"
        )

    def test_collate_with_default(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT COLLATE NOCASE DEFAULT 'anon')"
        )

    def test_collate_with_unique(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT UNIQUE COLLATE NOCASE)"
        )

    def test_collate_with_primary_key(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT PRIMARY KEY COLLATE NOCASE)"
        )

    def test_collate_rtrim(self) -> None:
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT COLLATE RTRIM)"
        )

    def test_collate_binary(self) -> None:
        # Explicit BINARY is a no-op (matches the default), but should parse.
        mini_sqlite.connect(":memory:").execute(
            "CREATE TABLE t(name TEXT COLLATE BINARY)"
        )


# ---------------------------------------------------------------------------
# Default-collation propagation: ``ORDER BY column`` against a column
# declared ``COLLATE NOCASE`` sorts case-insensitively without the
# user repeating COLLATE on the ORDER BY.
# ---------------------------------------------------------------------------


class TestOrderByPropagation:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE NOCASE)",
        "INSERT INTO t VALUES ('Banana'), ('apple'), ('CHERRY')",
    ]

    def test_implicit_nocase(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t ORDER BY name")

    def test_implicit_nocase_desc(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t ORDER BY name DESC")

    def test_explicit_binary_overrides(self) -> None:
        # Per SQLite, an explicit COLLATE on the ORDER BY beats the
        # column's declared collation.
        _check_with_setup(
            self.setup, "SELECT name FROM t ORDER BY name COLLATE BINARY"
        )

    def test_explicit_nocase_redundant(self) -> None:
        # Explicit COLLATE NOCASE produces the same result as the
        # implicit one (column-declared NOCASE).
        _check_with_setup(
            self.setup, "SELECT name FROM t ORDER BY name COLLATE NOCASE"
        )


# ---------------------------------------------------------------------------
# Multi-column table — some columns declared COLLATE NOCASE, others
# left as BINARY default.  The propagation is per-column.
# ---------------------------------------------------------------------------


class TestPerColumnDefault:
    setup = [
        "CREATE TABLE u(name TEXT COLLATE NOCASE, raw TEXT)",
        "INSERT INTO u VALUES ('Banana', 'Banana'), ('apple', 'apple'), ('CHERRY', 'CHERRY')",
    ]

    def test_collated_column_sorts_nocase(self) -> None:
        _check_with_setup(
            self.setup, "SELECT name FROM u ORDER BY name"
        )

    def test_binary_column_sorts_binary(self) -> None:
        # ``raw`` has no COLLATE declaration, so it uses BINARY default.
        _check_with_setup(
            self.setup, "SELECT raw FROM u ORDER BY raw"
        )


# ---------------------------------------------------------------------------
# RTRIM column collation propagates the same way.
# ---------------------------------------------------------------------------


class TestRtrimColumn:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE RTRIM)",
        "INSERT INTO t VALUES ('foo   '), ('bar'), ('foo')",
    ]

    def test_implicit_rtrim_sort(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t ORDER BY name")


# ---------------------------------------------------------------------------
# ALTER TABLE ADD COLUMN with COLLATE — the new column's declared
# collation should propagate the same way.
# ---------------------------------------------------------------------------


class TestAlterTable:
    def test_add_column_with_collate(self) -> None:
        # ALTER TABLE … ADD COLUMN with a COLLATE constraint should
        # accept the syntax and the new column should sort according
        # to its declared collation.
        setup = [
            "CREATE TABLE t(id INTEGER)",
            "ALTER TABLE t ADD COLUMN name TEXT COLLATE NOCASE",
            "INSERT INTO t(id, name) VALUES (1, 'Banana'), (2, 'apple'), (3, 'CHERRY')",
        ]
        _check_with_setup(setup, "SELECT name FROM t ORDER BY name")
