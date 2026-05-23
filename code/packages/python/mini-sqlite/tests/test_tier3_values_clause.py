"""``VALUES (a,b),(c,d)`` — standalone query, table source, CTE body.

SQLite (and the SQL standard) accepts ``VALUES`` as a first-class
query expression anywhere a SELECT can appear:

* **Standalone**: ``VALUES (1, 'a'), (2, 'b')`` is a top-level
  statement that returns a rowset.
* **Table source**: ``SELECT * FROM (VALUES (1), (2))`` — a derived
  table whose rows are the VALUES tuples.
* **CTE body**: ``WITH t(n) AS (VALUES (1), (2)) SELECT n FROM t`` —
  every place a CTE accepts a ``query_stmt``.
* **Set-op operand**: ``SELECT 1 UNION ALL VALUES (2)`` and the
  symmetric ``VALUES (1) UNION SELECT 2``.

Mini-sqlite desugars VALUES into a left-deep UNION-ALL chain of
single-row SELECTs, so downstream layers (planner, codegen, VM) see
only constructs they already handle.  Output columns are named
``column1``, ``column2``, … (1-indexed) when no explicit alias list
is given — this matches SQLite.

These oracle tests pin byte-identical results against stdlib
``sqlite3``.
"""

from __future__ import annotations

import sqlite3

import mini_sqlite


def _check(query: str) -> None:
    m = list(mini_sqlite.connect(":memory:").execute(query))
    r = list(sqlite3.connect(":memory:").execute(query))
    assert m == r, f"SQL: {query!r}\n  mini: {m}\n  ref:  {r}"


# ---------------------------------------------------------------------------
# Top-level VALUES statement.
# ---------------------------------------------------------------------------


class TestTopLevelValues:
    def test_single_row_single_column(self) -> None:
        _check("VALUES (1)")

    def test_single_row_two_columns(self) -> None:
        _check("VALUES (1, 2)")

    def test_multi_row(self) -> None:
        _check("VALUES (1, 'a'), (2, 'b'), (3, 'c')")

    def test_duplicate_rows_preserved(self) -> None:
        # UNION ALL (not UNION) semantics — duplicates survive.
        _check("VALUES (1), (1), (1)")

    def test_null_in_tuple(self) -> None:
        _check("VALUES (1, NULL), (NULL, 2)")

    def test_expressions_in_tuple(self) -> None:
        _check("VALUES (1+2, 3*4)")

    def test_string_concat_in_tuple(self) -> None:
        _check("VALUES ('foo' || 'bar')")


# ---------------------------------------------------------------------------
# VALUES as table source — synthetic ``column1``, ``column2`` names.
# ---------------------------------------------------------------------------


class TestValuesAsTableSource:
    def test_basic(self) -> None:
        _check("SELECT * FROM (VALUES (1, 'a'), (2, 'b'))")

    def test_with_alias(self) -> None:
        _check("SELECT * FROM (VALUES (1, 'a'), (2, 'b')) AS v")

    def test_column1_default_name(self) -> None:
        # SQLite names columns column1, column2, … when no alias list
        # is given; the WHERE below relies on that name.
        _check("SELECT column1 FROM (VALUES (1+1)) WHERE column1 > 1")

    def test_column1_plus_column2(self) -> None:
        _check("SELECT column1 + column2 FROM (VALUES (1, 2), (3, 4))")

    def test_select_star_includes_all_columns(self) -> None:
        _check("SELECT column1, column2 FROM (VALUES (1, NULL), (2, 'b'))")


# ---------------------------------------------------------------------------
# VALUES inside CTE bodies — both with and without explicit column
# aliases.  The alias case exercises ``_apply_cte_col_aliases`` walking
# down the left spine of the UNION-ALL chain to find the leftmost
# SelectStmt.
# ---------------------------------------------------------------------------


class TestValuesInCTE:
    def test_unaliased(self) -> None:
        _check("WITH x AS (VALUES (1), (2)) SELECT * FROM x")

    def test_explicit_column_alias(self) -> None:
        _check("WITH x(n) AS (VALUES (10), (20)) SELECT n FROM x ORDER BY n")

    def test_two_column_alias_list(self) -> None:
        _check(
            "WITH x(a, b) AS (VALUES (1, 'x'), (2, 'y')) "
            "SELECT a, b FROM x ORDER BY a"
        )

    def test_chained_ctes(self) -> None:
        _check(
            "WITH a(x) AS (VALUES (1), (2), (3)), "
            "     b AS (SELECT x*10 AS y FROM a) "
            "SELECT y FROM b ORDER BY y"
        )


# ---------------------------------------------------------------------------
# VALUES as a set-op operand — symmetric in either position.
# ---------------------------------------------------------------------------


class TestValuesInSetOp:
    def test_select_union_all_values(self) -> None:
        _check("SELECT 1 UNION ALL VALUES (2)")

    def test_values_union_select(self) -> None:
        _check("VALUES (1) UNION SELECT 2")

    def test_values_union_values(self) -> None:
        # Two VALUES queries set-op'd together.
        _check("VALUES (1) UNION VALUES (2)")

    def test_values_intersect_select(self) -> None:
        _check("VALUES (1), (2), (3) INTERSECT SELECT 2")

    def test_values_except_select(self) -> None:
        _check("VALUES (1), (2), (3) EXCEPT SELECT 2")
