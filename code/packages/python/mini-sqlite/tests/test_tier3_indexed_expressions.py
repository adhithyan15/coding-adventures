"""
Indexed expressions: ``CREATE INDEX … ON t(expr [COLLATE …] [ASC|DESC])``.

SQLite 3.9+ allows index columns to be arbitrary expressions, not just bare
column names.  Common cases:

  CREATE INDEX idx_lower ON t(LOWER(name))
  CREATE INDEX idx_collate ON t(name COLLATE NOCASE)
  CREATE INDEX idx_compound ON t(LOWER(name), id)

ORMs and migration tools (SQLAlchemy, Alembic, Django) routinely emit
indexed expressions for case-insensitive search.  Previously mini-sqlite
parse-errored on the function-call form in an index column list.

This release adds parse-and-accept semantics:

* **Bare column index** (`CREATE INDEX i ON t(c)`): creates a real index,
  speeds up bare-column lookups.
* **Expression index** (`CREATE INDEX i ON t(LOWER(c))`): silently no-ops
  the index creation — the SQL parses, the DDL succeeds, but no actual
  index is stored.  Lookups that match the expression fall back to a
  table scan (correct results, no perf benefit).
* **COLLATE clause**: discarded; only the BINARY collation is implemented.
* **ASC / DESC**: discarded (B-tree indexes are bidirectional).

Oracle-compared against the real ``sqlite3`` module for each pattern.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check_create_index(sql: str) -> None:
    """Verify both engines accept the CREATE INDEX (no parse / runtime error)."""
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")
    mini.execute(sql)
    ref.execute(sql)


# ---------------------------------------------------------------------------
# Bare-column indexes (must still work)
# ---------------------------------------------------------------------------


def test_bare_column_index():
    _check_create_index("CREATE INDEX idx ON t(name)")


def test_bare_column_index_desc():
    _check_create_index("CREATE INDEX idx ON t(name DESC)")


def test_compound_bare_columns():
    _check_create_index("CREATE INDEX idx ON t(name, age)")


def test_compound_bare_columns_with_directions():
    _check_create_index("CREATE INDEX idx ON t(name ASC, age DESC)")


# ---------------------------------------------------------------------------
# Function-call indexed expressions
# ---------------------------------------------------------------------------


def test_lower_function_index():
    _check_create_index("CREATE INDEX idx ON t(LOWER(name))")


def test_upper_function_index():
    _check_create_index("CREATE INDEX idx ON t(UPPER(name))")


def test_substr_function_index():
    _check_create_index("CREATE INDEX idx ON t(SUBSTR(name, 1, 3))")


def test_function_index_with_direction():
    _check_create_index("CREATE INDEX idx ON t(LOWER(name) ASC)")


# ---------------------------------------------------------------------------
# COLLATE clause
# ---------------------------------------------------------------------------


def test_collate_nocase():
    _check_create_index("CREATE INDEX idx ON t(name COLLATE NOCASE)")


def test_collate_binary():
    _check_create_index("CREATE INDEX idx ON t(name COLLATE BINARY)")


def test_collate_rtrim():
    _check_create_index("CREATE INDEX idx ON t(name COLLATE RTRIM)")


def test_collate_with_direction():
    _check_create_index("CREATE INDEX idx ON t(name COLLATE NOCASE DESC)")


# ---------------------------------------------------------------------------
# Mixed: function + bare column
# ---------------------------------------------------------------------------


def test_mixed_function_and_bare():
    _check_create_index("CREATE INDEX idx ON t(LOWER(name), id)")


def test_mixed_function_and_collate():
    _check_create_index("CREATE INDEX idx ON t(LOWER(name), name COLLATE NOCASE)")


# ---------------------------------------------------------------------------
# IF NOT EXISTS and UNIQUE with indexed expressions
# ---------------------------------------------------------------------------


def test_unique_indexed_expression():
    _check_create_index("CREATE UNIQUE INDEX idx ON t(LOWER(name))")


def test_if_not_exists_indexed_expression():
    mini = mini_sqlite.connect(":memory:")
    ref = sqlite3.connect(":memory:")
    for c in (mini, ref):
        c.execute("CREATE TABLE t (id INTEGER, name TEXT)")
        c.execute("CREATE INDEX idx ON t(LOWER(name))")
        # Second creation with IF NOT EXISTS must not raise.
        c.execute("CREATE INDEX IF NOT EXISTS idx ON t(LOWER(name))")


# ---------------------------------------------------------------------------
# Bare-column index is still indexed (PRAGMA index_list confirms)
# ---------------------------------------------------------------------------


def test_bare_column_index_registers():
    """A real index is created for bare-column form (visible in PRAGMA index_list)."""
    mini = mini_sqlite.connect(":memory:")
    mini.execute("CREATE TABLE t (id INTEGER, name TEXT)")
    mini.execute("CREATE INDEX my_idx ON t(name)")
    rows = mini.execute("PRAGMA index_list('t')").fetchall()
    names = [r[1] for r in rows]
    assert "my_idx" in names
