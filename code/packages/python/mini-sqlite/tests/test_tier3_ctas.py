"""
CREATE TABLE … AS SELECT … (CTAS).

SQLite extends standard CREATE TABLE with a ``AS SELECT`` form that copies
both structure *and* data from a query into a new table in one step:

  CREATE TABLE dst AS SELECT * FROM src
  CREATE TABLE dst AS SELECT x AS a, y AS b FROM src WHERE x > 0
  CREATE TABLE IF NOT EXISTS dst AS SELECT * FROM src

Rules:
* The destination table is created even when the source returns zero rows.
* ``IF NOT EXISTS`` makes the whole statement a no-op when *dst* already
  exists (rows are NOT inserted into the existing table).
* Column names come from the SELECT output aliases or, for bare column
  references, the column names.  Expression columns get a default name
  chosen by the engine.
* Column types in the destination are always BLOB affinity (SQLite's
  "weakest" type) because the grammar does not emit type information for
  derived columns.
* CTAS is blocked under ``PRAGMA query_only = 1`` (it's a DDL write).

Known differences from SQLite (documented limitations):
* SQLite names unnamed expression columns after the expression text
  (e.g. ``x * 2`` → column name ``x * 2``).  Mini-sqlite uses a
  positional fallback (``col_0``, ``col_1``, …) because the VM
  uses ``'?'`` as a sentinel for unnamed computed columns.
* Column ``type`` in ``PRAGMA table_info`` is always ``BLOB`` in
  mini-sqlite regardless of the source column's declared type.

Tests that depend on the SQLite column-naming behaviour use shape-only
oracles (row count, data values) rather than column-name oracles.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite
from mini_sqlite.errors import OperationalError


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _mini(ddl: list[str] | str = ()) -> mini_sqlite.Connection:
    """Return an in-memory mini-sqlite connection pre-loaded with *ddl*."""
    conn = mini_sqlite.connect(":memory:")
    if isinstance(ddl, str):
        ddl = [ddl]
    for stmt in ddl:
        conn.execute(stmt)
    return conn


def _real(ddl: list[str] | str = ()) -> sqlite3.Connection:
    """Return an in-memory sqlite3 connection pre-loaded with *ddl*."""
    conn = sqlite3.connect(":memory:")
    if isinstance(ddl, str):
        ddl = [ddl]
    for stmt in ddl:
        conn.execute(stmt)
    return conn


# ---------------------------------------------------------------------------
# Basic CTAS — non-empty source table
# ---------------------------------------------------------------------------


def test_ctas_star_copies_all_rows():
    """SELECT * CTAS copies every row from the source."""
    ddl = [
        "CREATE TABLE src (x INTEGER, y TEXT)",
        "INSERT INTO src VALUES (1, 'alpha')",
        "INSERT INTO src VALUES (2, 'beta')",
        "INSERT INTO src VALUES (3, 'gamma')",
    ]
    m = _mini(ddl)
    r = _real(ddl)

    m.execute("CREATE TABLE dst AS SELECT * FROM src")
    r.execute("CREATE TABLE dst AS SELECT * FROM src")

    m_rows = set(m.execute("SELECT * FROM dst").fetchall())
    r_rows = set(r.execute("SELECT * FROM dst").fetchall())
    assert m_rows == r_rows


def test_ctas_star_produces_correct_column_names():
    """Column names in the destination match source column names."""
    ddl = [
        "CREATE TABLE src (id INTEGER, name TEXT, value REAL)",
        "INSERT INTO src VALUES (1, 'a', 1.5)",
    ]
    m = _mini(ddl)
    m.execute("CREATE TABLE dst AS SELECT * FROM src")
    row = m.execute("SELECT id, name, value FROM dst").fetchone()
    assert row == (1, "a", 1.5)


def test_ctas_explicit_alias_columns():
    """Explicit AS aliases become destination column names."""
    ddl = [
        "CREATE TABLE src (x INTEGER, y INTEGER)",
        "INSERT INTO src VALUES (10, 20)",
    ]
    m = _mini(ddl)
    r = _real(ddl)

    m.execute("CREATE TABLE dst AS SELECT x AS a, y AS b FROM src")
    r.execute("CREATE TABLE dst AS SELECT x AS a, y AS b FROM src")

    m_row = m.execute("SELECT a, b FROM dst").fetchone()
    r_row = r.execute("SELECT a, b FROM dst").fetchone()
    assert m_row == r_row == (10, 20)


def test_ctas_with_where_clause():
    """CTAS respects a WHERE filter on the source."""
    ddl = [
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (2)",
        "INSERT INTO src VALUES (3)",
        "INSERT INTO src VALUES (4)",
    ]
    m = _mini(ddl)
    r = _real(ddl)

    m.execute("CREATE TABLE evens AS SELECT n FROM src WHERE n % 2 = 0")
    r.execute("CREATE TABLE evens AS SELECT n FROM src WHERE n % 2 = 0")

    m_rows = sorted(row[0] for row in m.execute("SELECT * FROM evens").fetchall())
    r_rows = sorted(row[0] for row in r.execute("SELECT * FROM evens").fetchall())
    assert m_rows == r_rows == [2, 4]


def test_ctas_rows_affected_equals_row_count():
    """rows_affected equals the number of rows in the source SELECT."""
    ddl = [
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (2)",
        "INSERT INTO src VALUES (3)",
    ]
    m = _mini(ddl)
    result = m.execute("CREATE TABLE dst AS SELECT * FROM src")
    assert result.rowcount == 3


# ---------------------------------------------------------------------------
# CTAS from empty source table
# ---------------------------------------------------------------------------


def test_ctas_from_empty_table_creates_table():
    """CTAS from an empty source creates the destination table (no rows)."""
    m = _mini([
        "CREATE TABLE src (x INTEGER, y TEXT)",
    ])
    m.execute("CREATE TABLE dst AS SELECT * FROM src")

    # Table must exist and be queryable.
    rows = m.execute("SELECT * FROM dst").fetchall()
    assert rows == []


def test_ctas_from_empty_table_correct_columns():
    """Destination column names match source even when source is empty."""
    m = _mini([
        "CREATE TABLE src (alpha INTEGER, beta TEXT, gamma REAL)",
    ])
    m.execute("CREATE TABLE dst AS SELECT * FROM src")

    # Verify columns via PRAGMA table_info.
    info = m.execute("PRAGMA table_info(dst)").fetchall()
    names = [row[1] for row in info]
    assert names == ["alpha", "beta", "gamma"]


def test_ctas_from_empty_table_explicit_columns():
    """Explicit column selection from empty source produces correct names."""
    m = _mini([
        "CREATE TABLE src (a INTEGER, b TEXT, c REAL)",
    ])
    m.execute("CREATE TABLE dst AS SELECT a, b FROM src")

    info = m.execute("PRAGMA table_info(dst)").fetchall()
    names = [row[1] for row in info]
    assert names == ["a", "b"]


def test_ctas_from_empty_table_explicit_aliases():
    """Explicit aliases from empty source appear as destination column names."""
    m = _mini([
        "CREATE TABLE src (x INTEGER, y TEXT)",
    ])
    m.execute("CREATE TABLE dst AS SELECT x AS num, y AS label FROM src")

    info = m.execute("PRAGMA table_info(dst)").fetchall()
    names = [row[1] for row in info]
    assert names == ["num", "label"]


def test_ctas_empty_then_insert():
    """Table created via empty-source CTAS accepts subsequent inserts."""
    m = _mini([
        "CREATE TABLE src (x INTEGER, y TEXT)",
    ])
    m.execute("CREATE TABLE dst AS SELECT * FROM src")
    m.execute("INSERT INTO dst VALUES (99, 'hello')")
    row = m.execute("SELECT * FROM dst").fetchone()
    assert row == (99, "hello")


# ---------------------------------------------------------------------------
# IF NOT EXISTS
# ---------------------------------------------------------------------------


def test_ctas_if_not_exists_noop_when_table_exists():
    """IF NOT EXISTS is a no-op when the destination already exists."""
    ddl = [
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (1)",
        "CREATE TABLE dst (n INTEGER)",
        "INSERT INTO dst VALUES (42)",
    ]
    m = _mini(ddl)
    r = _real(ddl)

    # Should not raise; dst is not modified.
    m.execute("CREATE TABLE IF NOT EXISTS dst AS SELECT * FROM src")
    r.execute("CREATE TABLE IF NOT EXISTS dst AS SELECT * FROM src")

    m_rows = m.execute("SELECT * FROM dst").fetchall()
    r_rows = r.execute("SELECT * FROM dst").fetchall()
    assert m_rows == r_rows == [(42,)]


def test_ctas_without_if_not_exists_raises_on_duplicate():
    """Without IF NOT EXISTS, CTAS on an existing table raises."""
    m = _mini([
        "CREATE TABLE src (n INTEGER)",
        "CREATE TABLE dst (n INTEGER)",
    ])
    try:
        m.execute("CREATE TABLE dst AS SELECT * FROM src")
        assert False, "expected OperationalError"
    except OperationalError:
        pass


def test_ctas_if_not_exists_from_empty_source_noop():
    """IF NOT EXISTS no-op works even when source is empty."""
    m = _mini([
        "CREATE TABLE src (x INTEGER)",
        "CREATE TABLE dst (x INTEGER)",
        "INSERT INTO dst VALUES (7)",
    ])
    result = m.execute("CREATE TABLE IF NOT EXISTS dst AS SELECT * FROM src")
    # dst unchanged
    rows = m.execute("SELECT * FROM dst").fetchall()
    assert rows == [(7,)]


# ---------------------------------------------------------------------------
# CTAS creates independent table (not a view)
# ---------------------------------------------------------------------------


def test_ctas_dst_is_independent_of_src():
    """Modifying src after CTAS does not affect dst."""
    ddl = [
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (2)",
    ]
    m = _mini(ddl)
    r = _real(ddl)

    m.execute("CREATE TABLE dst AS SELECT * FROM src")
    r.execute("CREATE TABLE dst AS SELECT * FROM src")

    m.execute("DELETE FROM src")
    r.execute("DELETE FROM src")

    m_rows = m.execute("SELECT * FROM dst").fetchall()
    r_rows = r.execute("SELECT * FROM dst").fetchall()
    assert set(m_rows) == set(r_rows)
    assert len(m_rows) == 2  # dst still has original rows


def test_ctas_modifying_dst_does_not_affect_src():
    """Modifying dst does not affect src."""
    m = _mini([
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (10)",
    ])
    m.execute("CREATE TABLE dst AS SELECT * FROM src")
    m.execute("UPDATE dst SET n = 999")

    src_row = m.execute("SELECT * FROM src").fetchone()
    assert src_row == (10,)


# ---------------------------------------------------------------------------
# CTAS with aggregation
# ---------------------------------------------------------------------------


def test_ctas_with_aggregation():
    """CTAS with COUNT(*) AS alias works for non-empty source."""
    m = _mini([
        "CREATE TABLE src (g TEXT, v INTEGER)",
        "INSERT INTO src VALUES ('a', 1)",
        "INSERT INTO src VALUES ('a', 2)",
        "INSERT INTO src VALUES ('b', 3)",
    ])
    r = _real([
        "CREATE TABLE src (g TEXT, v INTEGER)",
        "INSERT INTO src VALUES ('a', 1)",
        "INSERT INTO src VALUES ('a', 2)",
        "INSERT INTO src VALUES ('b', 3)",
    ])

    m.execute("CREATE TABLE counts AS SELECT g, COUNT(*) AS cnt FROM src GROUP BY g")
    r.execute("CREATE TABLE counts AS SELECT g, COUNT(*) AS cnt FROM src GROUP BY g")

    m_rows = set(m.execute("SELECT g, cnt FROM counts").fetchall())
    r_rows = set(r.execute("SELECT g, cnt FROM counts").fetchall())
    assert m_rows == r_rows == {("a", 2), ("b", 1)}


# ---------------------------------------------------------------------------
# PRAGMA query_only enforcement
# ---------------------------------------------------------------------------


def test_ctas_blocked_by_query_only():
    """CTAS raises OperationalError when query_only = 1."""
    m = _mini([
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (1)",
    ])
    m.execute("PRAGMA query_only = 1")
    try:
        m.execute("CREATE TABLE dst AS SELECT * FROM src")
        assert False, "expected OperationalError"
    except OperationalError as e:
        assert "readonly" in str(e).lower()


# ---------------------------------------------------------------------------
# CTAS with TEMP / TEMPORARY keyword
# ---------------------------------------------------------------------------


def test_ctas_create_temp_table():
    """CREATE TEMP TABLE ... AS SELECT ... is accepted and normalised."""
    m = _mini([
        "CREATE TABLE src (x INTEGER)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (2)",
    ])
    m.execute("CREATE TEMP TABLE dst AS SELECT * FROM src")
    rows = m.execute("SELECT * FROM dst").fetchall()
    assert set(rows) == {(1,), (2,)}


# ---------------------------------------------------------------------------
# CTAS with CTE source
# ---------------------------------------------------------------------------


def test_ctas_from_cte():
    """CTAS whose source is a WITH … SELECT copies the CTE output."""
    m = _mini()
    m.execute(
        "CREATE TABLE squares AS "
        "WITH nums(n) AS (VALUES (1),(2),(3),(4),(5)) "
        "SELECT n * n AS sq FROM nums"
    )
    rows = sorted(row[0] for row in m.execute("SELECT sq FROM squares").fetchall())
    assert rows == [1, 4, 9, 16, 25]


# ---------------------------------------------------------------------------
# CTAS with ORDER BY / LIMIT
# ---------------------------------------------------------------------------


def test_ctas_with_order_by_and_limit():
    """CTAS respects ORDER BY … LIMIT on the source SELECT."""
    m = _mini([
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (5)",
        "INSERT INTO src VALUES (3)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (4)",
        "INSERT INTO src VALUES (2)",
    ])
    r = _real([
        "CREATE TABLE src (n INTEGER)",
        "INSERT INTO src VALUES (5)",
        "INSERT INTO src VALUES (3)",
        "INSERT INTO src VALUES (1)",
        "INSERT INTO src VALUES (4)",
        "INSERT INTO src VALUES (2)",
    ])
    m.execute("CREATE TABLE top3 AS SELECT n FROM src ORDER BY n DESC LIMIT 3")
    r.execute("CREATE TABLE top3 AS SELECT n FROM src ORDER BY n DESC LIMIT 3")

    m_rows = sorted(row[0] for row in m.execute("SELECT * FROM top3").fetchall())
    r_rows = sorted(row[0] for row in r.execute("SELECT * FROM top3").fetchall())
    assert m_rows == r_rows == [3, 4, 5]
