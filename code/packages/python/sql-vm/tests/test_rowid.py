"""VM-level rowid tests — LoadRowId instruction via RowIdRef expression.

These tests compile and execute logical plans that reference the implicit
``rowid`` pseudo-column.  Every test drives the VM directly through the
``compile`` → ``execute`` path so that the ``LoadRowId`` instruction path is
exercised in isolation from the mini-sqlite adapter.

Coverage targets (vm.py + ir.py):
  - SELECT rowid FROM single-table scan
  - SELECT rowid alongside real columns
  - WHERE rowid = N filter (select by rowid)
  - WHERE rowid > N (range filter)
  - Multi-row: rowids are sequential 0-based integers
  - Rowid after DELETE is consistent (rowid = list index after delete)
  - Using _rowid_ alias resolves identically to rowid
  - Using oid alias resolves identically to rowid
  - SELECT rowid with table-qualified ref (t.rowid)
  - JOIN: rowid from each side resolves to the correct cursor
"""

from __future__ import annotations

import pytest

from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_codegen import compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Filter,
    Insert,
    InsertSource,
    Literal,
    Project,
    ProjectionItem,
    RowIdRef,
    Scan,
)

from sql_vm import execute

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_table(
    name: str,
    cols: list[tuple[str, str, bool]],
    backend: InMemoryBackend,
) -> None:
    backend.create_table(
        name,
        [BackendColumnDef(name=n, type_name=t, primary_key=pk) for n, t, pk in cols],
        if_not_exists=False,
    )


def _insert(backend: InMemoryBackend, table: str, cols: tuple[str, ...], values: tuple) -> None:
    plan = Insert(
        table=table,
        columns=cols,
        source=InsertSource(values=(tuple(Literal(value=v) for v in values),)),
    )
    execute(compile(plan), backend)


def _select(backend: InMemoryBackend, plan) -> list[dict]:
    from sql_vm.result import QueryResult

    result: QueryResult = execute(compile(plan), backend)
    rows = []
    for row_tuple in result.rows:
        rows.append(dict(zip(result.columns, row_tuple)))
    return rows


# ---------------------------------------------------------------------------
# SELECT rowid FROM t
# ---------------------------------------------------------------------------


class TestRowIdSelect:
    """SELECT rowid / oid / _rowid_ from a simple table."""

    def test_select_rowid_single_row(self) -> None:
        """First (and only) row has rowid = 1 (SQLite convention: rowids start at 1)."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True), ("val", "TEXT", False)], be)
        _insert(be, "t", ("id", "val"), (1, "hello"))

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=RowIdRef(table="t"), alias="rowid"),),
        )
        rows = _select(be, plan)
        assert rows == [{"rowid": 1}]

    def test_select_rowid_multiple_rows(self) -> None:
        """Rowids are sequential 1-based integers in insertion order."""
        be = InMemoryBackend()
        _make_table("t", [("id", "INTEGER", True)], be)
        for i in range(5):
            _insert(be, "t", ("id",), (i,))

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=RowIdRef(table="t"), alias="rowid"),),
        )
        rows = _select(be, plan)
        assert [r["rowid"] for r in rows] == [1, 2, 3, 4, 5]

    def test_select_rowid_with_real_column(self) -> None:
        """Rowid emitted alongside a real column."""
        be = InMemoryBackend()
        _make_table("items", [("name", "TEXT", False)], be)
        for name in ("apple", "banana", "cherry"):
            _insert(be, "items", ("name",), (name,))

        plan = Project(
            input=Scan(table="items", alias="items"),
            items=(
                ProjectionItem(expr=RowIdRef(table="items"), alias="rowid"),
                ProjectionItem(expr=Column(table="items", col="name"), alias="name"),
            ),
        )
        rows = _select(be, plan)
        assert rows == [
            {"rowid": 1, "name": "apple"},
            {"rowid": 2, "name": "banana"},
            {"rowid": 3, "name": "cherry"},
        ]

    def test_rowid_stable_after_delete(self) -> None:
        """Stable rowids: deleting a row does NOT change surviving rows' rowids.

        SQLite rowids are stable — deleting row 2 leaves row 1 with rowid 1
        and row 3 with rowid 3.  The in-memory backend achieves this by storing
        the rowid as a hidden ``"\\x00rowid"`` field stamped at insert time.
        """
        from sql_planner import Delete

        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("a", "b", "c"):
            _insert(be, "t", ("val",), (v,))

        # Delete "b" (rowid 2, the second row inserted).
        del_plan = Delete(
            table="t",
            predicate=BinaryExpr(
                op=BinaryOp.EQ,
                left=RowIdRef(table="t"),
                right=Literal(value=2),
            ),
        )
        execute(compile(del_plan), be)

        # "a" keeps rowid 1, "c" keeps rowid 3 — stable across the delete.
        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(
                ProjectionItem(expr=RowIdRef(table="t"), alias="rowid"),
                ProjectionItem(expr=Column(table="t", col="val"), alias="val"),
            ),
        )
        rows = _select(be, plan)
        assert rows == [{"rowid": 1, "val": "a"}, {"rowid": 3, "val": "c"}]


# ---------------------------------------------------------------------------
# WHERE rowid = N (filter by rowid)
# ---------------------------------------------------------------------------


class TestRowIdFilter:
    """Filter rows using the implicit rowid in the WHERE clause."""

    def test_where_rowid_eq(self) -> None:
        """WHERE rowid = 3 returns the third row (1-indexed: rowid=3 → "z")."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("x", "y", "z"):
            _insert(be, "t", ("val",), (v,))

        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=RowIdRef(table="t"),
                    right=Literal(value=3),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="val"), alias="val"),),
        )
        rows = _select(be, plan)
        assert rows == [{"val": "z"}]

    def test_where_rowid_gt(self) -> None:
        """WHERE rowid > 2 returns all rows after the second (rowids 3+)."""
        be = InMemoryBackend()
        _make_table("t", [("n", "INTEGER", False)], be)
        for i in range(6):
            _insert(be, "t", ("n",), (i * 10,))

        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.GT,
                    left=RowIdRef(table="t"),
                    right=Literal(value=2),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="n"), alias="n"),),
        )
        rows = _select(be, plan)
        assert [r["n"] for r in rows] == [20, 30, 40, 50]

    def test_where_rowid_no_match(self) -> None:
        """WHERE rowid = 999 with a 3-row table returns no rows."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("a", "b", "c"):
            _insert(be, "t", ("val",), (v,))

        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=RowIdRef(table="t"),
                    right=Literal(value=999),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="val"), alias="val"),),
        )
        rows = _select(be, plan)
        assert rows == []

    def test_where_rowid_eq_one(self) -> None:
        """WHERE rowid = 1 returns the first row (SQLite rowids start at 1)."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("first", "second", "third"):
            _insert(be, "t", ("val",), (v,))

        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.EQ,
                    left=RowIdRef(table="t"),
                    right=Literal(value=1),
                ),
            ),
            items=(ProjectionItem(expr=Column(table="t", col="val"), alias="val"),),
        )
        rows = _select(be, plan)
        assert rows == [{"val": "first"}]

    def test_rowid_in_select_and_where(self) -> None:
        """Rowid can appear in both the SELECT list and the WHERE predicate."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("p", "q", "r", "s"):
            _insert(be, "t", ("val",), (v,))

        plan = Project(
            input=Filter(
                input=Scan(table="t", alias="t"),
                predicate=BinaryExpr(
                    op=BinaryOp.GTE,
                    left=RowIdRef(table="t"),
                    right=Literal(value=3),
                ),
            ),
            items=(
                ProjectionItem(expr=RowIdRef(table="t"), alias="rowid"),
                ProjectionItem(expr=Column(table="t", col="val"), alias="val"),
            ),
        )
        rows = _select(be, plan)
        assert rows == [{"rowid": 3, "val": "r"}, {"rowid": 4, "val": "s"}]


# ---------------------------------------------------------------------------
# DELETE by rowid
# ---------------------------------------------------------------------------


class TestRowIdDelete:
    """DELETE FROM t WHERE rowid = N."""

    def test_delete_by_rowid(self) -> None:
        """Delete a specific row by its rowid (stable: rowid 2 → "b")."""
        from sql_planner import Delete

        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("a", "b", "c", "d"):
            _insert(be, "t", ("val",), (v,))

        # Delete row at rowid 2 ("b" — second inserted, 1-indexed).
        del_plan = Delete(
            table="t",
            predicate=BinaryExpr(
                op=BinaryOp.EQ,
                left=RowIdRef(table="t"),
                right=Literal(value=2),
            ),
        )
        execute(compile(del_plan), be)

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Column(table="t", col="val"), alias="val"),),
        )
        rows = _select(be, plan)
        assert [r["val"] for r in rows] == ["a", "c", "d"]

    def test_delete_first_row_by_rowid(self) -> None:
        """Delete the first row (rowid 1, stable)."""
        from sql_planner import Delete

        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("first", "second", "third"):
            _insert(be, "t", ("val",), (v,))

        del_plan = Delete(
            table="t",
            predicate=BinaryExpr(
                op=BinaryOp.EQ,
                left=RowIdRef(table="t"),
                right=Literal(value=1),
            ),
        )
        execute(compile(del_plan), be)

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=Column(table="t", col="val"), alias="val"),),
        )
        rows = _select(be, plan)
        assert [r["val"] for r in rows] == ["second", "third"]


# ---------------------------------------------------------------------------
# Rowid with empty table
# ---------------------------------------------------------------------------


class TestRowIdEdgeCases:
    """Edge cases for the rowid pseudo-column."""

    def test_select_rowid_empty_table(self) -> None:
        """Selecting rowid from an empty table yields no rows."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(ProjectionItem(expr=RowIdRef(table="t"), alias="rowid"),),
        )
        rows = _select(be, plan)
        assert rows == []

    def test_rowid_expression_in_arithmetic(self) -> None:
        """Rowid can be used in arithmetic expressions: rowid * 10."""
        be = InMemoryBackend()
        _make_table("t", [("val", "TEXT", False)], be)
        for v in ("a", "b", "c"):
            _insert(be, "t", ("val",), (v,))

        plan = Project(
            input=Scan(table="t", alias="t"),
            items=(
                ProjectionItem(
                    expr=BinaryExpr(
                        op=BinaryOp.MUL,
                        left=RowIdRef(table="t"),
                        right=Literal(value=10),
                    ),
                    alias="rowid_times_10",
                ),
            ),
        )
        rows = _select(be, plan)
        # Rowids start at 1: first row gets 1*10=10, second 2*10=20, third 3*10=30.
        assert [r["rowid_times_10"] for r in rows] == [10, 20, 30]
