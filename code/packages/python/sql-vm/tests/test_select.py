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

from sql_vm import QueryEvent, execute, set_event_listener
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


def test_sort_nulls_first_asc(employees: InMemoryBackend) -> None:
    """ASC sort places NULLs FIRST by SQLite-compatible default."""
    employees.insert(
        "employees",
        {"id": 6, "name": None, "dept": "x", "salary": 1, "active": True},
    )
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name")),),  # ASC default → NULLs first (SQLite)
    )
    result = execute(compile(plan), employees)
    first = result.rows[0][0]
    assert first is None


def test_sort_nulls_last_explicit(employees: InMemoryBackend) -> None:
    """Explicit NULLS LAST in ASC still puts NULLs at the end (overrides default)."""
    employees.insert(
        "employees",
        {"id": 6, "name": None, "dept": "x", "salary": 1, "active": True},
    )
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name"), nulls_first=False),),  # explicit NULLS LAST
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


def test_limit_offset_only(employees: InMemoryBackend) -> None:
    """LIMIT with an OFFSET but no COUNT skips the first N rows."""
    plan = Limit(
        input=Sort(
            input=Project(
                input=Scan(table="employees", alias="e"),
                items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
            ),
            keys=(SortKey(expr=Column("e", "name")),),
        ),
        count=None,   # exercises _do_limit's count-is-None branch
        offset=3,
    )
    result = execute(compile(plan), employees)
    # Sorted names are Alice, Bob, Carol, Dave, Eve — skip 3 → Dave, Eve
    assert [r[0] for r in result.rows] == ["Dave", "Eve"]


def test_sort_null_null_desc(employees: InMemoryBackend) -> None:
    """Two NULL rows in DESC sort must not crash the _Rev comparator.

    When multiple rows share NULL sort values, Python will compare their
    ``_Rev(None)`` wrappers.  ``_Rev.__lt__`` returns ``False`` when either
    operand is None — exercising the ``if self.v is None or other.v is None``
    branch.
    """
    employees.insert("employees", {"id": 6, "name": None, "dept": "x", "salary": 1, "active": True})
    employees.insert("employees", {"id": 7, "name": None, "dept": "y", "salary": 2, "active": True})
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name"), descending=True),),
    )
    result = execute(compile(plan), employees)
    # Both NULLs sort as FIRST in DESC (NULLS LAST for DESC means...).
    # We just want the sort to complete without error.
    names = [r[0] for r in result.rows]
    assert len(names) == 7  # 5 original + 2 NULLs inserted


def test_sort_nulls_both_none_asc(employees: InMemoryBackend) -> None:
    """Two NULL rows in ASC sort exercise the comparator's None-vs-None path.

    With SQLite-compatible defaults NULLs go FIRST in ASC; both NULL rows
    should land at the beginning of the result.
    """
    employees.insert("employees", {"id": 6, "name": None, "dept": "x", "salary": 1, "active": True})
    employees.insert("employees", {"id": 7, "name": None, "dept": "y", "salary": 2, "active": True})
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        ),
        keys=(SortKey(expr=Column("e", "name")),),  # ASC default → NULLs first
    )
    result = execute(compile(plan), employees)
    names = [r[0] for r in result.rows]
    # The two NULLs should appear at the beginning.
    assert names[0] is None
    assert names[1] is None


def test_sort_positional_column_idx(employees: InMemoryBackend) -> None:
    """ORDER BY N positional sort via column_idx uses index-based row lookup.

    This exercises the ``if k.column_idx is not None: idx = k.column_idx``
    branch in ``_do_sort``.  ``column_idx=0`` sorts by the first output column
    (salary here), exactly as ``ORDER BY 1`` would when salary is first.

    The planner sets ``positional_index`` on a ``SortKey`` for ``ORDER BY N``
    references; the codegen converts that to ``SortKey(column_idx=N-1)`` in the
    IR.  We construct the IR ``SortKey`` directly to test the VM independently
    of the planner.
    """
    from sql_codegen import compile as do_compile
    from sql_codegen.ir import SortResult
    from sql_planner import BinaryExpr, BinaryOp

    # Build a plan that projects two computed columns: salary*2 and name.
    # We then sort by the second column (name) using column_idx=1 directly.
    plan = Sort(
        input=Project(
            input=Scan(table="employees", alias="e"),
            items=(
                ProjectionItem(
                    expr=BinaryExpr(
                        op=BinaryOp.MUL,
                        left=Column("e", "salary"),
                        right=Literal(2),
                    ),
                    alias=None,  # display name will be "?"
                ),
                ProjectionItem(expr=Column("e", "name"), alias="name"),
            ),
        ),
        keys=(
            # Planner-level SortKey: positional_index=1 → sorts by "name"
            SortKey(expr=Column("e", "name"), positional_index=1),
        ),
    )
    prog = do_compile(plan)
    # Verify the SortResult key has column_idx set by the codegen.
    sort_instrs = [i for i in prog.instructions if isinstance(i, SortResult)]
    assert sort_instrs, "Expected a SortResult instruction"
    key = sort_instrs[0].keys[0]
    assert key.column_idx == 1, f"Expected column_idx=1, got {key.column_idx!r}"

    from sql_vm import execute
    result = execute(prog, employees)
    names = [r[1] for r in result.rows]
    assert names == sorted(names), "Rows should be sorted by name (column index 1)"


def test_unknown_table_raises(empty_backend: InMemoryBackend) -> None:
    plan = Project(
        input=Scan(table="nonexistent", alias="nope"),
        items=(ProjectionItem(expr=Column("nope", "x"), alias="x"),),
    )
    with pytest.raises(TableNotFound):
        execute(compile(plan), empty_backend)


def test_event_listener_receives_query_event(employees: InMemoryBackend) -> None:
    """set_event_listener installs a callback that fires after each scan.

    This exercises the event-listener code path in ``execute`` (lines 444-454
    of vm.py) and the ``set_event_listener`` helper (line 178).
    """
    events: list[QueryEvent] = []

    set_event_listener(events.append)
    try:
        plan = Project(
            input=Scan(table="employees", alias="e"),
            items=(ProjectionItem(expr=Column("e", "name"), alias="name"),),
        )
        execute(compile(plan), employees)
    finally:
        # Always restore: remove the listener so other tests are unaffected.
        set_event_listener(None)

    assert len(events) == 1
    ev = events[0]
    assert isinstance(ev, QueryEvent)
    assert ev.table == "employees"
    assert ev.rows_scanned == 5
    assert ev.rows_returned == 5
