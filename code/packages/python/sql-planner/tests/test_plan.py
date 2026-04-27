"""LogicalPlan node construction, equality, and children() walk."""

from __future__ import annotations

import pytest

from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateItem,
    BinaryExpr,
    BinaryOp,
    Column,
    CreateTable,
    Delete,
    Distinct,
    DropTable,
    Filter,
    FuncArg,
    Having,
    Insert,
    InsertSource,
    JoinKind,
    Literal,
    Project,
    Scan,
    Sort,
    Union,
    Update,
    children,
)
from sql_planner.plan import Assignment, Join, Limit, SortKey


def _scan(name: str = "t") -> Scan:
    return Scan(table=name)


class TestNodeEquality:
    def test_scan_equal(self) -> None:
        assert Scan(table="t") == Scan(table="t")

    def test_filter_equal(self) -> None:
        p = BinaryExpr(op=BinaryOp.EQ, left=Column(None, "x"), right=Literal(value=1))
        assert Filter(input=_scan(), predicate=p) == Filter(input=_scan(), predicate=p)

    def test_frozen(self) -> None:
        s = _scan()
        with pytest.raises(Exception):  # noqa: B017 — FrozenInstanceError
            s.table = "other"  # type: ignore[misc]


class TestInsertSource:
    def test_values_only(self) -> None:
        src = InsertSource(values=((Literal(value=1),),))
        assert src.values is not None
        assert src.query is None

    def test_query_only(self) -> None:
        src = InsertSource(query=_scan())
        assert src.values is None
        assert src.query is not None

    def test_both_rejected(self) -> None:
        with pytest.raises(ValueError):
            InsertSource(values=((Literal(value=1),),), query=_scan())

    def test_neither_rejected(self) -> None:
        with pytest.raises(ValueError):
            InsertSource()


class TestChildren:
    def test_scan(self) -> None:
        assert children(_scan()) == ()

    def test_create_table(self) -> None:
        assert children(CreateTable(table="t", columns=())) == ()

    def test_drop_table(self) -> None:
        assert children(DropTable(table="t")) == ()

    def test_filter(self) -> None:
        n = Filter(input=_scan(), predicate=Literal(value=True))
        assert children(n) == (n.input,)

    def test_project(self) -> None:
        n = Project(input=_scan(), items=())
        assert children(n) == (n.input,)

    def test_aggregate(self) -> None:
        n = Aggregate(input=_scan(), group_by=(), aggregates=())
        assert children(n) == (n.input,)

    def test_having(self) -> None:
        n = Having(input=_scan(), predicate=Literal(value=True))
        assert children(n) == (n.input,)

    def test_sort(self) -> None:
        n = Sort(input=_scan(), keys=())
        assert children(n) == (n.input,)

    def test_limit(self) -> None:
        n = Limit(input=_scan(), count=5)
        assert children(n) == (n.input,)

    def test_distinct(self) -> None:
        n = Distinct(input=_scan())
        assert children(n) == (n.input,)

    def test_join(self) -> None:
        left, right = _scan("a"), _scan("b")
        n = Join(left=left, right=right, kind=JoinKind.CROSS)
        assert children(n) == (left, right)

    def test_union(self) -> None:
        left, right = _scan("a"), _scan("b")
        n = Union(left=left, right=right)
        assert children(n) == (left, right)

    def test_insert_with_values(self) -> None:
        n = Insert(table="t", columns=None, source=InsertSource(values=()))
        assert children(n) == ()

    def test_insert_with_subquery(self) -> None:
        sub = _scan("src")
        n = Insert(table="t", columns=None, source=InsertSource(query=sub))
        assert children(n) == (sub,)

    def test_update(self) -> None:
        n = Update(table="t", assignments=(Assignment(column="x", value=Literal(1)),))
        assert children(n) == ()

    def test_delete(self) -> None:
        n = Delete(table="t")
        assert children(n) == ()


class TestAggregateItem:
    def test_default_distinct_false(self) -> None:
        ai = AggregateItem(func=AggFunc.COUNT, arg=FuncArg(star=True), alias="n")
        assert ai.distinct is False


class TestSortKey:
    def test_default_ascending(self) -> None:
        k = SortKey(expr=Column(None, "x"))
        assert k.descending is False
        assert k.nulls_first is None
