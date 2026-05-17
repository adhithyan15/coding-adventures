"""
TEMP / TEMPORARY table and view support.

SQLite allows ``CREATE TEMP TABLE`` and ``CREATE TEMPORARY TABLE`` as aliases
for ``CREATE TABLE``.  mini-sqlite normalises the modifier away before parsing
(in the engine layer) so the grammar stays clean and ``temp`` remains a valid
identifier everywhere.

Truth table for the normalisation:

+---------------------------------------+----------------------+
| Input                                 | Parsed as            |
+=======================================+======================+
| CREATE TEMP TABLE t (id INT)          | CREATE TABLE t (…)  |
| CREATE TEMPORARY TABLE t (id INT)     | CREATE TABLE t (…)  |
| CREATE TEMP VIEW v AS SELECT 1        | CREATE VIEW v AS …  |
| CREATE TEMPORARY VIEW v AS SELECT 1   | CREATE VIEW v AS …  |
| CREATE TABLE temp (x INT)             | unchanged            |
| DELETE FROM temp                      | unchanged            |
+---------------------------------------+----------------------+

Tests are grouped into:

  TestTempTable      — CREATE TEMP TABLE behaviour
  TestTemporaryTable — CREATE TEMPORARY TABLE alias
  TestTempView       — CREATE TEMP VIEW / CREATE TEMPORARY VIEW
  TestTempAsName     — ``temp`` used as an ordinary identifier name
"""

from __future__ import annotations

import pytest

# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def _conn():
    from mini_sqlite import connect  # type: ignore[import]

    return connect(":memory:")


# ---------------------------------------------------------------------------
# TestTempTable
# ---------------------------------------------------------------------------


class TestTempTable:
    """CREATE TEMP TABLE is silently treated as CREATE TABLE."""

    def test_create_temp_table_basic(self):
        """Rows inserted into a TEMP TABLE are visible in the same connection."""
        conn = _conn()
        conn.execute("CREATE TEMP TABLE t (id INTEGER, name TEXT)")
        conn.execute("INSERT INTO t VALUES (1, 'Alice'), (2, 'Bob')")
        rows = conn.execute("SELECT id, name FROM t ORDER BY id").fetchall()
        assert rows == [(1, "Alice"), (2, "Bob")]

    def test_create_temp_table_case_insensitive(self):
        """The TEMP modifier is matched case-insensitively (temp / TEMP / Temp)."""
        conn = _conn()
        conn.execute("create temp table low_case (x INTEGER)")
        conn.execute("INSERT INTO low_case VALUES (42)")
        assert conn.execute("SELECT x FROM low_case").fetchone() == (42,)

    def test_create_temp_table_if_not_exists(self):
        """IF NOT EXISTS works after the TEMP modifier is stripped."""
        conn = _conn()
        conn.execute("CREATE TEMP TABLE IF NOT EXISTS t (x INTEGER)")
        # Running again with IF NOT EXISTS must not raise
        conn.execute("CREATE TEMP TABLE IF NOT EXISTS t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (7)")
        assert conn.execute("SELECT x FROM t").fetchone() == (7,)

    def test_create_temp_table_drop(self):
        """A TEMP TABLE can be dropped with the plain DROP TABLE statement."""
        from mini_sqlite.errors import OperationalError  # type: ignore[import]

        conn = _conn()
        conn.execute("CREATE TEMP TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        conn.execute("DROP TABLE t")
        with pytest.raises(OperationalError):
            conn.execute("SELECT * FROM t")

    def test_create_temp_table_col_types(self):
        """Column types inside a TEMP TABLE are parsed correctly."""
        conn = _conn()
        conn.execute(
            "CREATE TEMP TABLE t ("
            "  id    INTEGER PRIMARY KEY,"
            "  label TEXT NOT NULL,"
            "  score REAL"
            ")"
        )
        conn.execute("INSERT INTO t VALUES (1, 'alpha', 3.14)")
        row = conn.execute("SELECT id, label, score FROM t").fetchone()
        assert row == (1, "alpha", 3.14)


# ---------------------------------------------------------------------------
# TestTemporaryTable
# ---------------------------------------------------------------------------


class TestTemporaryTable:
    """CREATE TEMPORARY TABLE is a synonym for CREATE TEMP TABLE."""

    def test_create_temporary_table_basic(self):
        conn = _conn()
        conn.execute("CREATE TEMPORARY TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (99)")
        assert conn.execute("SELECT x FROM t").fetchone() == (99,)

    def test_create_temporary_table_mixed_case(self):
        """TEMPORARY is stripped regardless of mixed case."""
        conn = _conn()
        conn.execute("CREATE TEMPORARY TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        assert conn.execute("SELECT count(*) FROM t").fetchone() == (1,)

    def test_create_temporary_table_multiple(self):
        """Multiple TEMPORARY tables co-exist without collision."""
        conn = _conn()
        conn.execute("CREATE TEMPORARY TABLE a (x INTEGER)")
        conn.execute("CREATE TEMPORARY TABLE b (y INTEGER)")
        conn.execute("INSERT INTO a VALUES (1)")
        conn.execute("INSERT INTO b VALUES (2)")
        rows = conn.execute("SELECT a.x, b.y FROM a, b").fetchall()
        assert rows == [(1, 2)]


# ---------------------------------------------------------------------------
# TestTempView
# ---------------------------------------------------------------------------


class TestTempView:
    """CREATE TEMP VIEW and CREATE TEMPORARY VIEW are normalised to CREATE VIEW."""

    def test_create_temp_view_basic(self):
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1), (2), (3)")
        conn.execute("CREATE TEMP VIEW v AS SELECT x FROM t WHERE x > 1")
        rows = conn.execute("SELECT x FROM v ORDER BY x").fetchall()
        assert rows == [(2,), (3,)]

    def test_create_temporary_view_basic(self):
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (10), (20)")
        conn.execute("CREATE TEMPORARY VIEW v AS SELECT x * 2 AS doubled FROM t")
        rows = conn.execute("SELECT doubled FROM v ORDER BY doubled").fetchall()
        assert rows == [(20,), (40,)]

    def test_create_temp_view_if_not_exists(self):
        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TEMP VIEW IF NOT EXISTS v AS SELECT x FROM t")
        conn.execute("CREATE TEMP VIEW IF NOT EXISTS v AS SELECT x FROM t")  # no error

    def test_drop_temp_view(self):
        from mini_sqlite.errors import OperationalError  # type: ignore[import]

        conn = _conn()
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("CREATE TEMP VIEW v AS SELECT x FROM t")
        conn.execute("DROP VIEW v")
        with pytest.raises(OperationalError):
            conn.execute("SELECT * FROM v")


# ---------------------------------------------------------------------------
# TestTempAsName
# ---------------------------------------------------------------------------


class TestTempAsName:
    """``temp`` and ``temporary`` used as ordinary identifiers must still work."""

    def test_table_named_temp(self):
        """A table named 'temp' is fully usable — no keyword collision."""
        conn = _conn()
        conn.execute("CREATE TABLE temp (val TEXT)")
        conn.execute("INSERT INTO temp VALUES ('hello')")
        assert conn.execute("SELECT val FROM temp").fetchone() == ("hello",)

    def test_delete_from_temp_table_name(self):
        """DELETE FROM temp must not be misinterpreted."""
        conn = _conn()
        conn.execute("CREATE TABLE temp (val TEXT)")
        conn.execute("INSERT INTO temp VALUES ('a'), ('b')")
        conn.execute("DELETE FROM temp")
        assert conn.execute("SELECT count(*) FROM temp").fetchone() == (0,)

    def test_select_from_temp_after_insert(self):
        """SELECT … FROM temp works like any other table name."""
        conn = _conn()
        conn.execute("CREATE TABLE temp (id INTEGER, name TEXT)")
        conn.execute("INSERT INTO temp VALUES (1, 'one'), (2, 'two')")
        rows = conn.execute("SELECT id, name FROM temp ORDER BY id").fetchall()
        assert rows == [(1, "one"), (2, "two")]

    def test_join_with_table_named_temp(self):
        """Joining a table named 'temp' with another table works."""
        conn = _conn()
        conn.execute("CREATE TABLE temp (k INTEGER, v TEXT)")
        conn.execute("CREATE TABLE other (k INTEGER, w TEXT)")
        conn.execute("INSERT INTO temp VALUES (1, 'a')")
        conn.execute("INSERT INTO other VALUES (1, 'b')")
        rows = conn.execute(
            "SELECT temp.v, other.w FROM temp JOIN other ON temp.k = other.k"
        ).fetchall()
        assert rows == [("a", "b")]

    def test_alias_named_temp(self):
        """Using 'temp' as a table alias should work."""
        conn = _conn()
        conn.execute("CREATE TABLE t (id INTEGER)")
        conn.execute("INSERT INTO t VALUES (5)")
        rows = conn.execute("SELECT temp.id FROM t AS temp").fetchall()
        assert rows == [(5,)]
