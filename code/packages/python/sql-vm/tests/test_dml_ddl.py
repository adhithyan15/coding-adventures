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


# --------------------------------------------------------------------------
# FOREIGN KEY constraint tests — VM-level enforcement via fk_child/fk_parent.
# --------------------------------------------------------------------------


def _make_fk_tables(
    be: InMemoryBackend,
) -> tuple[dict, dict]:
    """Create parents(id PK) and children(id, parent_id → parents.id).

    Returns (fk_child, fk_parent) registries populated by execute().
    """
    fk_c: dict = {}
    fk_p: dict = {}

    plan_parent = CreateTable(
        table="parents",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER", primary_key=True),
        ),
    )
    execute(compile(plan_parent), be, fk_child=fk_c, fk_parent=fk_p)

    plan_child = CreateTable(
        table="children",
        columns=(
            AstColumnDef(name="id", type_name="INTEGER"),
            AstColumnDef(
                name="parent_id",
                type_name="INTEGER",
                foreign_key=("parents", "id"),
            ),
        ),
    )
    execute(compile(plan_child), be, fk_child=fk_c, fk_parent=fk_p)
    return fk_c, fk_p


def test_fk_insert_valid() -> None:
    """INSERT child with existing parent row passes."""
    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)
    be.insert("parents", {"id": 1})

    plan = Insert(
        table="children",
        columns=("id", "parent_id"),
        source=InsertSource(values=((Literal(10), Literal(1)),)),
    )
    result = execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert result.rows_affected == 1
    assert _scan_rows(be, "children") == [{"id": 10, "parent_id": 1}]


def test_fk_insert_violates() -> None:
    """INSERT child with missing parent raises ConstraintViolation."""
    from sql_vm import ConstraintViolation

    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)

    plan = Insert(
        table="children",
        columns=("id", "parent_id"),
        source=InsertSource(values=((Literal(1), Literal(99)),)),
    )
    with pytest.raises(ConstraintViolation) as exc_info:
        execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert "FOREIGN KEY" in str(exc_info.value)


def test_fk_null_child_passes() -> None:
    """NULL FK value is allowed (unknown reference)."""
    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)

    plan = Insert(
        table="children",
        columns=("id", "parent_id"),
        source=InsertSource(values=((Literal(1), Literal(None)),)),
    )
    result = execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert result.rows_affected == 1


def test_fk_update_child_violates() -> None:
    """UPDATE child FK to non-existent parent raises ConstraintViolation."""
    from sql_vm import ConstraintViolation

    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)
    be.insert("parents", {"id": 1})
    be.insert("children", {"id": 10, "parent_id": 1})

    plan = Update(
        table="children",
        assignments=(Assignment(column="parent_id", value=Literal(999)),),
    )
    with pytest.raises(ConstraintViolation):
        execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert _scan_rows(be, "children") == [{"id": 10, "parent_id": 1}]


def test_fk_delete_parent_restricted() -> None:
    """DELETE parent row that has referencing children raises ConstraintViolation."""
    from sql_vm import ConstraintViolation

    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)
    be.insert("parents", {"id": 1})
    be.insert("children", {"id": 10, "parent_id": 1})

    plan = Delete(
        table="parents",
        predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("parents", "id"), right=Literal(1)),
    )
    with pytest.raises(ConstraintViolation) as exc_info:
        execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert "FOREIGN KEY" in str(exc_info.value)
    assert _scan_rows(be, "parents") == [{"id": 1}]


def test_fk_delete_parent_no_children() -> None:
    """DELETE parent row with no children succeeds."""
    be = InMemoryBackend()
    fk_c, fk_p = _make_fk_tables(be)
    be.insert("parents", {"id": 1})
    be.insert("parents", {"id": 2})

    plan = Delete(
        table="parents",
        predicate=BinaryExpr(op=BinaryOp.EQ, left=Column("parents", "id"), right=Literal(1)),
    )
    result = execute(compile(plan), be, fk_child=fk_c, fk_parent=fk_p)
    assert result.rows_affected == 1
    assert _scan_rows(be, "parents") == [{"id": 2}]


# ---- RETURNING clause execution --------------------------------------------


def _make_employee_backend() -> InMemoryBackend:
    """Return a backend with an 'employees' table containing id, name, salary."""
    be = InMemoryBackend()
    be.create_table(
        "employees",
        [
            BackendColumnDef(name="id", type_name="INTEGER"),
            BackendColumnDef(name="name", type_name="TEXT"),
            BackendColumnDef(name="salary", type_name="INTEGER"),
        ],
        False,
    )
    return be


def test_insert_returning_single_row() -> None:
    """INSERT … RETURNING id, name returns the inserted row."""
    be = _make_employee_backend()
    plan = Insert(
        table="employees",
        columns=("id", "name", "salary"),
        source=InsertSource(values=((Literal(1), Literal("Alice"), Literal(50000)),)),
        returning=(Column("employees", "id"), Column("employees", "name")),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id", "name")
    assert list(result.rows) == [(1, "Alice")]
    # The row must be stored in the backend too.
    assert _scan_rows(be, "employees") == [{"id": 1, "name": "Alice", "salary": 50000}]


def test_insert_returning_multiple_rows() -> None:
    """INSERT two rows with RETURNING — result contains both rows in order."""
    be = _make_employee_backend()
    plan = Insert(
        table="employees",
        columns=("id", "name", "salary"),
        source=InsertSource(values=(
            (Literal(1), Literal("Alice"), Literal(50000)),
            (Literal(2), Literal("Bob"), Literal(60000)),
        )),
        returning=(Column("employees", "id"), Column("employees", "salary")),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id", "salary")
    assert list(result.rows) == [(1, 50000), (2, 60000)]


def test_insert_returning_salary_column() -> None:
    """INSERT … RETURNING salary reads back the inserted salary value."""
    be = _make_employee_backend()
    plan = Insert(
        table="employees",
        columns=("id", "name", "salary"),
        source=InsertSource(values=((Literal(99), Literal("Z"), Literal(99999)),)),
        returning=(Column("employees", "salary"),),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("salary",)
    assert list(result.rows) == [(99999,)]


def test_update_returning_updated_values() -> None:
    """UPDATE … RETURNING shows post-update row values."""
    be = _make_employee_backend()
    be.insert("employees", {"id": 1, "name": "Alice", "salary": 50000})
    be.insert("employees", {"id": 2, "name": "Bob", "salary": 60000})
    plan = Update(
        table="employees",
        assignments=(Assignment(column="salary", value=Literal(75000)),),
        predicate=BinaryExpr(
            op=BinaryOp.EQ, left=Column("employees", "id"), right=Literal(1)
        ),
        returning=(Column("employees", "id"), Column("employees", "salary")),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id", "salary")
    assert list(result.rows) == [(1, 75000)]


def test_update_returning_all_matched_rows() -> None:
    """UPDATE without WHERE returns one RETURNING row per affected row."""
    be = _make_employee_backend()
    be.insert("employees", {"id": 1, "name": "Alice", "salary": 50000})
    be.insert("employees", {"id": 2, "name": "Bob", "salary": 60000})
    plan = Update(
        table="employees",
        assignments=(Assignment(column="salary", value=Literal(99999)),),
        returning=(Column("employees", "id"),),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id",)
    assert len(result.rows) == 2


def test_delete_returning_deleted_row() -> None:
    """DELETE … RETURNING returns the deleted row's values before deletion."""
    be = _make_employee_backend()
    be.insert("employees", {"id": 1, "name": "Alice", "salary": 50000})
    be.insert("employees", {"id": 2, "name": "Bob", "salary": 60000})
    plan = Delete(
        table="employees",
        predicate=BinaryExpr(
            op=BinaryOp.EQ, left=Column("employees", "id"), right=Literal(2)
        ),
        returning=(Column("employees", "id"), Column("employees", "name")),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id", "name")
    assert list(result.rows) == [(2, "Bob")]
    # Verify the row was actually deleted.
    remaining = _scan_rows(be, "employees")
    assert len(remaining) == 1
    assert remaining[0]["id"] == 1


def test_delete_returning_multiple_rows() -> None:
    """DELETE all rows with RETURNING — each deleted row appears in result."""
    be = _make_employee_backend()
    be.insert("employees", {"id": 1, "name": "Alice", "salary": 50000})
    be.insert("employees", {"id": 2, "name": "Bob", "salary": 60000})
    be.insert("employees", {"id": 3, "name": "Charlie", "salary": 70000})
    plan = Delete(
        table="employees",
        returning=(Column("employees", "id"),),
    )
    result = execute(compile(plan), be)
    assert result.columns == ("id",)
    assert len(result.rows) == 3
    assert _scan_rows(be, "employees") == []
