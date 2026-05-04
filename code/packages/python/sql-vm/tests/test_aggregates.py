"""Aggregate execution — COUNT, SUM, AVG, MIN, MAX, GROUP_CONCAT, GROUP BY, HAVING."""

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


# ---------------------------------------------------------------------------
# GROUP_CONCAT
# ---------------------------------------------------------------------------


def test_group_concat_default_separator(employees: InMemoryBackend) -> None:
    """GROUP_CONCAT with no separator uses ',' (SQLite default)."""
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("e", "name")),
                alias="names",
                separator=None,   # None → VM defaults to ","
            ),
        ),
    )
    result = execute(compile(plan), employees)
    assert len(result.rows) == 1
    # All 5 names joined — order is insertion order for an in-memory backend.
    names = result.rows[0][0].split(",")
    assert sorted(names) == ["Alice", "Bob", "Carol", "Dave", "Eve"]


def test_group_concat_custom_separator(employees: InMemoryBackend) -> None:
    """GROUP_CONCAT with an explicit separator."""
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("e", "name")),
                alias="names",
                separator=" | ",
            ),
        ),
    )
    result = execute(compile(plan), employees)
    assert len(result.rows) == 1
    names = result.rows[0][0].split(" | ")
    assert sorted(names) == ["Alice", "Bob", "Carol", "Dave", "Eve"]


def test_group_concat_per_group(employees: InMemoryBackend) -> None:
    """GROUP_CONCAT within GROUP BY groups each partition independently."""
    from sql_planner import Sort
    from sql_planner.plan import SortKey as PlanSortKey

    agg = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(Column("e", "dept"),),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("e", "name")),
                alias="members",
                separator=",",
            ),
        ),
    )
    # Sort by dept so output is deterministic.
    plan = Sort(
        input=agg,
        keys=(PlanSortKey(expr=Column(table=None, col="dept"), descending=False),),
    )
    result = execute(compile(plan), employees)
    # eng has Alice, Bob, Eve; sales has Carol, Dave
    assert len(result.rows) == 2
    dept_eng = result.rows[0]
    dept_sales = result.rows[1]
    assert dept_eng[0] == "eng"
    assert sorted(dept_eng[1].split(",")) == ["Alice", "Bob", "Eve"]
    assert dept_sales[0] == "sales"
    assert sorted(dept_sales[1].split(",")) == ["Carol", "Dave"]


def test_group_concat_null_inputs_ignored(employees: InMemoryBackend) -> None:
    """GROUP_CONCAT ignores NULL values — consistent with COUNT/SUM/etc."""
    from sql_backend.schema import ColumnDef

    be = InMemoryBackend()
    be.create_table(
        "nulls",
        [ColumnDef(name="v", type_name="TEXT")],
        False,
    )
    be.insert("nulls", {"v": "a"})
    be.insert("nulls", {"v": None})
    be.insert("nulls", {"v": "b"})
    plan = Aggregate(
        input=Scan(table="nulls", alias="n"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("n", "v")),
                alias="result",
                separator=",",
            ),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows == (("a,b",),)


def test_group_concat_empty_group_returns_null() -> None:
    """GROUP_CONCAT over an empty table returns NULL — consistent with SUM."""
    from sql_backend.schema import ColumnDef

    be = InMemoryBackend()
    be.create_table("empty", [ColumnDef(name="v", type_name="TEXT")], False)
    plan = Aggregate(
        input=Scan(table="empty", alias="e"),
        group_by=(),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("e", "v")),
                alias="result",
                separator=",",
            ),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows == ((None,),)


def test_group_concat_numeric_values(employees: InMemoryBackend) -> None:
    """GROUP_CONCAT converts numeric values to strings."""
    plan = Aggregate(
        input=Scan(table="employees", alias="e"),
        group_by=(Column("e", "dept"),),
        aggregates=(
            AggregateItem(
                func=AggFunc.GROUP_CONCAT,
                arg=FuncArg(value=Column("e", "salary")),
                alias="salaries",
                separator=",",
            ),
        ),
    )
    result = execute(compile(plan), employees)
    # Just verify the result is a string with comma-separated numbers.
    for row in result.rows:
        parts = row[1].split(",")
        assert all(p.isdigit() for p in parts), f"Non-numeric part in {row[1]!r}"
