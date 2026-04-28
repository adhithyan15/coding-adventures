"""DML / DDL execution — Insert, Update, Delete, CreateTable, DropTable."""

from __future__ import annotations

import pytest
from sql_backend.in_memory import InMemoryBackend
from sql_backend.schema import ColumnDef as BackendColumnDef
from sql_codegen import compile
from sql_planner import (
    Assignment,
    BinaryExpr,
    BinaryOp,
    Column,
    CreateTable,
    Delete,
    DropTable,
    Insert,
    InsertSource,
    Literal,
    Update,
)
from sql_planner.ast import ColumnDef as AstColumnDef

from sql_vm import TableAlreadyExists, TableNotFound, execute


def _scan_rows(backend: InMemoryBackend, table: str) -> list[dict]:
    cur = backend.scan(table)
    rows: list[dict] = []
    while True:
        r = cur.next()
        if r is None:
            break
        rows.append(dict(r))
    cur.close()
    return rows


def test_create_table() -> None:
    be = InMemoryBackend()
    plan = CreateTable(
        table="widgets",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER", primary_key=True),
            AstColumnDef(name="label", type_name="TEXT"),
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 0
    assert "widgets" in be.tables()


def test_create_table_if_not_exists_swallows_duplicate() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    plan = CreateTable(
        table="t",
        columns=(AstColumnDef(name="x", type_name="INTEGER"),),
        if_not_exists=True,
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 0


def test_create_table_duplicate_raises() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    plan = CreateTable(
        table="t",
        columns=(AstColumnDef(name="x", type_name="INTEGER"),),
    )
    with pytest.raises(TableAlreadyExists):
        execute(compile(plan), be)


def test_drop_table() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    result = execute(compile(DropTable(table="t")), be)
    assert result.rows_affected == 0
    assert "t" not in be.tables()


def test_drop_table_missing_raises() -> None:
    be = InMemoryBackend()
    with pytest.raises(TableNotFound):
        execute(compile(DropTable(table="ghost")), be)


def test_drop_table_if_exists_is_quiet() -> None:
    be = InMemoryBackend()
    result = execute(compile(DropTable(table="ghost", if_exists=True)), be)
    assert result.rows_affected == 0


def test_insert_values() -> None:
    be = InMemoryBackend()
    be.create_table(
        "t",
        [
            BackendColumnDef(name="x", type_name="INTEGER"),
            BackendColumnDef(name="y", type_name="TEXT"),
        ],
        False,
    )
    plan = Insert(
        table="t",
        columns=("x", "y"),
        source=InsertSource(values=((Literal(1), Literal("a")), (Literal(2), Literal("b")))),
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 2
    assert _scan_rows(be, "t") == [{"x": 1, "y": "a"}, {"x": 2, "y": "b"}]


def test_update_all_rows() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": 1})
    be.insert("t", {"x": 2})
    plan = Update(
        table="t",
        assignments=(Assignment(column="x", value=Literal(99)),),
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 2
    assert _scan_rows(be, "t") == [{"x": 99}, {"x": 99}]


def test_update_with_predicate() -> None:
    be = InMemoryBackend()
    be.create_table(
        "t",
        [
            BackendColumnDef(name="id", type_name="INTEGER"),
            BackendColumnDef(name="val", type_name="INTEGER"),
        ],
        False,
    )
    be.insert("t", {"id": 1, "val": 10})
    be.insert("t", {"id": 2, "val": 20})
    be.insert("t", {"id": 3, "val": 30})
    plan = Update(
        table="t",
        assignments=(Assignment(column="val", value=Literal(0)),),
        predicate=BinaryExpr(
            op=BinaryOp.EQ, left=Column("t", "id"), right=Literal(2)
        ),
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 1
    rows = sorted(_scan_rows(be, "t"), key=lambda r: r["id"])
    assert rows == [{"id": 1, "val": 10}, {"id": 2, "val": 0}, {"id": 3, "val": 30}]


def test_delete_all_rows() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": 1})
    be.insert("t", {"x": 2})
    result = execute(compile(Delete(table="t")), be)
    assert result.rows_affected == 2
    assert _scan_rows(be, "t") == []


def test_delete_with_predicate() -> None:
    be = InMemoryBackend()
    be.create_table("t", [BackendColumnDef(name="x", type_name="INTEGER")], False)
    be.insert("t", {"x": 1})
    be.insert("t", {"x": 2})
    be.insert("t", {"x": 3})
    plan = Delete(
        table="t",
        predicate=BinaryExpr(op=BinaryOp.GT, left=Column("t", "x"), right=Literal(1)),
    )
    result = execute(compile(plan), be)
    assert result.rows_affected == 2
    assert _scan_rows(be, "t") == [{"x": 1}]


def test_insert_unknown_table_raises() -> None:
    be = InMemoryBackend()
    plan = Insert(
        table="ghost",
        columns=("x",),
        source=InsertSource(values=((Literal(1),),)),
    )
    with pytest.raises(TableNotFound):
        execute(compile(plan), be)


# --------------------------------------------------------------------------
# CHECK constraint tests — VM-level enforcement via check_registry.
# --------------------------------------------------------------------------


def _make_check_table(be: InMemoryBackend, check: object) -> None:
    """Create table t(id INTEGER, val INTEGER CHECK(<check>)) in be."""
    plan = CreateTable(
        table="t",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER", check_expr=None),
            AstColumnDef(name="val", type_name="INTEGER", check_expr=check),
        ),
    )
    registry: dict = {}
    execute(compile(plan), be, check_registry=registry)
    return registry


def test_check_constraint_insert_valid() -> None:
    """INSERT satisfying CHECK (val > 0) succeeds."""
    be = InMemoryBackend()
    # val > 0
    check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
    registry: dict = {}
    plan_create = CreateTable(
        table="t",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER"),
            AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
        ),
    )
    execute(compile(plan_create), be, check_registry=registry)

    plan_insert = Insert(
        table="t",
        columns=("id", "val"),
        source=InsertSource(values=((Literal(1), Literal(5)),)),
    )
    result = execute(compile(plan_insert), be, check_registry=registry)
    assert result.rows_affected == 1
    rows = _scan_rows(be, "t")
    assert rows == [{"id": 1, "val": 5}]


def test_check_constraint_insert_violates() -> None:
    """INSERT violating CHECK (val > 0) raises ConstraintViolation."""
    from sql_vm import ConstraintViolation

    be = InMemoryBackend()
    check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
    registry: dict = {}
    plan_create = CreateTable(
        table="t",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER"),
            AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
        ),
    )
    execute(compile(plan_create), be, check_registry=registry)

    plan_insert = Insert(
        table="t",
        columns=("id", "val"),
        source=InsertSource(values=((Literal(1), Literal(-1)),)),
    )
    with pytest.raises(ConstraintViolation) as exc_info:
        execute(compile(plan_insert), be, check_registry=registry)
    assert "CHECK constraint failed" in str(exc_info.value)


def test_check_constraint_update_violates() -> None:
    """UPDATE violating CHECK (val > 0) raises ConstraintViolation."""
    from sql_vm import ConstraintViolation

    be = InMemoryBackend()
    check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
    registry: dict = {}
    plan_create = CreateTable(
        table="t",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER"),
            AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
        ),
    )
    execute(compile(plan_create), be, check_registry=registry)
    be.insert("t", {"id": 1, "val": 10})

    plan_update = Update(
        table="t",
        assignments=(Assignment(column="val", value=Literal(-99)),),
    )
    with pytest.raises(ConstraintViolation):
        execute(compile(plan_update), be, check_registry=registry)
    # Row should be unchanged.
    assert _scan_rows(be, "t") == [{"id": 1, "val": 10}]


def test_check_null_passes() -> None:
    """NULL satisfies CHECK (val > 0) by SQL three-valued-logic convention."""
    be = InMemoryBackend()
    check_expr = BinaryExpr(op=BinaryOp.GT, left=Column(None, "val"), right=Literal(0))
    registry: dict = {}
    plan_create = CreateTable(
        table="t",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER"),
            AstColumnDef(name="val", type_name="INTEGER", check_expr=check_expr),
        ),
    )
    execute(compile(plan_create), be, check_registry=registry)

    plan_insert = Insert(
        table="t",
        columns=("id", "val"),
        source=InsertSource(values=((Literal(1), Literal(None)),)),
    )
    result = execute(compile(plan_insert), be, check_registry=registry)
    assert result.rows_affected == 1
