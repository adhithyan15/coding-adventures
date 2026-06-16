"""Oracle tests for literal-column display names and SELECT * from derived tables.

Root cause fixed: _column_display_name() in sql-codegen now returns the
SQLite-compatible surface representation for Literal expressions (e.g. "1",
"'hi'", "NULL").  Previously every unnamed literal got "?", so two integer
columns in the same projection both had key "?" and dict(zip(cols, row)) lost
all but the last value.

Every test below is an oracle check — both sqlite3 and mini_sqlite run the
same SQL and we assert the results are byte-for-byte identical.
"""

from __future__ import annotations

import sqlite3

import pytest

import mini_sqlite


def _both(sql: str, setup: list[str] | None = None) -> tuple[list[tuple], list[tuple]]:
    """Return (ref_rows, our_rows) for the given SQL."""
    ref = sqlite3.connect(":memory:")
    ours = mini_sqlite.connect(":memory:")
    for s in setup or []:
        ref.execute(s)
        ours.execute(s)
    return ref.execute(sql).fetchall(), ours.execute(sql).fetchall()


def _cols(sql: str, setup: list[str] | None = None) -> tuple[list[str], list[str]]:
    """Return (ref_col_names, our_col_names) for the given SQL."""
    ref = sqlite3.connect(":memory:")
    ours = mini_sqlite.connect(":memory:")
    for s in setup or []:
        ref.execute(s)
        ours.execute(s)
    rc = ref.cursor()
    oc = ours.cursor()
    rc.execute(sql)
    oc.execute(sql)
    return [d[0] for d in rc.description], [d[0] for d in oc.description]


# ---------------------------------------------------------------------------
# Column names for bare literal projections
# ---------------------------------------------------------------------------


class TestLiteralColumnNames:
    """cursor.description column names for unnamed literal expressions."""

    def test_integer_literal_column_name(self) -> None:
        ref_cols, our_cols = _cols("SELECT 1")
        assert our_cols == ref_cols  # SQLite: ["1"]

    def test_two_integer_literals_have_distinct_names(self) -> None:
        ref_cols, our_cols = _cols("SELECT 1, 2")
        assert our_cols == ref_cols  # SQLite: ["1", "2"]

    def test_three_integer_literals_distinct_names(self) -> None:
        ref_cols, our_cols = _cols("SELECT 1, 2, 3")
        assert our_cols == ref_cols

    def test_null_literal_column_name(self) -> None:
        ref_cols, our_cols = _cols("SELECT NULL")
        assert our_cols == ref_cols  # SQLite: ["NULL"]

    def test_string_literal_column_name(self) -> None:
        ref_cols, our_cols = _cols("SELECT 'hello'")
        assert our_cols == ref_cols  # SQLite: ["'hello'"]

    def test_float_literal_column_name(self) -> None:
        ref_cols, our_cols = _cols("SELECT 1.5")
        assert our_cols == ref_cols  # SQLite: ["1.5"]

    def test_alias_overrides_display_name(self) -> None:
        ref_cols, our_cols = _cols("SELECT 1 AS one")
        assert our_cols == ref_cols  # SQLite: ["one"]


# ---------------------------------------------------------------------------
# SELECT * FROM subquery with unnamed literal columns
# ---------------------------------------------------------------------------


class TestSubqueryLiteralColumns:
    """SELECT * from derived tables containing unnamed literal expressions."""

    def test_select_star_from_two_int_literals(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 1, 2)")
        assert ours == ref  # was (2,) — now (1, 2)

    def test_select_star_from_three_int_literals(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 1, 2, 3)")
        assert ours == ref  # was (3,) — now (1, 2, 3)

    def test_select_star_from_null_and_int(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT NULL, 1)")
        assert ours == ref

    def test_select_star_from_string_literals(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 'a', 'b')")
        assert ours == ref

    def test_select_star_from_mixed_literal_and_alias(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 1 AS x, 2)")
        assert ours == ref

    def test_select_star_from_all_aliased(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 1 AS a, 2 AS b)")
        assert ours == ref

    def test_select_star_from_single_int_literal(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 1)")
        assert ours == ref

    def test_select_star_from_real_table_columns_unchanged(self) -> None:
        setup = ["CREATE TABLE t(a INT, b INT)", "INSERT INTO t VALUES (10, 20)"]
        ref, ours = _both("SELECT * FROM (SELECT a, b FROM t)", setup)
        assert ours == ref

    def test_subquery_value_correctness_first_col(self) -> None:
        # Make sure we get the FIRST column's value, not the last
        ref, ours = _both("SELECT * FROM (SELECT 42, 99)")
        assert ours == [(42, 99)]
        assert ours == ref

    def test_subquery_value_correctness_ordering(self) -> None:
        ref, ours = _both("SELECT * FROM (SELECT 10, 20, 30)")
        assert ours == [(10, 20, 30)]
        assert ours == ref


class TestLiteralRoundTrips:
    """Parametric round-trip checks for bare integer literals."""

    @pytest.mark.parametrize("val", [0, -1, 100, 9999])
    def test_integer_literal_round_trips(self, val: int) -> None:
        ref, ours = _both(f"SELECT {val}")
        assert ours == ref
