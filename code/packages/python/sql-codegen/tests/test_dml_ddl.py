"""DML and DDL compilation — Insert, Update, Delete, CreateTable, DropTable."""

from __future__ import annotations

from sql_backend.schema import ColumnDef
from sql_planner import (
    BinaryExpr,
    Column,
    Literal,
)
from sql_planner import (
    BinaryOp as AstOp,
)
from sql_planner.plan import (
    Assignment,
    CreateTable,
    Delete,
    DropTable,
    EmptyResult,
    Insert,
    InsertSource,
    Update,
)

from sql_codegen import (
    BeginRow,
    DeleteRows,
    EmitColumn,
    EmitRow,
    InsertRow,
    JumpIfFalse,
    LoadColumn,
    LoadLastInsertedColumn,
    OpenScan,
    SetResultSchema,
    UpdateRows,
    compile,
)
from sql_codegen import (
    CreateTable as IrCreateTable,
)
from sql_codegen import (
    DropTable as IrDropTable,
)


def test_insert_values_emits_insert_row() -> None:
    plan = Insert(
        table="t",
        columns=("a", "b"),
        source=InsertSource(values=((Literal(1), Literal(2)),)),
    )
    prog = compile(plan)
    inserts = [i for i in prog.instructions if isinstance(i, InsertRow)]
    assert len(inserts) == 1
    assert inserts[0].table == "t"
    assert inserts[0].columns == ("a", "b")


def test_insert_multiple_rows() -> None:
    plan = Insert(
        table="t",
        columns=("a",),
        source=InsertSource(values=((Literal(1),), (Literal(2),), (Literal(3),))),
    )
    prog = compile(plan)
    inserts = [i for i in prog.instructions if isinstance(i, InsertRow)]
    assert len(inserts) == 3


def test_insert_select_compiles() -> None:
    """INSERT … SELECT now compiles to a SetResultSchema + scan body + InsertFromResult."""
    from sql_planner import Scan

    from sql_codegen.ir import InsertFromResult

    plan = Insert(
        table="t",
        columns=None,
        source=InsertSource(query=Scan(table="src")),
    )
    prog = compile(plan)
    # The program must contain an InsertFromResult targeting 't'.
    inserts = [i for i in prog.instructions if isinstance(i, InsertFromResult)]
    assert len(inserts) == 1
    assert inserts[0].table == "t"
    # Should NOT have a plain InsertRow (that's for VALUES inserts).
    from sql_codegen.ir import InsertRow
    assert not any(isinstance(i, InsertRow) for i in prog.instructions)


def test_update_without_predicate() -> None:
    plan = Update(
        table="t",
        assignments=(Assignment(column="x", value=Literal(42)),),
    )
    prog = compile(plan)
    updates = [i for i in prog.instructions if isinstance(i, UpdateRows)]
    assert len(updates) == 1
    assert updates[0].table == "t"
    assert updates[0].assignments == ("x",)


def test_update_with_predicate_emits_jump() -> None:
    plan = Update(
        table="t",
        assignments=(Assignment(column="x", value=Literal(1)),),
        predicate=BinaryExpr(op=AstOp.GT, left=Column("t", "x"), right=Literal(0)),
    )
    prog = compile(plan)
    jumps = [i for i in prog.instructions if isinstance(i, JumpIfFalse)]
    assert len(jumps) >= 1
    opens = [i for i in prog.instructions if isinstance(i, OpenScan)]
    assert opens[0].table == "t"


def test_delete_without_predicate() -> None:
    plan = Delete(table="t")
    prog = compile(plan)
    deletes = [i for i in prog.instructions if isinstance(i, DeleteRows)]
    assert len(deletes) == 1
    assert deletes[0].table == "t"


def test_delete_with_predicate_emits_jump() -> None:
    plan = Delete(
        table="t",
        predicate=BinaryExpr(op=AstOp.EQ, left=Column("t", "id"), right=Literal(1)),
    )
    prog = compile(plan)
    jumps = [i for i in prog.instructions if isinstance(i, JumpIfFalse)]
    assert len(jumps) >= 1


def test_create_table_emits_create_table() -> None:
    plan = CreateTable(
        table="users",
        columns=(
            ColumnDef(name="id", type_name="INTEGER", primary_key=True),
            ColumnDef(name="name", type_name="TEXT", not_null=True),
        ),
        if_not_exists=True,
    )
    prog = compile(plan)
    creates = [i for i in prog.instructions if isinstance(i, IrCreateTable)]
    assert len(creates) == 1
    assert creates[0].table == "users"
    assert creates[0].if_not_exists is True
    assert len(creates[0].columns) == 2


def test_drop_table_emits_drop_table() -> None:
    plan = DropTable(table="t", if_exists=True)
    prog = compile(plan)
    drops = [i for i in prog.instructions if isinstance(i, IrDropTable)]
    assert len(drops) == 1
    assert drops[0].table == "t"
    assert drops[0].if_exists is True


def test_empty_result_sets_schema() -> None:
    plan = EmptyResult(columns=("x", "y"))
    prog = compile(plan)
    schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
    assert schemas[0].columns == ("x", "y")
    assert prog.result_schema == ("x", "y")


# ---- RETURNING clause compilation ------------------------------------------


def test_insert_returning_emits_load_last_inserted_column() -> None:
    """INSERT … RETURNING id, name emits LoadLastInsertedColumn for each col."""
    plan = Insert(
        table="t",
        columns=("id", "name"),
        source=InsertSource(values=((Literal(1), Literal("Alice")),)),
        returning=(Column("t", "id"), Column("t", "name")),
    )
    prog = compile(plan)
    # SetResultSchema must be emitted once at the top.
    schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
    assert len(schemas) == 1
    assert schemas[0].columns == ("id", "name")
    # LoadLastInsertedColumn for each returning col.
    load_instr = [i for i in prog.instructions if isinstance(i, LoadLastInsertedColumn)]
    assert len(load_instr) == 2
    assert load_instr[0].col == "id"
    assert load_instr[1].col == "name"
    # BeginRow + EmitRow frame the RETURNING columns.
    assert any(isinstance(i, BeginRow) for i in prog.instructions)
    assert any(isinstance(i, EmitRow) for i in prog.instructions)
    emit_cols = [i for i in prog.instructions if isinstance(i, EmitColumn)]
    assert len(emit_cols) == 2
    assert emit_cols[0].name == "id"
    assert emit_cols[1].name == "name"


def test_insert_returning_multiple_rows() -> None:
    """INSERT two rows with RETURNING — each row emits a LoadLastInsertedColumn."""
    plan = Insert(
        table="t",
        columns=("id",),
        source=InsertSource(values=((Literal(1),), (Literal(2),))),
        returning=(Column("t", "id"),),
    )
    prog = compile(plan)
    # Two InsertRow, two LoadLastInsertedColumn (one per row).
    inserts = [i for i in prog.instructions if isinstance(i, InsertRow)]
    loads = [i for i in prog.instructions if isinstance(i, LoadLastInsertedColumn)]
    assert len(inserts) == 2
    assert len(loads) == 2


def test_insert_without_returning_no_schema() -> None:
    """INSERT without RETURNING emits no SetResultSchema."""
    plan = Insert(
        table="t",
        columns=("id",),
        source=InsertSource(values=((Literal(42),),)),
    )
    prog = compile(plan)
    schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
    assert len(schemas) == 0
    loads = [i for i in prog.instructions if isinstance(i, LoadLastInsertedColumn)]
    assert len(loads) == 0


def test_update_returning_emits_load_column_after_update() -> None:
    """UPDATE … RETURNING emits SetResultSchema + BeginRow + LoadColumn + EmitRow."""
    plan = Update(
        table="t",
        assignments=(Assignment(column="x", value=Literal(99)),),
        returning=(Column("t", "x"),),
    )
    prog = compile(plan)
    # SetResultSchema at top.
    schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
    assert len(schemas) == 1
    assert schemas[0].columns == ("x",)
    # LoadColumn for RETURNING x — cursor 0 is the scan cursor.
    load_cols = [i for i in prog.instructions if isinstance(i, LoadColumn)]
    assert any(lc.column == "x" for lc in load_cols)
    # BeginRow and EmitRow framing.
    assert any(isinstance(i, BeginRow) for i in prog.instructions)
    assert any(isinstance(i, EmitRow) for i in prog.instructions)
    # UpdateRows must precede the RETURNING block (RETURNING reads post-update row).
    instr_list = list(prog.instructions)
    update_idx = next(i for i, ins in enumerate(instr_list) if isinstance(ins, UpdateRows))
    begin_idx = next(
        (i for i, ins in enumerate(instr_list) if isinstance(ins, BeginRow)), None
    )
    assert begin_idx is not None and begin_idx > update_idx


def test_delete_returning_emits_load_column_before_delete() -> None:
    """DELETE … RETURNING emits RETURNING instructions BEFORE DeleteRows."""
    plan = Delete(
        table="t",
        returning=(Column("t", "id"),),
    )
    prog = compile(plan)
    schemas = [i for i in prog.instructions if isinstance(i, SetResultSchema)]
    assert len(schemas) == 1
    assert schemas[0].columns == ("id",)
    instr_list = list(prog.instructions)
    delete_idx = next(i for i, ins in enumerate(instr_list) if isinstance(ins, DeleteRows))
    # BeginRow must appear BEFORE DeleteRows (RETURNING reads pre-delete row).
    begin_idx = next(
        (i for i, ins in enumerate(instr_list) if isinstance(ins, BeginRow)), None
    )
    assert begin_idx is not None and begin_idx < delete_idx
    assert any(isinstance(i, EmitRow) for i in prog.instructions)
