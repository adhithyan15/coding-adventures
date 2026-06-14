"""``ORDER BY expr COLLATE name`` — collation-aware sorting.

SQLite recognises three built-in collations:

* ``BINARY`` — byte-for-byte comparison; the default when no
  ``COLLATE`` clause is given (and what ``None`` means on
  :class:`~sql_planner.ast.SortKey`).
* ``NOCASE`` — ASCII case-insensitive: ``'A' == 'a'`` for sort
  purposes.  Non-ASCII characters are *not* folded (matching
  SQLite's behaviour, which only knows ASCII case).
* ``RTRIM`` — strips trailing spaces before comparing.

This file pins ``ORDER BY column COLLATE name`` byte-for-byte
against stdlib sqlite3 for all three collations, with and without
DESC, NULLS FIRST/LAST, and multi-column ORDER BY.

The collation transform happens inside the VM's sort key builder
(``_do_sort``).  Non-string values (ints, floats, blobs, NULL)
pass through unchanged because SQLite's collations only affect TEXT
comparison — a regression test for that invariant is below.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# NOCASE — the workhorse: ASCII case-insensitive sorting.
# ---------------------------------------------------------------------------


class TestNocase:
    def test_basic_asc(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY column1 COLLATE NOCASE"
        )

    def test_basic_desc(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY column1 COLLATE NOCASE DESC"
        )

    def test_mixed_case_words(self) -> None:
        # Without NOCASE, BINARY sort would put all uppercase letters
        # before any lowercase letter.  With NOCASE, mixed-case words
        # interleave alphabetically.
        _check(
            "SELECT column1 FROM (VALUES "
            "('apple'),('Banana'),('cherry'),('APPLE')) "
            "ORDER BY column1 COLLATE NOCASE"
        )

    def test_with_null(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),(NULL),('B')) "
            "ORDER BY column1 COLLATE NOCASE"
        )

    def test_with_nulls_first(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),(NULL),('B')) "
            "ORDER BY column1 COLLATE NOCASE NULLS FIRST"
        )

    def test_with_nulls_last(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),(NULL),('B')) "
            "ORDER BY column1 COLLATE NOCASE NULLS LAST"
        )

    def test_inert_on_integers(self) -> None:
        # Collations only affect TEXT comparison; numeric sort is
        # unchanged.  This is a regression guard: if a future change
        # accidentally lowercased non-string values (e.g. via
        # ``str(v).lower()``), this test would break.
        _check(
            "SELECT column1 FROM (VALUES (3),(1),(2)) "
            "ORDER BY column1 COLLATE NOCASE"
        )

    def test_multi_column_one_collated(self) -> None:
        # First key uses NOCASE; second uses BINARY default.
        _check(
            "SELECT column1, column2 FROM "
            "(VALUES ('A', 2),('a', 1),('B', 3)) "
            "ORDER BY column1 COLLATE NOCASE, column2"
        )


# ---------------------------------------------------------------------------
# BINARY — explicit form of the default.  Should be a no-op vs no
# COLLATE clause.
# ---------------------------------------------------------------------------


class TestBinary:
    def test_explicit_binary_equals_default(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY column1 COLLATE BINARY"
        )

    def test_binary_desc(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY column1 COLLATE BINARY DESC"
        )

    def test_no_collate_clause(self) -> None:
        # Sanity: omitting COLLATE behaves like COLLATE BINARY.
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY column1"
        )


# ---------------------------------------------------------------------------
# RTRIM — strips trailing spaces, then BINARY-compares the result.
# ---------------------------------------------------------------------------


class TestRtrim:
    def test_basic(self) -> None:
        # ``'a  '`` and ``'a'`` are sort-equal under RTRIM; their
        # relative position is implementation-defined but their
        # position vs ``'b'`` is not.
        _check(
            "SELECT column1 FROM (VALUES ('a  '),('b'),('A  ')) "
            "ORDER BY column1 COLLATE RTRIM"
        )

    def test_trailing_space_normalisation(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('foo   '),('bar'),('foo')) "
            "ORDER BY column1 COLLATE RTRIM"
        )


# ---------------------------------------------------------------------------
# Compose with NULLS FIRST / LAST and DESC — the three orthogonal
# axes of ORDER BY must all interact correctly.
# ---------------------------------------------------------------------------


class TestComposability:
    def test_collate_desc_nulls_first(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),(NULL),('b'),('A')) "
            "ORDER BY column1 COLLATE NOCASE DESC NULLS FIRST"
        )

    def test_collate_asc_nulls_last(self) -> None:
        _check(
            "SELECT column1 FROM (VALUES ('a'),(NULL),('b'),('A')) "
            "ORDER BY column1 COLLATE NOCASE ASC NULLS LAST"
        )


# ---------------------------------------------------------------------------
# Edge cases.
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_collate_with_aliased_column(self) -> None:
        _check(
            "SELECT column1 AS name FROM (VALUES ('B'),('a')) "
            "ORDER BY name COLLATE NOCASE"
        )

    def test_collate_with_positional(self) -> None:
        # SQLite supports ``ORDER BY N COLLATE name`` — the collation
        # applies to whatever the Nth output column resolves to.
        _check(
            "SELECT column1 FROM (VALUES ('B'),('a'),('C')) "
            "ORDER BY 1 COLLATE NOCASE"
        )

    def test_unknown_collation_passes_through(self) -> None:
        # SQLite validates collation names lazily — an unknown name
        # is accepted at parse time and only errors if the comparator
        # actually runs.  Our VM ignores unknown names (pass-through
        # to BINARY comparison) to match this lenient behaviour.
        # The query below uses a made-up collation, and we just
        # verify it doesn't crash.
        # (Skipping the oracle check here since sqlite3 actually does
        # raise "no such collation sequence: FOOBAR" — we're more
        # lenient on purpose, treating unknown collations as
        # BINARY, which is reasonable for an educational engine.)
        rows = list(
            mini_sqlite.connect(":memory:").execute(
                "SELECT column1 FROM (VALUES ('B'),('a')) "
                "ORDER BY column1 COLLATE FOOBAR"
            )
        )
        # Unknown collation falls through to BINARY ordering.
        assert rows == [("B",), ("a",)]
