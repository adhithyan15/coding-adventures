"""ORDER BY / LIMIT trailing a UNION / INTERSECT / EXCEPT compound.

SQLite (and the SQL standard) treats trailing ``ORDER BY`` and
``LIMIT`` at the end of a compound query as applying to the *whole*
compound, not just the rightmost SELECT::

    SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x DESC LIMIT 1
    -- ⇒ (2,)   — the whole {1, 2} set is sorted DESC, then limited

The grammar parses these clauses onto the last ``select_stmt`` leg
(because that's where they syntactically appear), but the adapter
hoists them up to a wrapper SELECT::

    SELECT * FROM (compound) ORDER BY x DESC LIMIT 1

…which makes the compound's output columns (inherited from the
*leftmost* SELECT, matching SQLite) visible to the ORDER BY clause.

These oracle tests pin the behaviour byte-for-byte against stdlib
sqlite3.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# UNION ALL — duplicates preserved; ORDER BY operates on the union.
# ---------------------------------------------------------------------------


class TestUnionAllCompound:
    def test_order_by_column_name(self) -> None:
        _check("SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x")

    def test_order_by_desc(self) -> None:
        _check("SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x DESC")

    def test_order_by_with_limit(self) -> None:
        _check("SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x LIMIT 1")

    def test_order_by_with_limit_offset(self) -> None:
        _check("SELECT 1 AS x UNION ALL SELECT 2 ORDER BY x LIMIT 1 OFFSET 1")

    def test_limit_only_no_order(self) -> None:
        _check("SELECT 1 AS x UNION ALL SELECT 2 LIMIT 1")

    def test_order_by_position(self) -> None:
        # ORDER BY 1 references the first output column of the compound.
        _check("SELECT 3 UNION ALL SELECT 1 UNION ALL SELECT 2 ORDER BY 1")

    def test_unsorted_input_sorted_by_compound_order_by(self) -> None:
        _check(
            "SELECT 3 AS x UNION ALL SELECT 1 UNION ALL SELECT 2 ORDER BY x"
        )


# ---------------------------------------------------------------------------
# UNION (dedup) — duplicates removed; the trailing ORDER BY still applies.
# ---------------------------------------------------------------------------


class TestUnionDedupCompound:
    def test_dedup_then_order(self) -> None:
        _check("SELECT 3 UNION SELECT 1 UNION SELECT 2 ORDER BY 1")

    def test_dedup_then_order_desc(self) -> None:
        _check("SELECT 1 UNION SELECT 2 UNION SELECT 1 ORDER BY 1 DESC")

    def test_dedup_with_limit(self) -> None:
        _check("SELECT 1 UNION SELECT 2 UNION SELECT 3 ORDER BY 1 LIMIT 2")


# ---------------------------------------------------------------------------
# INTERSECT / EXCEPT — same hoisting machinery applies.
# ---------------------------------------------------------------------------


class TestIntersectExceptCompound:
    def test_intersect_order_by(self) -> None:
        _check("SELECT 5 INTERSECT SELECT 5 ORDER BY 1")

    def test_except_order_by(self) -> None:
        _check("SELECT 1 EXCEPT SELECT 2 ORDER BY 1")

    def test_intersect_with_limit(self) -> None:
        # The intersect produces {1, 2}; LIMIT 1 keeps just one row.
        _check(
            "SELECT 1 UNION SELECT 2 UNION SELECT 3 "
            "INTERSECT SELECT 2 UNION SELECT 1 "
            "ORDER BY 1 LIMIT 1"
        )

    def test_chain_with_compound_order_limit(self) -> None:
        _check(
            "SELECT 3 UNION SELECT 1 UNION SELECT 4 UNION SELECT 1 UNION SELECT 5 "
            "ORDER BY 1 LIMIT 3"
        )


# ---------------------------------------------------------------------------
# VALUES on either side, with trailing ORDER BY/LIMIT.  This exercises
# the interaction between PR #3968's VALUES support and this PR's
# compound-tail hoisting.
# ---------------------------------------------------------------------------


class TestCompoundWithValues:
    def test_values_union_select_then_order(self) -> None:
        _check("VALUES (3),(1) UNION SELECT 2 ORDER BY 1")

    def test_values_union_values_then_order(self) -> None:
        # VALUES on the LEFT means the trailing ORDER BY hangs off a
        # SELECT on the right — the path PR #3968 supports.  (SQLite
        # itself rejects ``SELECT … UNION VALUES … ORDER BY …`` as a
        # syntax error because the trailing ORDER BY can't bind to a
        # VALUES leg, so we don't test that asymmetric form here.)
        _check("VALUES (3),(1) UNION SELECT 2 ORDER BY 1 DESC")


# ---------------------------------------------------------------------------
# Regression guards — make sure a single SELECT still gets its own
# ORDER BY/LIMIT (no hoisting when there are no set-ops).
# ---------------------------------------------------------------------------


class TestSingleSelectStillWorks:
    def test_single_select_order_by(self) -> None:
        _check("SELECT * FROM (VALUES (3),(1),(2)) ORDER BY column1")

    def test_single_select_limit(self) -> None:
        _check("SELECT * FROM (VALUES (3),(1),(2)) ORDER BY column1 LIMIT 2")
