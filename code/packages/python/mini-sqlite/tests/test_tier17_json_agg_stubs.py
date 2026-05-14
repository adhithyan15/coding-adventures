"""Tier 17 — json_group_array / json_group_object aggregates + admin stubs.

Covers three new feature areas:

1. **``json_group_array(val)`` aggregate** — SQLite-compatible aggregate that
   accumulates non-NULL values from the group into a JSON array.  The scalar
   function with the same name previously existed as a no-op alias for
   ``json_array``; it is now a proper aggregate that collects values across
   rows.  Returns ``'[]'`` for an empty group (never NULL).

2. **``json_group_object(key, val)`` aggregate** — New aggregate that builds a
   JSON object from per-row key-value pairs.  Both key and value are arbitrary
   row expressions.  Rows with NULL key or NULL value are silently skipped.
   Duplicate keys: last writer wins.  Returns ``'{}'`` for an empty group.

3. **Admin statement stubs** — ``VACUUM``, ``ANALYZE``, ``REINDEX``, and
   ``EXPLAIN`` are intercepted before the parser so migration tools and ORM
   setup routines that call these statements do not raise ``ProgrammingError``.
   They return ``QueryResult(rows_affected=0)`` with no output rows.

Oracle notes (json aggregates):
  - Both ``json_group_array`` and ``json_group_object`` accumulate rows in
    insertion order in SQLite.  To avoid fragile order-dependent assertions,
    oracle comparisons for the full table (no GROUP BY) sort parsed arrays /
    dict keys before comparing.
  - VACUUM / ANALYZE / REINDEX return empty results in real sqlite3, which
    matches our stub behaviour exactly.
  - EXPLAIN returns query-plan rows in real sqlite3; our version returns no
    rows.  EXPLAIN tests therefore only verify that no exception is raised
    (not that the output is oracle-equivalent).
"""

from __future__ import annotations

import json
import sqlite3

import mini_sqlite

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _setup_both(setup: list[str]) -> tuple[sqlite3.Connection, mini_sqlite.Connection]:
    """Create fresh in-memory connections and run *setup* statements on both."""
    ref = sqlite3.connect(":memory:")
    got = mini_sqlite.connect(":memory:")
    for sql in setup:
        ref.execute(sql)
        got.execute(sql)
    return ref, got


def _exec_both(sql: str, setup: list[str]) -> tuple[list, list]:
    ref, got = _setup_both(setup)
    ref_rows = ref.execute(sql).fetchall()
    got_rows = got.execute(sql).fetchall()
    return ref_rows, got_rows


# ---------------------------------------------------------------------------
# json_group_array — basic
# ---------------------------------------------------------------------------


EMPLOYEE_SETUP = [
    "CREATE TABLE emp (name TEXT, dept TEXT, salary INTEGER)",
    "INSERT INTO emp VALUES ('Alice', 'eng', 90000)",
    "INSERT INTO emp VALUES ('Bob',   'eng', 80000)",
    "INSERT INTO emp VALUES ('Carol', 'sales', 70000)",
    "INSERT INTO emp VALUES ('Dave',  'sales', 65000)",
    "INSERT INTO emp VALUES ('Eve',   'eng', 75000)",
]


class TestJsonGroupArray:
    """json_group_array end-to-end integration tests."""

    def test_full_table_names(self) -> None:
        """Accumulates every name into a JSON array (order-insensitive compare)."""
        ref_rows, got_rows = _exec_both(
            "SELECT json_group_array(name) FROM emp",
            EMPLOYEE_SETUP,
        )
        assert len(got_rows) == 1
        ref_arr = sorted(json.loads(ref_rows[0][0]))
        got_arr = sorted(json.loads(got_rows[0][0]))
        assert got_arr == ref_arr

    def test_empty_table_returns_empty_array(self) -> None:
        """Returns '[]' (not NULL) for an empty table — oracle-verified."""
        ref_rows, got_rows = _exec_both(
            "SELECT json_group_array(name) FROM emp",
            ["CREATE TABLE emp (name TEXT)"],
        )
        assert got_rows == ref_rows
        assert json.loads(got_rows[0][0]) == []

    def test_null_values_skipped(self) -> None:
        """NULL values are silently excluded from the JSON array."""
        # Use parameterized inserts — inline NULL literal is not part of our SQL dialect.
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE emp (name TEXT)")
        conn.execute("INSERT INTO emp VALUES (?)", ("Alpha",))
        conn.execute("INSERT INTO emp VALUES (?)", (None,))   # NULL row → must be skipped
        conn.execute("INSERT INTO emp VALUES (?)", ("Beta",))
        got_arr = sorted(json.loads(
            conn.execute("SELECT json_group_array(name) FROM emp").fetchone()[0]
        ))
        assert got_arr == ["Alpha", "Beta"]

    def test_per_group_with_group_by(self) -> None:
        """GROUP BY partitions accumulate independently."""
        ref_rows, got_rows = _exec_both(
            "SELECT dept, json_group_array(name) FROM emp GROUP BY dept ORDER BY dept",
            EMPLOYEE_SETUP,
        )
        assert len(got_rows) == 2
        # eng: Alice, Bob, Eve  /  sales: Carol, Dave
        for (ref_dept, ref_json), (got_dept, got_json) in zip(ref_rows, got_rows, strict=True):
            assert got_dept == ref_dept
            assert sorted(json.loads(got_json)) == sorted(json.loads(ref_json))

    def test_integer_values(self) -> None:
        """Integer SQL values become JSON numbers (no quotes)."""
        ref_rows, got_rows = _exec_both(
            "SELECT json_group_array(salary) FROM emp",
            EMPLOYEE_SETUP,
        )
        ref_arr = sorted(json.loads(ref_rows[0][0]))
        got_arr = sorted(json.loads(got_rows[0][0]))
        assert got_arr == ref_arr
        assert all(isinstance(v, int) for v in got_arr)

    def test_mixed_with_filter(self) -> None:
        """WHERE clause applied before aggregation."""
        ref_rows, got_rows = _exec_both(
            "SELECT json_group_array(name) FROM emp WHERE salary > 75000",
            EMPLOYEE_SETUP,
        )
        ref_arr = sorted(json.loads(ref_rows[0][0]))
        got_arr = sorted(json.loads(got_rows[0][0]))
        assert got_arr == ref_arr
        assert sorted(got_arr) == ["Alice", "Bob"]

    def test_result_is_valid_json(self) -> None:
        """The result is always parseable JSON."""
        _, got_rows = _exec_both(
            "SELECT json_group_array(name) FROM emp",
            EMPLOYEE_SETUP,
        )
        result = got_rows[0][0]
        parsed = json.loads(result)
        assert isinstance(parsed, list)
        assert len(parsed) == 5

    def test_combined_with_count(self) -> None:
        """json_group_array can be mixed with other aggregates in the same query."""
        ref_rows, got_rows = _exec_both(
            "SELECT dept, COUNT(*), json_group_array(name) "
            "FROM emp GROUP BY dept ORDER BY dept",
            EMPLOYEE_SETUP,
        )
        assert len(got_rows) == len(ref_rows) == 2
        for (ref_dept, ref_cnt, ref_arr_str), (got_dept, got_cnt, got_arr_str) in zip(
            ref_rows, got_rows, strict=True
        ):
            assert got_dept == ref_dept
            assert got_cnt == ref_cnt
            assert sorted(json.loads(got_arr_str)) == sorted(json.loads(ref_arr_str))


# ---------------------------------------------------------------------------
# json_group_object — basic
# ---------------------------------------------------------------------------


class TestJsonGroupObject:
    """json_group_object end-to-end integration tests."""

    def test_basic_key_value_pairs(self) -> None:
        """Builds a JSON object mapping name → salary."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(name, salary) FROM emp",
            EMPLOYEE_SETUP,
        )
        assert len(got_rows) == 1
        obj = json.loads(got_rows[0][0])
        assert obj["Alice"] == 90000
        assert obj["Bob"] == 80000
        assert obj["Carol"] == 70000

    def test_empty_table_returns_empty_object(self) -> None:
        """Returns '{}' (not NULL) for an empty table — oracle-verified."""
        ref_rows, got_rows = _exec_both(
            "SELECT json_group_object(k, v) FROM t",
            [
                "CREATE TABLE t (k TEXT, v INTEGER)",
            ],
        )
        assert got_rows == ref_rows
        assert json.loads(got_rows[0][0]) == {}

    def test_null_value_rows_skipped(self) -> None:
        """Rows where value is NULL are silently excluded."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(k, v) FROM t",
            [
                "CREATE TABLE t (k TEXT, v INTEGER)",
                "INSERT INTO t VALUES ('a', 1)",
                "INSERT INTO t VALUES ('b', NULL)",
                "INSERT INTO t VALUES ('c', 3)",
            ],
        )
        obj = json.loads(got_rows[0][0])
        assert obj == {"a": 1, "c": 3}
        assert "b" not in obj

    def test_null_key_rows_skipped(self) -> None:
        """Rows where key is NULL are silently excluded."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(k, v) FROM t",
            [
                "CREATE TABLE t (k TEXT, v INTEGER)",
                "INSERT INTO t VALUES (NULL, 99)",
                "INSERT INTO t VALUES ('x', 1)",
            ],
        )
        obj = json.loads(got_rows[0][0])
        assert obj == {"x": 1}

    def test_duplicate_keys_last_writer_wins(self) -> None:
        """When the same key appears multiple times, the last row wins."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(k, v) FROM t",
            [
                "CREATE TABLE t (k TEXT, v INTEGER)",
                "INSERT INTO t VALUES ('x', 1)",
                "INSERT INTO t VALUES ('x', 99)",
            ],
        )
        obj = json.loads(got_rows[0][0])
        assert obj["x"] == 99

    def test_per_group_with_group_by(self) -> None:
        """GROUP BY partitions produce separate JSON objects."""
        _, got_rows = _exec_both(
            "SELECT dept, json_group_object(name, salary) "
            "FROM emp GROUP BY dept ORDER BY dept",
            EMPLOYEE_SETUP,
        )
        assert len(got_rows) == 2
        eng_obj = json.loads(got_rows[0][1])
        sales_obj = json.loads(got_rows[1][1])
        assert eng_obj["Alice"] == 90000
        assert eng_obj["Bob"] == 80000
        assert sales_obj["Carol"] == 70000

    def test_result_is_valid_json(self) -> None:
        """The result is always parseable JSON."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(name, salary) FROM emp",
            EMPLOYEE_SETUP,
        )
        parsed = json.loads(got_rows[0][0])
        assert isinstance(parsed, dict)
        assert len(parsed) == 5

    def test_string_values(self) -> None:
        """String values are stored as JSON strings (quoted)."""
        _, got_rows = _exec_both(
            "SELECT json_group_object(name, dept) FROM emp",
            EMPLOYEE_SETUP,
        )
        obj = json.loads(got_rows[0][0])
        assert obj["Alice"] == "eng"
        assert obj["Carol"] == "sales"


# ---------------------------------------------------------------------------
# Admin statement stubs — VACUUM, ANALYZE, REINDEX, EXPLAIN
# ---------------------------------------------------------------------------


class TestAdminStubs:
    """VACUUM / ANALYZE / REINDEX / EXPLAIN are no-ops in mini-sqlite."""

    def test_vacuum_no_error(self) -> None:
        """VACUUM returns no rows and does not raise."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.execute("VACUUM")
        assert cur.fetchall() == []

    def test_vacuum_oracle(self) -> None:
        """VACUUM matches real sqlite3 — both return no rows."""
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        ref_rows = ref.execute("VACUUM").fetchall()
        got_rows = got.execute("VACUUM").fetchall()
        assert got_rows == ref_rows == []

    def test_analyze_no_error(self) -> None:
        """ANALYZE returns no rows and does not raise."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.execute("ANALYZE")
        assert cur.fetchall() == []

    def test_analyze_oracle(self) -> None:
        """ANALYZE matches real sqlite3 — both return no rows."""
        ref = sqlite3.connect(":memory:")
        got = mini_sqlite.connect(":memory:")
        ref_rows = ref.execute("ANALYZE").fetchall()
        got_rows = got.execute("ANALYZE").fetchall()
        assert got_rows == ref_rows == []

    def test_reindex_no_error(self) -> None:
        """REINDEX returns no rows and does not raise."""
        conn = mini_sqlite.connect(":memory:")
        cur = conn.execute("REINDEX")
        assert cur.fetchall() == []

    def test_explain_no_error(self) -> None:
        """EXPLAIN SELECT does not raise (rows not oracle-compared)."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (1)")
        # EXPLAIN returns real query-plan rows in sqlite3 but empty rows in
        # mini-sqlite.  We only assert that no exception is raised.
        cur = conn.execute("EXPLAIN SELECT * FROM t")
        _ = cur.fetchall()  # any result is acceptable

    def test_explain_query_plan_no_error(self) -> None:
        """EXPLAIN QUERY PLAN does not raise."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        cur = conn.execute("EXPLAIN QUERY PLAN SELECT * FROM t")
        _ = cur.fetchall()

    def test_vacuum_after_dml(self) -> None:
        """VACUUM does not disturb previously inserted data."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x INTEGER)")
        conn.execute("INSERT INTO t VALUES (42)")
        conn.execute("VACUUM")
        rows = conn.execute("SELECT x FROM t").fetchall()
        assert rows == [(42,)]

    def test_analyze_after_dml(self) -> None:
        """ANALYZE does not disturb previously inserted data."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("CREATE TABLE t (x TEXT)")
        conn.execute("INSERT INTO t VALUES ('hello')")
        conn.execute("ANALYZE")
        rows = conn.execute("SELECT x FROM t").fetchall()
        assert rows == [("hello",)]

    def test_multiple_admin_stubs_in_sequence(self) -> None:
        """Multiple admin statements can be issued without error."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("VACUUM")
        conn.execute("ANALYZE")
        conn.execute("REINDEX")
        # If we get here without an exception, the test passes.

    def test_case_insensitive_vacuum(self) -> None:
        """Admin stubs are case-insensitive (lowercase works too)."""
        conn = mini_sqlite.connect(":memory:")
        conn.execute("vacuum")
        conn.execute("analyze")
        conn.execute("reindex")


# NOTE: non-finite float safety (inf/nan → JSON null) is tested at the VM unit
# level in sql-vm/tests/test_aggregates.py.  It cannot be tested here because
# mini-sqlite's parameter binding layer correctly rejects float('inf') /
# float('nan') before they reach the engine (ProgrammingError: cannot bind
# non-finite float).  The fix therefore protects against values that enter the
# aggregate state through other means (e.g., computed columns, UDFs, or future
# backend paths that bypass binding).
