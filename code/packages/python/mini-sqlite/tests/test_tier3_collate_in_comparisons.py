"""``expr COLLATE name`` as a comparison-operand postfix.

This is the follow-up to PR #3985 (which added COLLATE in ORDER BY)
and PR #3533 (which added COLLATE in CREATE INDEX columns).  Here we
extend the postfix to **comparison operands** — the most common
real-world use case for COLLATE in SQLite:

    SELECT … FROM users WHERE email = 'Foo@Bar.com' COLLATE NOCASE
    SELECT … FROM t WHERE name BETWEEN 'a' AND 'M' COLLATE NOCASE
    SELECT 'A' IS DISTINCT FROM 'a' COLLATE NOCASE

SQLite's semantics: when a COLLATE clause is attached to either
operand of a comparison, the comparison itself uses that collation.
If both operands carry COLLATE, the LEFT side wins.  If neither does,
the comparison is BINARY (byte-for-byte).

Implementation strategy in mini-sqlite (pure adapter-level rewrite —
no planner / codegen / VM changes needed): when ``_comparison`` sees
a BinaryExpr being built and either side has a COLLATE clause, we
rewrite both operands by wrapping them in the matching scalar
function — ``lower()`` for NOCASE, ``rtrim()`` for RTRIM, identity
for BINARY (and any unknown collation name).

These oracle tests pin the behaviour byte-for-byte against stdlib
``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# NOCASE — case-insensitive equality, the canonical use case.
# ---------------------------------------------------------------------------


class TestNocaseEquality:
    def test_basic_eq(self) -> None:
        _check("SELECT 'A' = 'a' COLLATE NOCASE")

    def test_basic_neq(self) -> None:
        _check("SELECT 'A' <> 'b' COLLATE NOCASE")

    def test_lhs_collate(self) -> None:
        _check("SELECT 'A' COLLATE NOCASE = 'a'")

    def test_both_sides_collate(self) -> None:
        # Both sides have COLLATE; SQLite uses the LEFT one.  Since
        # both are NOCASE in this test, the answer is unambiguous.
        _check("SELECT 'A' COLLATE NOCASE = 'a' COLLATE NOCASE")

    def test_default_binary(self) -> None:
        # No COLLATE → BINARY comparison; 'A' != 'a'.
        _check("SELECT 'A' = 'a'")

    def test_lt(self) -> None:
        _check("SELECT 'A' < 'b' COLLATE NOCASE")

    def test_in_where(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),('B'),('c')) "
            "WHERE column1 = 'A' COLLATE NOCASE"
        )

    def test_in_where_lhs_collate(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),('B'),('c')) "
            "WHERE column1 COLLATE NOCASE = 'A'"
        )


# ---------------------------------------------------------------------------
# RTRIM — strip trailing spaces before comparing.
# ---------------------------------------------------------------------------


class TestRtrim:
    def test_eq(self) -> None:
        _check("SELECT 'foo' = 'foo   ' COLLATE RTRIM")

    def test_in_where(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('hello  '),('world')) "
            "WHERE column1 = 'hello' COLLATE RTRIM"
        )

    def test_lt(self) -> None:
        _check("SELECT 'foo  ' < 'foo z' COLLATE RTRIM")


# ---------------------------------------------------------------------------
# BETWEEN — collation propagates to both bounds.
# ---------------------------------------------------------------------------


class TestBetween:
    def test_between_nocase(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),('B'),('c'),('D'),('e')) "
            "WHERE column1 BETWEEN 'A' AND 'C' COLLATE NOCASE "
            "ORDER BY 1 COLLATE NOCASE"
        )

    def test_not_between_nocase(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),('B'),('c'),('D'),('e')) "
            "WHERE column1 NOT BETWEEN 'A' AND 'C' COLLATE NOCASE "
            "ORDER BY 1 COLLATE NOCASE"
        )


# ---------------------------------------------------------------------------
# IS DISTINCT FROM / IS NOT DISTINCT FROM — NULL-safe equality.
# ---------------------------------------------------------------------------


class TestIsDistinct:
    def test_is_distinct_from_nocase(self) -> None:
        _check("SELECT 'A' IS DISTINCT FROM 'a' COLLATE NOCASE")

    def test_is_not_distinct_from_nocase(self) -> None:
        _check("SELECT 'A' IS NOT DISTINCT FROM 'a' COLLATE NOCASE")

    def test_is_distinct_from_with_null(self) -> None:
        # NULL handling: COLLATE doesn't change the NULL-safety
        # semantics; IS NOT DISTINCT FROM treats both-NULL as equal.
        _check("SELECT NULL IS NOT DISTINCT FROM NULL")


# ---------------------------------------------------------------------------
# Composition with other clauses — make sure COLLATE plays nicely
# with WHERE / ORDER BY / multi-row results.
# ---------------------------------------------------------------------------


class TestComposition:
    def test_where_plus_order_by(self) -> None:
        _check(
            "SELECT column1 FROM "
            "(VALUES ('Banana'),('apple'),('Cherry'),('BANANA')) "
            "WHERE column1 = 'banana' COLLATE NOCASE "
            "ORDER BY column1 COLLATE NOCASE"
        )

    def test_lt_and_lte(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('A'),('b'),('C'),('d')) "
            "WHERE column1 <= 'B' COLLATE NOCASE "
            "ORDER BY 1 COLLATE NOCASE"
        )

    def test_multiple_predicates_each_collate(self) -> None:
        # Two independent comparisons in one WHERE — each has its own
        # COLLATE clause.  Both should fire.
        _check(
            "SELECT column1 FROM (VALUES ('a'),('B'),('c')) "
            "WHERE column1 > 'A' COLLATE NOCASE "
            "AND column1 < 'D' COLLATE NOCASE "
            "ORDER BY 1 COLLATE NOCASE"
        )


# ---------------------------------------------------------------------------
# Edge cases — NULL, integer operands, unknown collation.
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_null_propagates(self) -> None:
        # NULL = anything → NULL, regardless of COLLATE.
        _check("SELECT NULL = 'a' COLLATE NOCASE")

    def test_integer_eq_inert(self) -> None:
        # COLLATE on integer operands behaves as a no-op in SQLite —
        # the operands aren't strings.  Our ``lower()``-rewrite still
        # works because ``lower(1)`` coerces to text ``'1'``, and both
        # sides get the same transform, so ``1 = 1`` and
        # ``lower(1) = lower(1)`` give the same answer.
        _check("SELECT 1 = 1 COLLATE NOCASE")

    def test_unknown_collation_falls_through(self) -> None:
        # SQLite raises ``no such collation sequence: FOOBAR`` for an
        # unknown name.  Mini-sqlite is more lenient — it treats the
        # unknown name as BINARY (identity transform) for the same
        # reason ORDER BY does.  Verify we don't crash; the result
        # matches the BINARY comparison.
        rows = list(
            mini_sqlite.connect(":memory:").execute(
                "SELECT 'A' = 'a' COLLATE FOOBAR"
            )
        )
        assert rows == [(0,)]
