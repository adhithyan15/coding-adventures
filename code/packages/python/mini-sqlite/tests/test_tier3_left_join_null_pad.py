"""Oracle tests for ``SELECT *`` NULL-padding on LEFT JOIN unmatched rows.

This is the follow-up to PR #3605 (``SELECT *`` cross-join columns)
where the matched-row case was fixed but unmatched rows still lost
their right-side column slots.  The follow-up wires a cursor-schema
cache into VM state so that ``_do_scan_all_columns`` can emit
``None`` for every column the cursor *would have* produced, instead
of bailing out when the cursor has no current row.

Wire-up:
- ``_VmState.cursor_schema: dict[int, list[str]]`` — new cache.
- ``_do_open`` (OpenScan handler) probes ``backend.columns(table)``
  and caches the visible column names.
- ``_do_advance`` (AdvanceCursor handler) lazily snapshots
  ``row.keys()`` the first time a cursor produces a row, covering
  subquery / working-set / derived-table cursors that don't go
  through OpenScan.
- ``_do_scan_all_columns`` consults the cache when ``current_row``
  is missing for the cursor and emits ``None`` per cached column.

Test coverage:
- LEFT JOIN with unmatched left rows (the headline fix).
- LEFT JOIN with derived tables on the right (lazy-cache path).
- LEFT JOIN where the right side is a CTE.
- LEFT JOIN where the right side has wider schema than the left.
- LEFT JOIN with no matches at all (every left row unmatched).
- Sanity: matched-row LEFT JOIN still works (no regression on PR #3605).

All assertions compare against ``sqlite3`` row-for-row.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _both(setup: list[str], query: str):
    """Apply *setup* on both engines then run *query* and return rows."""
    conn_m = mini_sqlite.connect(":memory:")
    conn_r = sqlite3.connect(":memory:")
    for s in setup:
        conn_m.execute(s)
        conn_r.execute(s)
    m = conn_m.execute(query).fetchall()
    r = conn_r.execute(query).fetchall()
    return m, r


def _check(setup: list[str], query: str) -> None:
    m, r = _both(setup, query)
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Headline fix — LEFT JOIN with some matched and some unmatched left rows
# ---------------------------------------------------------------------------


class TestLeftJoinPartialMatch:
    def test_one_matched_one_unmatched(self) -> None:
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "CREATE TABLE b (id INTEGER, val INTEGER)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
                "INSERT INTO b VALUES (1, 100)",  # no row for a.id=2
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )

    def test_wider_right_schema(self) -> None:
        # Right side has more columns than left — NULL-padding must produce
        # the right number of Nones, not just one.
        _check(
            [
                "CREATE TABLE a (id INTEGER)",
                "CREATE TABLE b (id INTEGER, c1 INTEGER, c2 INTEGER, c3 INTEGER)",
                "INSERT INTO a VALUES (1), (2), (3)",
                "INSERT INTO b VALUES (1, 10, 20, 30)",  # only a.id=1 matches
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )

    def test_all_left_unmatched(self) -> None:
        # b is empty — every a row gets NULLs.
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "CREATE TABLE b (id INTEGER, val INTEGER)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y'), (3, 'z')",
                # b is intentionally empty
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )

    def test_all_left_matched(self) -> None:
        """Regression guard — matched-row LEFT JOIN must still work."""
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "CREATE TABLE b (id INTEGER, val INTEGER)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
                "INSERT INTO b VALUES (1, 100), (2, 200)",
            ],
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )


# ---------------------------------------------------------------------------
# Lazy-cache path — right side is a derived table / CTE
# ---------------------------------------------------------------------------


class TestLeftJoinDerivedRight:
    def test_derived_table_right_partial_match(self) -> None:
        # The derived table's cursor schema is populated lazily by the
        # first matched row.  After that, unmatched left rows null-pad
        # against the cached schema.
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            ],
            "SELECT * FROM a LEFT JOIN "
            "(SELECT 1 AS id, 100 AS val) sub ON a.id = sub.id "
            "ORDER BY a.id",
        )

    def test_cte_right_partial_match(self) -> None:
        _check(
            [
                "CREATE TABLE a (id INTEGER, name TEXT)",
                "INSERT INTO a VALUES (1, 'x'), (2, 'y')",
            ],
            "WITH b(id, val) AS (SELECT 1, 100) "
            "SELECT * FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id",
        )


# ---------------------------------------------------------------------------
# Cross-join + LEFT JOIN — both fixes compose correctly
# ---------------------------------------------------------------------------


class TestCrossJoinComposesWithLeftJoin:
    def test_cross_join_then_left_join(self) -> None:
        # FROM a, b LEFT JOIN c ON ... — both the cross-join SELECT * fix
        # (PR #3605) and the LEFT JOIN NULL-pad fix (this PR) must compose.
        _check(
            [
                "CREATE TABLE a (av INTEGER)",
                "CREATE TABLE b (bv INTEGER)",
                "CREATE TABLE c (av INTEGER, cv INTEGER)",
                "INSERT INTO a VALUES (1)",
                "INSERT INTO b VALUES (10)",
                "INSERT INTO c VALUES (1, 100)",  # matches a.av=1
            ],
            "SELECT * FROM a, b LEFT JOIN c ON a.av = c.av",
        )
