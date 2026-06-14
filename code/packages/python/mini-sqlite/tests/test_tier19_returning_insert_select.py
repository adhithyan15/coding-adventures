"""Tier 19 — RETURNING clause on INSERT … SELECT statements.

SQLite supports RETURNING on all DML forms including INSERT … SELECT, where
the source of new rows is a sub-query rather than a literal VALUES list.  Each
row that is successfully inserted emits one row in the RETURNING result set.

    INSERT INTO dst SELECT id, name FROM src RETURNING id, name
    INSERT INTO log SELECT event, ts FROM events WHERE ts > ? RETURNING rowid, event

Semantics (verified against real sqlite3):
  - One RETURNING row is emitted per successfully inserted row, in insertion order.
  - Rows skipped by ON CONFLICT IGNORE (or filtered by WHERE) produce no RETURNING row.
  - The RETURNING result set is a regular result cursor — fetchall(), description, etc.
  - The rows ARE durably inserted; RETURNING does not consume the INSERT.

Implementation notes:
  - ``InsertFromResult`` (sql-codegen IR) gains a ``returning_columns`` field.
  - In the VM, ``_do_insert_from_result`` snapshots the source rows from
    ``st.result``, drains + inserts them, and re-populates ``st.result`` with
    the RETURNING output when ``returning_columns`` is non-empty.
  - The codegen extracts column names from the RETURNING expressions and passes
    them to ``InsertFromResult``; no new VM instructions are needed.

All tests oracle-verified against real sqlite3.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _both(sql: str, setup: list[str], params=()) -> tuple[list, list]:
    """Run *sql* on both sqlite3 and mini_sqlite; return (ref, got)."""
    ref = sqlite3.connect(":memory:")
    got = mini_sqlite.connect(":memory:")
    for s in setup:
        ref.execute(s)
        got.execute(s)
    return (
        ref.execute(sql, params).fetchall(),
        got.execute(sql, params).fetchall(),
    )


def _assert_both(sql: str, setup: list[str], params=()) -> None:
    ref, got = _both(sql, setup, params)
    assert got == ref, f"mini-sqlite={got!r}, sqlite3={ref!r}\nSQL: {sql}"


SRC_SETUP = [
    "CREATE TABLE src (id INTEGER, name TEXT, salary INTEGER)",
    "INSERT INTO src VALUES (1, 'Alice', 50000)",
    "INSERT INTO src VALUES (2, 'Bob',   60000)",
    "INSERT INTO src VALUES (3, 'Carol', 70000)",
    "CREATE TABLE dst (id INTEGER, name TEXT, salary INTEGER)",
]


# ---------------------------------------------------------------------------
# Basic RETURNING on INSERT … SELECT
# ---------------------------------------------------------------------------


class TestBasicReturning:
    """Core INSERT … SELECT RETURNING behaviour."""

    def test_returning_two_columns(self) -> None:
        """RETURNING two columns from a full-table SELECT."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src RETURNING id, name",
            SRC_SETUP,
        )

    def test_returning_single_column(self) -> None:
        """RETURNING a single column."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src RETURNING id",
            SRC_SETUP,
        )

    def test_returning_all_columns(self) -> None:
        """RETURNING all three columns — same values as the SELECT source."""
        _assert_both(
            "INSERT INTO dst SELECT id, name, salary FROM src RETURNING id, name, salary",
            SRC_SETUP,
        )

    def test_returning_last_column_only(self) -> None:
        """RETURNING a non-key column."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src RETURNING salary",
            SRC_SETUP,
        )

    def test_returning_one_row_selected(self) -> None:
        """INSERT … SELECT with a WHERE clause — RETURNING only the inserted row."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src WHERE id = 2 RETURNING id, name",
            SRC_SETUP,
        )

    def test_returning_no_rows_selected(self) -> None:
        """When WHERE filters everything out, RETURNING is an empty result set."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src WHERE id > 999 RETURNING id",
            SRC_SETUP,
        )

    def test_returning_filtered_subset(self) -> None:
        """Multiple rows from a filtered SELECT — RETURNING one row per insert."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src WHERE salary >= 60000 RETURNING id, salary",
            SRC_SETUP,
        )


# ---------------------------------------------------------------------------
# cursor.description correctness
# ---------------------------------------------------------------------------


class TestDescription:
    """RETURNING sets cursor.description to the RETURNING column names."""

    def test_description_single_column(self) -> None:
        """cursor.description matches the RETURNING column name."""
        c = mini_sqlite.connect(":memory:")
        for s in SRC_SETUP:
            c.execute(s)
        cur = c.execute("INSERT INTO dst SELECT * FROM src RETURNING id")
        assert cur.description is not None
        assert cur.description[0][0] == "id"

    def test_description_multiple_columns(self) -> None:
        """cursor.description lists all RETURNING columns in order."""
        c = mini_sqlite.connect(":memory:")
        for s in SRC_SETUP:
            c.execute(s)
        cur = c.execute(
            "INSERT INTO dst SELECT * FROM src RETURNING id, name, salary"
        )
        assert cur.description is not None
        col_names = [d[0] for d in cur.description]
        assert col_names == ["id", "name", "salary"]

    def test_rows_are_durably_inserted(self) -> None:
        """RETURNING does not consume the INSERT — rows survive afterwards."""
        c = mini_sqlite.connect(":memory:")
        for s in SRC_SETUP:
            c.execute(s)
        c.execute("INSERT INTO dst SELECT * FROM src RETURNING id")
        count = c.execute("SELECT COUNT(*) FROM dst").fetchone()[0]
        assert count == 3


# ---------------------------------------------------------------------------
# Order and cardinality
# ---------------------------------------------------------------------------


class TestOrderAndCardinality:
    """RETURNING row count and ordering match the inserted rows."""

    def test_row_count_matches_source(self) -> None:
        """Number of RETURNING rows equals number of rows inserted."""
        ref, got = _both(
            "INSERT INTO dst SELECT * FROM src RETURNING id",
            SRC_SETUP,
        )
        assert len(got) == 3
        assert len(got) == len(ref)

    def test_order_preserved(self) -> None:
        """RETURNING rows appear in insertion (source scan) order."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src ORDER BY id RETURNING id",
            SRC_SETUP,
        )

    def test_reverse_order(self) -> None:
        """Reverse-order INSERT … SELECT emits RETURNING in that same order."""
        _assert_both(
            "INSERT INTO dst SELECT * FROM src ORDER BY id DESC RETURNING id",
            SRC_SETUP,
        )


# ---------------------------------------------------------------------------
# ON CONFLICT … + RETURNING
# ---------------------------------------------------------------------------


class TestOnConflictReturning:
    """RETURNING interacts correctly with conflict handling."""

    def test_ignore_conflict_skips_returning_row(self) -> None:
        """ON CONFLICT IGNORE: skipped rows do NOT appear in RETURNING."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "CREATE TABLE src2 (id INTEGER, v TEXT)",
            "INSERT INTO src2 VALUES (1, 'a'), (2, 'b')",
            "INSERT INTO t VALUES (1, 'existing')",   # row 1 will conflict
        ]
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        for s in setup:
            ref.execute(s)
            got.execute(s)
        ref_rows = ref.execute(
            "INSERT OR IGNORE INTO t SELECT * FROM src2 RETURNING id, v"
        ).fetchall()
        got_rows = got.execute(
            "INSERT OR IGNORE INTO t SELECT * FROM src2 RETURNING id, v"
        ).fetchall()
        assert got_rows == ref_rows  # only row 2 returned
        assert got_rows == [(2, "b")]

    def test_replace_conflict_returning(self) -> None:
        """ON CONFLICT REPLACE: the replacing row appears in RETURNING."""
        setup = [
            "CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)",
            "CREATE TABLE src2 (id INTEGER, v TEXT)",
            "INSERT INTO src2 VALUES (1, 'new'), (2, 'also-new')",
            "INSERT INTO t VALUES (1, 'old')",
        ]
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        for s in setup:
            ref.execute(s)
            got.execute(s)
        ref_rows = ref.execute(
            "INSERT OR REPLACE INTO t SELECT * FROM src2 RETURNING id, v"
        ).fetchall()
        got_rows = got.execute(
            "INSERT OR REPLACE INTO t SELECT * FROM src2 RETURNING id, v"
        ).fetchall()
        assert got_rows == ref_rows


# ---------------------------------------------------------------------------
# Explicit column list (INSERT INTO t (a, b) SELECT …)
# ---------------------------------------------------------------------------


class TestExplicitColumnList:
    """RETURNING works when INSERT specifies an explicit target column list."""

    def test_explicit_column_list_returning(self) -> None:
        """INSERT INTO dst (id, name, salary) SELECT … RETURNING works."""
        _assert_both(
            "INSERT INTO dst (id, name, salary) SELECT id, name, salary "
            "FROM src RETURNING id, name",
            SRC_SETUP,
        )

    def test_partial_column_list_returning(self) -> None:
        """INSERT with a partial column list (two of three) RETURNING the inserted cols."""
        setup = [
            "CREATE TABLE src2 (id INTEGER, label TEXT)",
            "INSERT INTO src2 VALUES (10, 'x'), (20, 'y')",
            "CREATE TABLE dst2 (id INTEGER, label TEXT, extra TEXT)",
        ]
        _assert_both(
            "INSERT INTO dst2 (id, label) SELECT id, label FROM src2 RETURNING id, label",
            setup,
        )
