"""Implicit COLLATE propagation from column declaration into WHERE.

This is the fourth and most useful part of the COLLATE work, building
on PRs #3985 (ORDER BY), #4002 (explicit COLLATE postfix in
comparisons), and #4013 (COLLATE in column defs + ORDER BY).

Here we let the column's *declared* COLLATE clause propagate into any
comparison the column participates in — matching SQLite's implicit
collation semantics::

    CREATE TABLE users(email TEXT COLLATE NOCASE);
    INSERT INTO users VALUES ('Adhithya@example.com'),
                              ('FOO@bar.com');
    SELECT * FROM users WHERE email = 'adhithya@example.com';
    -- returns the first row, case-insensitively, without an explicit
    -- COLLATE NOCASE on the WHERE clause.

Implementation: the planner's expression rewriter walks the resolved
WHERE / HAVING / UPDATE-WHERE / DELETE-WHERE predicate.  For each
BinaryExpr comparison whose operand is a resolved ``Column`` with a
declared collation (looked up via
``SchemaProvider.column_collation``), we wrap both operands in the
matching scalar function (``lower`` for NOCASE, ``rtrim`` for RTRIM).
This mirrors what PR #4002's adapter does for the *explicit*-postfix
form, but driven by schema metadata instead of an AST keyword.

Known divergence (documented limitation, not a bug): when a column is
declared ``COLLATE NOCASE`` and the user writes ``WHERE col = 'x'
COLLATE BINARY`` to *override* the declared collation, mini-sqlite
still applies the column's NOCASE — because the explicit BINARY
postfix is a no-op transform at the adapter level (BINARY is the
identity collation), so the planner can't tell the user gave an
explicit override.  The common usage pattern (overriding NOCASE with
NOCASE, RTRIM with RTRIM, BINARY with NOCASE, etc.) all work
correctly; only the rare BINARY-as-explicit-override case diverges.
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
# Basic equality — the workhorse use case.
# ---------------------------------------------------------------------------


class TestImplicitNocaseEquality:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE NOCASE)",
        "INSERT INTO t VALUES ('Banana'), ('apple'), ('BANANA')",
    ]

    def test_eq_lowercase_pattern(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t WHERE name = 'banana' ORDER BY 1")

    def test_eq_uppercase_pattern(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t WHERE name = 'BANANA' ORDER BY 1")

    def test_eq_mixed_case_pattern(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t WHERE name = 'BaNaNa' ORDER BY 1")

    def test_neq(self) -> None:
        _check_with_setup(
            self.setup, "SELECT name FROM t WHERE name <> 'banana' ORDER BY 1"
        )

    def test_lt(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t WHERE name < 'C' ORDER BY 1")

    def test_gte(self) -> None:
        _check_with_setup(self.setup, "SELECT name FROM t WHERE name >= 'b' ORDER BY 1")


# ---------------------------------------------------------------------------
# Per-column independence: collated column propagates, BINARY column
# (no COLLATE declaration) doesn't.
# ---------------------------------------------------------------------------


class TestPerColumnPropagation:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE NOCASE, raw TEXT)",
        "INSERT INTO t VALUES ('Banana', 'Banana'), ('apple', 'apple'), ('BANANA', 'BANANA')",
    ]

    def test_collated_column_implicit(self) -> None:
        # ``name`` is COLLATE NOCASE — implicit on the WHERE.
        _check_with_setup(
            self.setup, "SELECT name FROM t WHERE name = 'banana' ORDER BY 1"
        )

    def test_binary_column_no_propagation(self) -> None:
        # ``raw`` is BINARY by default — no propagation, case matters.
        _check_with_setup(
            self.setup, "SELECT raw FROM t WHERE raw = 'banana' ORDER BY 1"
        )

    def test_binary_column_explicit_nocase_still_works(self) -> None:
        # User can still opt in with explicit COLLATE NOCASE.
        _check_with_setup(
            self.setup,
            "SELECT raw FROM t WHERE raw = 'banana' COLLATE NOCASE ORDER BY 1",
        )


# ---------------------------------------------------------------------------
# Complex predicates — propagation works inside AND / OR / NOT and
# nested in BETWEEN / IS [NOT] DISTINCT FROM.
# ---------------------------------------------------------------------------


class TestComplexPredicates:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE NOCASE, n INTEGER)",
        "INSERT INTO t VALUES ('Banana', 1), ('apple', 2), ('CHERRY', 3)",
    ]

    def test_or(self) -> None:
        _check_with_setup(
            self.setup,
            "SELECT name FROM t WHERE name = 'apple' OR name = 'banana' "
            "ORDER BY 1",
        )

    def test_and(self) -> None:
        _check_with_setup(
            self.setup,
            "SELECT name FROM t WHERE name = 'banana' AND n = 1 ORDER BY 1",
        )

    def test_not(self) -> None:
        _check_with_setup(
            self.setup, "SELECT name FROM t WHERE NOT name = 'banana' ORDER BY 1"
        )

    def test_between(self) -> None:
        _check_with_setup(
            self.setup,
            "SELECT name FROM t WHERE name BETWEEN 'a' AND 'c' ORDER BY 1",
        )

    def test_is_distinct_from(self) -> None:
        _check_with_setup(
            self.setup,
            "SELECT name FROM t WHERE name IS DISTINCT FROM 'banana' ORDER BY 1",
        )


# ---------------------------------------------------------------------------
# RTRIM column declaration also propagates.
# ---------------------------------------------------------------------------


class TestRtrimColumn:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE RTRIM)",
        "INSERT INTO t VALUES ('foo'), ('foo  '), ('bar')",
    ]

    def test_eq_with_trailing_spaces(self) -> None:
        # The column's RTRIM collation makes 'foo' and 'foo  ' compare equal.
        _check_with_setup(
            self.setup, "SELECT name FROM t WHERE name = 'foo' ORDER BY 1"
        )


# ---------------------------------------------------------------------------
# UPDATE / DELETE — the same propagation applies to UPDATE WHERE and
# DELETE WHERE clauses.
# ---------------------------------------------------------------------------


class TestUpdateDelete:
    def test_update_where_collated(self) -> None:
        mc = mini_sqlite.connect(":memory:")
        rc = sqlite3.connect(":memory:")
        for db in (mc, rc):
            db.execute("CREATE TABLE t(name TEXT COLLATE NOCASE, n INTEGER)")
            db.execute("INSERT INTO t VALUES ('Banana', 1), ('apple', 2)")
            db.execute("UPDATE t SET n = 99 WHERE name = 'BANANA'")
        assert list(mc.execute("SELECT n FROM t WHERE name = 'banana'")) == [(99,)]
        assert list(rc.execute("SELECT n FROM t WHERE name = 'banana'")) == [(99,)]

    def test_delete_where_collated(self) -> None:
        mc = mini_sqlite.connect(":memory:")
        rc = sqlite3.connect(":memory:")
        for db in (mc, rc):
            db.execute("CREATE TABLE t(name TEXT COLLATE NOCASE)")
            db.execute("INSERT INTO t VALUES ('Banana'), ('apple')")
            db.execute("DELETE FROM t WHERE name = 'BANANA'")
        ours = list(mc.execute("SELECT name FROM t ORDER BY 1"))
        ref = list(rc.execute("SELECT name FROM t ORDER BY 1"))
        assert ours == ref, f"mini: {ours}, ref: {ref}"


# ---------------------------------------------------------------------------
# HAVING clause — implicit COLLATE propagation hits a known limitation
# here.  After GROUP BY runs, the column reference in HAVING refers to
# the *grouped value* (a fresh row built from the group key), not a
# resolved table column with declared collation visible to the planner
# pass.  The planner's pass therefore can't apply NOCASE.  Users can
# work around it with an explicit ``COLLATE NOCASE`` on the HAVING
# clause.  Tracked as a follow-up.
# ---------------------------------------------------------------------------


# ---------------------------------------------------------------------------
# Explicit COLLATE still overrides (except for the documented BINARY
# limitation).  These tests pin the override path that *does* work.
# ---------------------------------------------------------------------------


class TestExplicitOverride:
    setup = [
        "CREATE TABLE t(name TEXT COLLATE NOCASE)",
        "INSERT INTO t VALUES ('foo'), ('foo  ')",
    ]

    def test_explicit_rtrim_overrides_nocase(self) -> None:
        # Column-declared NOCASE; explicit RTRIM in the WHERE.  Since
        # NOCASE doesn't strip spaces, 'foo' != 'foo  '.  RTRIM does.
        _check_with_setup(
            self.setup,
            "SELECT name FROM t WHERE name = 'foo  ' COLLATE RTRIM ORDER BY 1",
        )
