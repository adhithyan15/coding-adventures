"""End-to-end SELECT tests — planner → codegen → VM."""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_codegen import compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    Distinct,
    Filter,
    Literal,
    Project,
    ProjectionItem,
    Scan,
    Sort,
    Wildcard,
)
from sql_planner.plan import Limit, SortKey

from sql_vm import execute
from sql_vm.errors import TableNotFound


def _names(backend: InMemoryBackend) -> list[str]:
    plan = Project(
        input=Scan(table="employees", alias="e"),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )
    result = execute(compile(plan), backend)
    return [r[0] for r in result.rows]


def test_select_wildcard(employees: InMemoryBackend) -> None:
    plan = Project(
        input=Scan(table="employees", alias="e"),
        items=(ProjectionItem(expr=Wildcard(), alias=None),),
    )
    result = execute(compile(plan), employees)
    assert len(result.rows) == 5
    assert set(result.columns) >= {"id", "name", "dept", "salary", "active"}


def test_project_single_column(employees: InMemoryBackend) -> None:
    assert _names(employees) == ["Alice", "Bob", "Carol", "Dave", "Eve"]


def test_filter_eq(employees: InMemoryBackend) -> None:
    plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=BinaryExpr(
                op=BinaryOp.EQ, left=Column("e", "dept"), right=Literal("eng")
            ),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )
    result = execute(compile(plan), employees)
    assert sorted(r[0] for r in result.rows) == ["Alice", "Bob", "Eve"]


def test_filter_with_null_predicate_skips_row(employees: InMemoryBackend) -> None:
    # Insert a row with NULL dept. Filter on dept = 'eng'. NULL row should be
    # skipped because NULL = 'eng' evaluates to NULL, which JumpIfFalse treats
    # as false.
    employees.insert(
        "employees", {"id": 6, "name": "Nullius", "dept": None, "salary": 1, "active": True}
    )
    plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=BinaryExpr(
                op=BinaryOp.EQ, left=Column("e", "dept"), right=Literal("eng")
            ),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )
    result = execute(compile(plan), employees)
    names = [r[0] for r in result.rows]
    assert "Nullius" not in names


def test_sort_asc_desc(employees: InMemoryBackend) -> None:
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name"), descending=True),),
    )
    result = execute(compile(plan), employees)
    assert [r[0] for r in result.rows] == ["Eve", "Dave", "Carol", "Bob", "Alice"]


def test_sort_nulls_last(employees: InMemoryBackend) -> None:
    employees.insert(
        "employees",
        {"id": 6, "name": None, "dept": "x", "salary": 1, "active": True},
    )
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name")),),  # ASC default → NULLs last
    )
    result = execute(compile(plan), employees)
    last = result.rows[-1][0]
    assert last is None


def test_limit_offset(employees: InMemoryBackend) -> None:
    plan = Limit(
        input=Sort(
            input=Project(
                input=Scan(table="employees", alias="e"),
                items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
            ),
            keys=(SortKey(expr=Column("e", "name")),),
        ),
        count=2,
        offset=1,
    )
    result = execute(compile(plan), employees)
    assert [r[0] for r in result.rows] == ["Bob", "Carol"]


def test_distinct(employees: InMemoryBackend) -> None:
    plan = Distinct(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "dept"), alias="dept"),),
        ),
    )
    result = execute(compile(plan), employees)
    assert sorted(r[0] for r in result.rows) == ["eng", "sales"]


def test_empty_scan(empty_backend: InMemoryBackend) -> None:
    plan = Project(
        input=Scan(table="t", alias="t"),
        items=(ProjectionItem(expr=Column("t", "x"), alias="x"),),
    )
    result = execute(compile(plan), empty_backend)
    assert result.rows == ()
    assert result.columns == ("x",)


def test_unknown_table_raises(empty_backend: InMemoryBackend) -> None:
    plan = Project(
        input=Scan(table="nonexistent", alias="nope"),
        items=(ProjectionItem(expr=Column("nope", "x"), alias="x"),),
    )
    with pytest.raises(TableNotFound):
        execute(compile(plan), empty_backend)
