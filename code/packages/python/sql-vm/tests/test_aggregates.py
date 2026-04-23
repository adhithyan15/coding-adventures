"""Aggregate execution — COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING."""

from __future__ import annotations

from sql_backend.in_memory import InMemoryBackend
from sql_codegen import compile
from sql_planner import (
    AggFunc,
    Aggregate,
    AggregateItem,
    BinaryExpr,
    BinaryOp,
    Column,
    FuncArg,
    Having,
    Literal,
    Scan,
)
from sql_planner.expr import AggregateExpr

from sql_vm import execute


def test_count_star(employees: InMemoryBackend) -> None:
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(func=AggFunc.COUNT, arg=FuncArg(star=True), alias="n"),
        ),
    )
    result = execute(compile(plan), employees)
    assert result.rows == ((5,),)


def test_sum_salary(employees: InMemoryBackend) -> None:
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.SUM,
                arg=FuncArg(value=Column("e", "salary")),
                alias="total",
            ),
        ),
    )
    result = execute(compile(plan), employees)
    assert result.rows == ((400000,),)


def test_min_max_avg(employees: InMemoryBackend) -> None:
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.MIN, arg=FuncArg(value=Column("e", "salary")), alias="lo"
            ),
            AggregateItem(
                func=AggFunc.MAX, arg=FuncArg(value=Column("e", "salary")), alias="hi"
            ),
            AggregateItem(
                func=AggFunc.AVG, arg=FuncArg(value=Column("e", "salary")), alias="avg"
            ),
        ),
    )
    result = execute(compile(plan), employees)
    assert len(result.rows) == 1
    lo, hi, avg = result.rows[0]
    assert lo == 70000
    assert hi == 90000
    assert avg == 80000.0


def test_group_by_dept(employees: InMemoryBackend) -> None:
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(Column("e", "dept"),),
        aggregates=(
            AggregateItem(func=AggFunc.COUNT, arg=FuncArg(star=True), alias="n"),
        ),
    )
    result = execute(compile(plan), employees)
    counts = dict(result.rows)
    assert counts == {"eng": 3, "sales": 2}


def test_count_ignores_null_values() -> None:
    from sql_backend.schema import ColumnDef

    be = InMemoryBackend()
    be.create_table("t", [ColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": 1})
    be.insert("t", {"x": None})
    be.insert("t", {"x": 3})

    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.COUNT, arg=FuncArg(value=Column("t", "x")), alias="n"
            ),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows == ((2,),)


def test_sum_null_returns_null() -> None:
    from sql_backend.schema import ColumnDef

    be = InMemoryBackend()
    be.create_table("t", [ColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": None})
    be.insert("t", {"x": None})

    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.SUM, arg=FuncArg(value=Column("t", "x")), alias="s"
            ),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows == ((None,),)


def test_avg_empty_group() -> None:
    from sql_backend.schema import ColumnDef

    be = InMemoryBackend()
    be.create_table("t", [ColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": None})

    plan = Aggregate(
        input=Scan(table="t", alias="t"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.AVG, arg=FuncArg(value=Column("t", "x")), alias="a"
            ),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows == ((None,),)


def test_having_filters_groups(employees: InMemoryBackend) -> None:
    agg = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(Column("e", "dept"),),
        aggregates=(
            AggregateItem(func=AggFunc.COUNT, arg=FuncArg(star=True), alias="n"),
        ),
    )
    plan = Having(
        input=agg,
        predicate=BinaryExpr(
            op=BinaryOp.GT,
            left=AggregateExpr(func=AggFunc.COUNT, arg=None),
            right=Literal(2),
        ),
    )
    result = execute(compile(plan), employees)
    assert result.rows == (("eng", 3),)
