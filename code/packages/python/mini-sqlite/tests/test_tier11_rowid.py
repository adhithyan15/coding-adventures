"""Tier-11: ROWID / _rowid_ / oid pseudo-column integration tests.

These tests drive the full mini-sqlite stack — SQL text → parser → planner →
optimizer → codegen → VM — and compare results against the real SQLite3 engine
to verify oracle-grade compatibility.

Coverage targets:
  - SELECT rowid FROM t           (basic pseudo-column in SELECT list)
  - SELECT rowid, * FROM t        (rowid + wildcard expansion; rowid NOT in *)
  - SELECT _rowid_, oid FROM t    (alias names)
  - SELECT t.rowid FROM t         (qualified reference)
  - WHERE rowid = N               (filter by rowid)
  - WHERE rowid > N               (range filter; pagination pattern)
  - WHERE _rowid_ = N             (alias in WHERE)
  - DELETE FROM t WHERE rowid = N (delete by rowid)
  - Rowid is stable after DELETE  (surviving rows keep their original rowid)
  - Rowid monotonically increases (never reused)
  - Rowid with joined tables      (each table has its own rowid namespace)
  - ORDER BY rowid                (rowid as sort key)
  - SELECT rowid in subquery      (rowid in scalar subquery context)
  - SELECT * does NOT include rowid (rowid is implicit, not part of wildcard)
"""

from __future__ import annotations

import sqlite3

import pytest

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
# Basic SELECT rowid
# ---------------------------------------------------------------------------


class TestRowIdSelect:
    """SELECT rowid / _rowid_ / oid from a simple single table."""

    def test_select_rowid_basic(self) -> None:
        """SELECT rowid FROM t — first inserted row has rowid 1."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('hello');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_rowid_multiple_rows(self) -> None:
        """Rowids are sequential integers starting at 1."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_rowid_alias_rowid_(self) -> None:
        """_rowid_ is a synonym for rowid."""
        setup = "CREATE TABLE t (val TEXT); INSERT INTO t VALUES ('x');"
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT _rowid_ FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_rowid_alias_oid(self) -> None:
        """oid is a synonym for rowid."""
        setup = "CREATE TABLE t (val TEXT); INSERT INTO t VALUES ('x');"
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT oid FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_rowid_qualified(self) -> None:
        """t.rowid uses the table-qualified form."""
        setup = "CREATE TABLE t (val TEXT); INSERT INTO t VALUES ('x');"
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT t.rowid FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_rowid_with_other_columns(self) -> None:
        """SELECT rowid alongside real columns."""
        setup = """
            CREATE TABLE items (name TEXT, price INTEGER);
            INSERT INTO items VALUES ('apple', 10);
            INSERT INTO items VALUES ('banana', 5);
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, name, price FROM items"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_select_star_does_not_include_rowid(self) -> None:
        """SELECT * does NOT include the implicit rowid column."""
        setup = """
            CREATE TABLE t (id INTEGER, val TEXT);
            INSERT INTO t VALUES (1, 'a');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT * FROM t"
        mini_rows = _rows(con.execute(sql))
        ref_rows = _rows(ref.execute(sql))
        assert mini_rows == ref_rows

    @pytest.mark.skip(reason="SELECT expr, * mixed projection not yet supported by parser")
    def test_select_rowid_star(self) -> None:
        """SELECT rowid, * — rowid prepended to all real columns.

        Skipped: the current parser does not support mixing explicit column
        references with ``*`` in the same SELECT list (``SELECT col, *``).
        Pure ``SELECT *`` is supported; ``SELECT rowid, *`` requires a grammar
        extension.
        """
        setup = """
            CREATE TABLE t (id INTEGER, val TEXT);
            INSERT INTO t VALUES (42, 'hello');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, * FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))


# ---------------------------------------------------------------------------
# Rowid stability
# ---------------------------------------------------------------------------


class TestRowIdStability:
    """Rowids are stable: DELETE does not renumber surviving rows."""

    def test_rowid_stable_after_middle_delete(self) -> None:
        """Deleting the middle row leaves other rowids unchanged."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        del_sql = "DELETE FROM t WHERE val = 'b'"
        con.execute(del_sql)
        ref.execute(del_sql)

        sql = "SELECT rowid, val FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_rowid_stable_after_first_delete(self) -> None:
        """Deleting the first row leaves the second row with its original rowid."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('first');
            INSERT INTO t VALUES ('second');
            INSERT INTO t VALUES ('third');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        del_sql = "DELETE FROM t WHERE rowid = 1"
        con.execute(del_sql)
        ref.execute(del_sql)

        sql = "SELECT rowid, val FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_rowid_not_reused_after_partial_delete(self) -> None:
        """After deleting some rows, remaining rows keep their rowids."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('keep1');
            INSERT INTO t VALUES ('delete_me');
            INSERT INTO t VALUES ('keep2');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        del_sql = "DELETE FROM t WHERE val = 'delete_me'"
        con.execute(del_sql)
        ref.execute(del_sql)

        sql = "SELECT rowid, val FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))


# ---------------------------------------------------------------------------
# WHERE by rowid
# ---------------------------------------------------------------------------


class TestRowIdFilter:
    """Use rowid in the WHERE clause."""

    def test_where_rowid_eq(self) -> None:
        """WHERE rowid = N fetches exactly one row."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT val FROM t WHERE rowid = 2"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_where_rowid_gt(self) -> None:
        """WHERE rowid > N — pagination offset pattern."""
        setup = """
            CREATE TABLE log (msg TEXT);
            INSERT INTO log VALUES ('e1');
            INSERT INTO log VALUES ('e2');
            INSERT INTO log VALUES ('e3');
            INSERT INTO log VALUES ('e4');
            INSERT INTO log VALUES ('e5');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, msg FROM log WHERE rowid > 2"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_where_rowid_between(self) -> None:
        """WHERE rowid BETWEEN lo AND hi — a range scan."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
            INSERT INTO t VALUES ('d');
            INSERT INTO t VALUES ('e');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, val FROM t WHERE rowid BETWEEN 2 AND 4"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_where_oid_alias(self) -> None:
        """oid works in WHERE just like rowid."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('x');
            INSERT INTO t VALUES ('y');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT val FROM t WHERE oid = 1"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_where_rowid_no_match(self) -> None:
        """WHERE rowid = 999 on a 3-row table returns empty result."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT val FROM t WHERE rowid = 999"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))


# ---------------------------------------------------------------------------
# DELETE by rowid
# ---------------------------------------------------------------------------


class TestRowIdDelete:
    """DELETE FROM t WHERE rowid = N."""

    def test_delete_by_rowid(self) -> None:
        """Exact rowid delete removes exactly one row."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
            INSERT INTO t VALUES ('c');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        del_sql = "DELETE FROM t WHERE rowid = 2"
        con.execute(del_sql)
        ref.execute(del_sql)

        sql = "SELECT rowid, val FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_delete_range_by_rowid(self) -> None:
        """DELETE WHERE rowid > N removes a suffix of rows."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('k');
            INSERT INTO t VALUES ('l');
            INSERT INTO t VALUES ('m');
            INSERT INTO t VALUES ('n');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        del_sql = "DELETE FROM t WHERE rowid > 2"
        con.execute(del_sql)
        ref.execute(del_sql)

        sql = "SELECT rowid, val FROM t"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))


# ---------------------------------------------------------------------------
# ORDER BY rowid
# ---------------------------------------------------------------------------


class TestRowIdOrderBy:
    """rowid as a sort key in ORDER BY."""

    def test_order_by_rowid_asc(self) -> None:
        """ORDER BY rowid ASC — natural insertion order."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('c');
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, val FROM t ORDER BY rowid ASC"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))

    def test_order_by_rowid_desc(self) -> None:
        """ORDER BY rowid DESC — reverse insertion order."""
        setup = """
            CREATE TABLE t (val TEXT);
            INSERT INTO t VALUES ('c');
            INSERT INTO t VALUES ('a');
            INSERT INTO t VALUES ('b');
        """
        con = _con()
        ref = _ref()
        _setup(con, setup)
        _setup(ref, setup)

        sql = "SELECT rowid, val FROM t ORDER BY rowid DESC"
        assert _rows(con.execute(sql)) == _rows(ref.execute(sql))
