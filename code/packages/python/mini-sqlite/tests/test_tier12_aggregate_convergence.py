"""Tier-12: GROUP BY + HAVING oracle convergence tests.

These tests drive the full mini-sqlite stack — SQL text → parser → planner →
optimizer → codegen → VM — and compare results against the real SQLite3 engine.

Focus area: GROUP BY combined with HAVING where the same aggregate expression
appears in both the SELECT list and the HAVING predicate.  Before the
``_collect_aggregates`` deduplication fix, mini-sqlite returned an extra spurious
column for every such query (e.g. ``('A', 3, 3)`` instead of ``('A', 3)``).

Coverage targets:
  - SELECT cat, SUM(val) … GROUP BY cat HAVING SUM(val) > N
  - HAVING COUNT(*) with COUNT(*) absent from SELECT list
  - Two different aggregates where only one appears in HAVING
  - HAVING with same aggregate used multiple times in complex condition
  - GROUP BY without HAVING (sanity check: no regression)
  - Implicit aggregate (no GROUP BY) with HAVING
  - ORDER BY aggregate that also appears in HAVING
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _con() -> mini_sqlite.Connection:
    """Return a fresh in-memory mini_sqlite connection."""
    return mini_sqlite.connect(":memory:")


def _ref() -> sqlite3.Connection:
    """Return a fresh in-memory real-sqlite3 connection."""
    return sqlite3.connect(":memory:")


def _rows(cur) -> list[tuple]:
    return cur.fetchall()


def _setup(con, sql: str) -> None:
    """Execute one or more semi-colon-separated setup statements."""
    for stmt in sql.strip().split(";"):
        stmt = stmt.strip()
        if stmt:
            con.execute(stmt)


# ---------------------------------------------------------------------------
# Core deduplication regression tests
# ---------------------------------------------------------------------------


class TestHavingSameAggregate:
    """Same aggregate expression in both SELECT list and HAVING clause."""

    def test_sum_in_select_and_having(self) -> None:
        """SELECT cat, SUM(val) … HAVING SUM(val) > N must return 2 columns, not 3.

        This is the canonical regression: before the fix, SUM(val) created
        two separate aggregate slots (_agg_0 for SELECT, _agg_1 for HAVING),
        causing an extra column in the output.
        """
        setup = """
            CREATE TABLE items (cat TEXT, val INTEGER);
            INSERT INTO items VALUES ('A', 1);
            INSERT INTO items VALUES ('A', 2);
            INSERT INTO items VALUES ('B', 10);
            INSERT INTO items VALUES ('C', 0);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT cat, SUM(val) FROM items GROUP BY cat HAVING SUM(val) > 2"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_count_star_in_select_and_having(self) -> None:
        """SELECT dept, COUNT(*) … HAVING COUNT(*) >= 2."""
        setup = """
            CREATE TABLE emp (dept TEXT, name TEXT);
            INSERT INTO emp VALUES ('eng', 'alice');
            INSERT INTO emp VALUES ('eng', 'bob');
            INSERT INTO emp VALUES ('eng', 'carol');
            INSERT INTO emp VALUES ('hr', 'dave');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT dept, COUNT(*) FROM emp GROUP BY dept HAVING COUNT(*) >= 2"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_max_in_select_and_having(self) -> None:
        """SELECT grp, MAX(score) … HAVING MAX(score) < 90."""
        setup = """
            CREATE TABLE scores (grp TEXT, score INTEGER);
            INSERT INTO scores VALUES ('x', 50);
            INSERT INTO scores VALUES ('x', 80);
            INSERT INTO scores VALUES ('y', 95);
            INSERT INTO scores VALUES ('y', 70);
            INSERT INTO scores VALUES ('z', 40);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT grp, MAX(score) FROM scores GROUP BY grp HAVING MAX(score) < 90"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_avg_in_select_and_having(self) -> None:
        """SELECT cat, AVG(val) … HAVING AVG(val) > 5."""
        setup = """
            CREATE TABLE data (cat TEXT, val REAL);
            INSERT INTO data VALUES ('p', 3.0);
            INSERT INTO data VALUES ('p', 9.0);
            INSERT INTO data VALUES ('q', 2.0);
            INSERT INTO data VALUES ('q', 4.0);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT cat, AVG(val) FROM data GROUP BY cat HAVING AVG(val) > 5"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows


# ---------------------------------------------------------------------------
# HAVING with aggregate NOT in SELECT list
# ---------------------------------------------------------------------------


class TestHavingAggNotInSelect:
    """HAVING references an aggregate that does not appear in the SELECT list."""

    def test_having_count_not_in_select(self) -> None:
        """SELECT cat FROM t GROUP BY cat HAVING COUNT(*) > 1 — COUNT only in HAVING."""
        setup = """
            CREATE TABLE t (cat TEXT, val INTEGER);
            INSERT INTO t VALUES ('A', 10);
            INSERT INTO t VALUES ('A', 20);
            INSERT INTO t VALUES ('B', 5);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT cat FROM t GROUP BY cat HAVING COUNT(*) > 1"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_having_sum_not_in_select(self) -> None:
        """SELECT name FROM orders GROUP BY name HAVING SUM(amount) > 100."""
        setup = """
            CREATE TABLE orders (name TEXT, amount INTEGER);
            INSERT INTO orders VALUES ('alice', 60);
            INSERT INTO orders VALUES ('alice', 70);
            INSERT INTO orders VALUES ('bob', 30);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT name FROM orders GROUP BY name HAVING SUM(amount) > 100"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows


# ---------------------------------------------------------------------------
# Two different aggregates
# ---------------------------------------------------------------------------


class TestTwoAggregates:
    """SELECT list has two different aggregates; HAVING references one of them."""

    def test_sum_and_count_having_sum(self) -> None:
        """SELECT dept, SUM(salary), COUNT(*) … HAVING SUM(salary) > 100000."""
        setup = """
            CREATE TABLE staff (dept TEXT, salary INTEGER);
            INSERT INTO staff VALUES ('eng', 90000);
            INSERT INTO staff VALUES ('eng', 80000);
            INSERT INTO staff VALUES ('hr', 50000);
            INSERT INTO staff VALUES ('hr', 45000);
            INSERT INTO staff VALUES ('legal', 200000);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = (
            "SELECT dept, SUM(salary), COUNT(*) FROM staff "
            "GROUP BY dept HAVING SUM(salary) > 100000"
        )
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_sum_and_count_having_count(self) -> None:
        """SELECT dept, SUM(salary), COUNT(*) … HAVING COUNT(*) >= 2."""
        setup = """
            CREATE TABLE staff (dept TEXT, salary INTEGER);
            INSERT INTO staff VALUES ('eng', 90000);
            INSERT INTO staff VALUES ('eng', 80000);
            INSERT INTO staff VALUES ('hr', 50000);
            INSERT INTO staff VALUES ('legal', 200000);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = (
            "SELECT dept, SUM(salary), COUNT(*) FROM staff "
            "GROUP BY dept HAVING COUNT(*) >= 2"
        )
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows


# ---------------------------------------------------------------------------
# Sanity checks — GROUP BY without HAVING
# ---------------------------------------------------------------------------


class TestGroupBySanity:
    """GROUP BY queries without HAVING — verify no regression from the fix."""

    def test_group_by_sum_no_having(self) -> None:
        """Basic GROUP BY + SUM without HAVING still works."""
        setup = """
            CREATE TABLE sales (region TEXT, revenue INTEGER);
            INSERT INTO sales VALUES ('east', 100);
            INSERT INTO sales VALUES ('east', 200);
            INSERT INTO sales VALUES ('west', 300);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT region, SUM(revenue) FROM sales GROUP BY region"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    def test_group_by_count_no_having(self) -> None:
        """GROUP BY + COUNT(*) without HAVING."""
        setup = """
            CREATE TABLE log (level TEXT, msg TEXT);
            INSERT INTO log VALUES ('info', 'a');
            INSERT INTO log VALUES ('info', 'b');
            INSERT INTO log VALUES ('warn', 'c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT level, COUNT(*) FROM log GROUP BY level"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows


# ---------------------------------------------------------------------------
# Edge: empty result from HAVING
# ---------------------------------------------------------------------------


class TestHavingNoRows:
    """HAVING condition that matches no groups — result should be empty."""

    def test_having_sum_no_match(self) -> None:
        """HAVING SUM(val) > 9999 matches nothing."""
        setup = """
            CREATE TABLE t (g TEXT, val INTEGER);
            INSERT INTO t VALUES ('a', 1);
            INSERT INTO t VALUES ('b', 2);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT g, SUM(val) FROM t GROUP BY g HAVING SUM(val) > 9999"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows
        assert mini_rows == []

    def test_having_count_no_match(self) -> None:
        """HAVING COUNT(*) > 100 on a tiny table — empty result."""
        setup = """
            CREATE TABLE t (g TEXT);
            INSERT INTO t VALUES ('x');
            INSERT INTO t VALUES ('y');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT g, COUNT(*) FROM t GROUP BY g HAVING COUNT(*) > 100"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows
        assert mini_rows == []


# ---------------------------------------------------------------------------
# HAVING with ORDER BY on same aggregate
# ---------------------------------------------------------------------------


class TestHavingWithOrderBy:
    """HAVING + ORDER BY that references the same aggregate in the SELECT list."""

    def test_having_and_order_by_sum(self) -> None:
        """SELECT cat, SUM(val) … HAVING SUM(val) > 2 ORDER BY SUM(val) DESC."""
        setup = """
            CREATE TABLE items (cat TEXT, val INTEGER);
            INSERT INTO items VALUES ('A', 1);
            INSERT INTO items VALUES ('A', 4);
            INSERT INTO items VALUES ('B', 10);
            INSERT INTO items VALUES ('C', 0);
            INSERT INTO items VALUES ('D', 5);
            INSERT INTO items VALUES ('D', 3);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = (
            "SELECT cat, SUM(val) FROM items "
            "GROUP BY cat HAVING SUM(val) > 2 ORDER BY SUM(val) DESC"
        )
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows
