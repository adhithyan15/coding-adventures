"""Correlated-subquery VM tests.

These tests verify that:

1. ``LoadOuterColumn`` reads the correct column from the outer cursor's
   current row (``_VmState.outer_current_row``).
2. The ``execute()`` ``outer_current_row`` parameter is threaded correctly
   into ``_VmState``.
3. Correlated ``IN``, ``NOT IN``, ``EXISTS``, and scalar subqueries all
   produce the expected results via the full planner → codegen → VM pipeline.
4. NULL semantics are preserved end-to-end in correlated contexts.

Background — how correlated subqueries work
--------------------------------------------

The planner converts ``table.col`` references that resolve only in the *outer*
scope into :class:`~sql_planner.expr.CorrelatedRef` nodes.  The codegen
compiles each ``CorrelatedRef`` to a
:class:`~sql_codegen.ir.LoadOuterColumn` instruction embedding the outer
cursor ID.  At runtime the three subquery execution handlers
(``RunExistsSubquery``, ``RunScalarSubquery``, ``RunInSubquery``) call
``execute()`` with ``outer_current_row=st.current_row`` so the inner
program's ``LoadOuterColumn`` resolves against the outer scan's snapshot.
"""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef
from sql_codegen import (
    BeginRow,
    EmitColumn,
    EmitRow,
    Halt,
    Label,
    LoadOuterColumn,
    Program,
    SetResultSchema,
)
from sql_codegen import compile as codegen_compile
from sql_planner import (
    BinaryExpr,
    BinaryOp,
    Column,
    CorrelatedRef,
    ExistsSubquery,
    Filter,
    InSubquery,
    Literal,
    Project,
    ProjectionItem,
    ScalarSubquery,
    Scan,
)

from sql_vm import execute

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture
def two_table_backend() -> InMemoryBackend:
    """Employees + departments in-memory backend for correlated-subquery tests.

    Schema::

        employees(id INTEGER, name TEXT, dept_id INTEGER)
        departments(id INTEGER, dept_name TEXT)

    Data::

        employees: Alice(1,10), Bob(2,20), Carol(3,10), Dave(4,30)
        departments: eng(10), sales(20)
        Dave belongs to dept 30 which has no entry in departments.
    """
    be = InMemoryBackend()
    be.create_table(
        "employees",
        [
            ColumnDef(name="id", type_name="INTEGER"),
            ColumnDef(name="name", type_name="TEXT"),
            ColumnDef(name="dept_id", type_name="INTEGER"),
        ],
        False,
    )
    be.create_table(
        "departments",
        [
            ColumnDef(name="id", type_name="INTEGER"),
            ColumnDef(name="dept_name", type_name="TEXT"),
        ],
        False,
    )
    for row in [
        {"id": 1, "name": "Alice", "dept_id": 10},
        {"id": 2, "name": "Bob", "dept_id": 20},
        {"id": 3, "name": "Carol", "dept_id": 10},
        {"id": 4, "name": "Dave", "dept_id": 30},
    ]:
        be.insert("employees", row)
    for row in [
        {"id": 10, "dept_name": "eng"},
        {"id": 20, "dept_name": "sales"},
    ]:
        be.insert("departments", row)
    return be


# ---------------------------------------------------------------------------
# Unit: LoadOuterColumn instruction directly
# ---------------------------------------------------------------------------


def test_load_outer_column_basic(two_table_backend: InMemoryBackend) -> None:
    """LoadOuterColumn reads the correct column from outer_current_row.

    We manually craft a tiny inner program that loads ``dept_id`` from the
    outer cursor 0 (employees scan) and emits it as the single result column.
    The inner program is executed via ``execute()`` with a hand-crafted
    ``outer_current_row`` snapshot.
    """
    # Inner program: emit one row containing the outer dept_id.
    inner_instrs = (
        SetResultSchema(columns=("outer_dept_id",)),
        BeginRow(),
        LoadOuterColumn(cursor_id=0, col="dept_id"),
        EmitColumn(name="outer_dept_id"),
        EmitRow(),
        Halt(),
    )
    inner_program = Program(
        instructions=inner_instrs,
        labels={ins.name: i for i, ins in enumerate(inner_instrs) if isinstance(ins, Label)},
        result_schema=("outer_dept_id",),
    )

    # Provide the outer current_row snapshot: cursor 0 has a fake employee row.
    fake_outer_row: dict[int, dict[str, object]] = {
        0: {"id": 42, "name": "TestUser", "dept_id": 999},
    }

    result = execute(inner_program, two_table_backend, outer_current_row=fake_outer_row)
    assert list(result.rows) == [(999,)]


def test_load_outer_column_missing_cursor(two_table_backend: InMemoryBackend) -> None:
    """LoadOuterColumn returns NULL when outer cursor ID is not in the snapshot."""
    inner_instrs = (
        SetResultSchema(columns=("val",)),
        BeginRow(),
        LoadOuterColumn(cursor_id=99, col="dept_id"),  # cursor 99 is not in snapshot
        EmitColumn(name="val"),
        EmitRow(),
        Halt(),
    )
    inner_program = Program(
        instructions=inner_instrs,
        labels={},
        result_schema=("val",),
    )

    result = execute(inner_program, two_table_backend, outer_current_row={0: {"dept_id": 10}})
    assert list(result.rows) == [(None,)]


def test_load_outer_column_missing_col(two_table_backend: InMemoryBackend) -> None:
    """LoadOuterColumn returns NULL when the column is absent from the outer row."""
    inner_instrs = (
        SetResultSchema(columns=("val",)),
        BeginRow(),
        LoadOuterColumn(cursor_id=0, col="nonexistent_col"),
        EmitColumn(name="val"),
        EmitRow(),
        Halt(),
    )
    inner_program = Program(
        instructions=inner_instrs,
        labels={},
        result_schema=("val",),
    )

    result = execute(inner_program, two_table_backend, outer_current_row={0: {"dept_id": 10}})
    assert list(result.rows) == [(None,)]


def test_outer_current_row_not_provided(two_table_backend: InMemoryBackend) -> None:
    """LoadOuterColumn returns NULL when execute() is called without outer_current_row."""
    inner_instrs = (
        SetResultSchema(columns=("val",)),
        BeginRow(),
        LoadOuterColumn(cursor_id=0, col="dept_id"),
        EmitColumn(name="val"),
        EmitRow(),
        Halt(),
    )
    inner_program = Program(
        instructions=inner_instrs,
        labels={},
        result_schema=("val",),
    )

    # No outer_current_row → defaults to empty dict → cursor 0 absent → NULL.
    result = execute(inner_program, two_table_backend)
    assert list(result.rows) == [(None,)]


# ---------------------------------------------------------------------------
# Integration: full planner → codegen → VM pipeline
# ---------------------------------------------------------------------------


def test_correlated_in_subquery_basic(two_table_backend: InMemoryBackend) -> None:
    """Correlated IN: employees whose dept_id appears in departments.id.

    SQL equivalent::

        SELECT name FROM employees AS e
        WHERE e.dept_id IN (SELECT id FROM departments AS d WHERE d.id = e.dept_id)

    Alice(10), Bob(20), Carol(10) match; Dave(30) does not.
    """
    # Build plan manually to avoid parser dependency in this test suite.
    # Inner plan: Filter(Scan(departments), dept.id = CorrelatedRef(e, dept_id)) → Project(id)
    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_filter = Filter(
        input=Scan(table="departments", alias="d"),
        predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
    )
    inner_plan = Project(
        input=inner_filter,
        items=(ProjectionItem(expr=Column("d", "id"), alias="id"),),
    )

    outer_plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=InSubquery(
                operand=Column("e", "dept_id"),
                query=inner_plan,
            ),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    names = sorted(r[0] for r in result.rows)
    assert names == ["Alice", "Bob", "Carol"]


def test_correlated_in_subquery_no_match(two_table_backend: InMemoryBackend) -> None:
    """Correlated IN where inner query always returns empty set → no rows."""
    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    # Filter that can never match: d.id = e.dept_id AND d.id > 1000
    inner_filter = Filter(
        input=Scan(table="departments", alias="d"),
        predicate=BinaryExpr(
            op=BinaryOp.AND,
            left=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
            right=BinaryExpr(op=BinaryOp.GT, left=Column("d", "id"), right=Literal(1000)),
        ),
    )
    inner_plan = Project(
        input=inner_filter,
        items=(ProjectionItem(expr=Column("d", "id"), alias="id"),),
    )
    outer_plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=InSubquery(operand=Column("e", "dept_id"), query=inner_plan),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    assert result.rows == ()


def test_correlated_scalar_subquery_in_select(two_table_backend: InMemoryBackend) -> None:
    """Correlated scalar subquery in SELECT list — dept name for each employee.

    SQL equivalent::

        SELECT e.name, (SELECT d.dept_name FROM departments AS d WHERE d.id = e.dept_id)
        FROM employees AS e

    Dave(30) has no matching department → NULL.
    """
    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_plan = Project(
        input=Filter(
            input=Scan(table="departments", alias="d"),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
        ),
        items=(ProjectionItem(expr=Column("d", "dept_name"), alias="dept_name"),),
    )

    outer_plan = Project(
        input=Scan(table="employees", alias="e"),
        items=(
            ProjectionItem(expr=Column("e", "name"), alias="name"),
            ProjectionItem(expr=ScalarSubquery(query=inner_plan), alias="dept"),
        ),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    # Build a name → dept dict from the result rows.
    name_to_dept = {r[0]: r[1] for r in result.rows}
    assert name_to_dept["Alice"] == "eng"
    assert name_to_dept["Bob"] == "sales"
    assert name_to_dept["Carol"] == "eng"
    assert name_to_dept["Dave"] is None  # no matching department


def test_correlated_exists_subquery(two_table_backend: InMemoryBackend) -> None:
    """Correlated EXISTS: employees who have a matching department row.

    SQL equivalent::

        SELECT e.name FROM employees AS e
        WHERE EXISTS (SELECT 1 FROM departments AS d WHERE d.id = e.dept_id)

    Alice, Bob, Carol have matching departments; Dave does not.
    """

    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_plan = Project(
        input=Filter(
            input=Scan(table="departments", alias="d"),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
        ),
        items=(ProjectionItem(expr=Literal(1), alias="one"),),
    )

    outer_plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=ExistsSubquery(query=inner_plan),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    names = sorted(r[0] for r in result.rows)
    assert names == ["Alice", "Bob", "Carol"]


def test_correlated_not_exists_subquery(two_table_backend: InMemoryBackend) -> None:
    """NOT EXISTS: employees whose dept has no department entry.

    SQL equivalent::

        SELECT e.name FROM employees AS e
        WHERE NOT EXISTS (SELECT 1 FROM departments AS d WHERE d.id = e.dept_id)

    Only Dave (dept 30) has no department entry.
    """
    from sql_planner.expr import UnaryExpr, UnaryOp

    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_plan = Project(
        input=Filter(
            input=Scan(table="departments", alias="d"),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
        ),
        items=(ProjectionItem(expr=Literal(1), alias="one"),),
    )

    # NOT EXISTS = UnaryExpr(NOT, ExistsSubquery(...))
    outer_plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=UnaryExpr(op=UnaryOp.NOT, operand=ExistsSubquery(query=inner_plan)),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    names = sorted(r[0] for r in result.rows)
    assert names == ["Dave"]


def test_correlated_in_subquery_with_not_in(two_table_backend: InMemoryBackend) -> None:
    """Correlated NOT IN: employees whose dept is NOT in departments.

    Only Dave(30) qualifies.
    """
    from sql_planner.expr import NotInSubquery

    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_plan = Project(
        input=Filter(
            input=Scan(table="departments", alias="d"),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
        ),
        items=(ProjectionItem(expr=Column("d", "id"), alias="id"),),
    )

    outer_plan = Project(
        input=Filter(
            input=Scan(table="employees", alias="e"),
            predicate=NotInSubquery(operand=Column("e", "dept_id"), query=inner_plan),
        ),
        items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    names = sorted(r[0] for r in result.rows)
    assert names == ["Dave"]


def test_correlated_subquery_reruns_per_outer_row(two_table_backend: InMemoryBackend) -> None:
    """The inner sub-program re-runs for each outer row and sees fresh outer values.

    If the inner program only ran once (caching bug), it would return the
    same result for all outer rows.  This test verifies independent per-row
    execution by checking that different employees get different dept names.
    """
    corr = CorrelatedRef(outer_alias="e", col="dept_id")
    inner_plan = Project(
        input=Filter(
            input=Scan(table="departments", alias="d"),
            predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("d", "id"), right=corr),
        ),
        items=(ProjectionItem(expr=Column("d", "dept_name"), alias="dept_name"),),
    )
    outer_plan = Project(
        input=Scan(table="employees", alias="e"),
        items=(
            ProjectionItem(expr=Column("e", "name"), alias="name"),
            ProjectionItem(expr=ScalarSubquery(query=inner_plan), alias="dept"),
        ),
    )

    program = codegen_compile(outer_plan)
    result = execute(program, two_table_backend)
    name_to_dept = {r[0]: r[1] for r in result.rows}

    # Alice and Carol are both in eng (dept 10), Bob is in sales (dept 20).
    # Dave has no department entry → NULL.
    assert name_to_dept["Alice"] == "eng"
    assert name_to_dept["Carol"] == "eng"
    assert name_to_dept["Bob"] == "sales"
    assert name_to_dept["Dave"] is None
    # Confirm all four employees are present (no accidental row loss).
    assert len(result.rows) == 4
