"""Program structure — label resolution, schema, Halt termination."""

from __future__ import annotations

from sql_planner import Column, Project, ProjectionItem, Scan

from sql_codegen import (
    AdvanceCursor,
    CloseScan,
    Halt,
    Jump,
    Label,
    OpenScan,
    Program,
    compile,
)


def test_program_is_frozen_dataclass() -> None:
    prog = Program(instructions=(Halt(),), labels={}, result_schema=())
    # Equal structure → equal objects.
    assert prog == Program(instructions=(Halt(),), labels={}, result_schema=())


def test_compile_returns_program() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    assert isinstance(prog, Program)


def test_program_ends_with_halt() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    assert isinstance(prog.instructions[-1], Halt)


def test_labels_resolve_to_indices() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    # Every Label in the stream should be in the labels dict.
    for i, ins in enumerate(prog.instructions):
        if isinstance(ins, Label):
            assert prog.labels[ins.name] == i


def test_scan_emits_open_advance_jump_close_pattern() -> None:
    plan = Scan(table="t", alias="t")
    prog = compile(plan)
    kinds = [type(i).__name__ for i in prog.instructions]
    # Order: OpenScan, Label, AdvanceCursor, ..., Jump, Label, CloseScan, Halt.
    assert kinds[0] == "OpenScan"
    assert kinds[1] == "Label"
    assert kinds[2] == "AdvanceCursor"
    assert kinds[-1] == "Halt"
    assert kinds[-2] == "CloseScan"


def test_advance_target_is_registered() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    advance = next(i for i in prog.instructions if isinstance(i, AdvanceCursor))
    # on_exhausted is a label name that must resolve.
    assert advance.on_exhausted in prog.labels


def test_jump_target_is_registered() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    jumps = [i for i in prog.instructions if isinstance(i, Jump)]
    for j in jumps:
        assert j.label in prog.labels


def test_project_schema_is_set() -> None:
    plan = Project(
        input=Scan(table="t", alias="t"),
        items=(
            ProjectionItem(expr=Column("t", "x"), alias="x"),
            ProjectionItem(expr=Column("t", "y"), alias=None),
        ),
    )
    prog = compile(plan)
    assert prog.result_schema == ("x", "y")


def test_cursor_is_closed() -> None:
    plan = Scan(table="t")
    prog = compile(plan)
    closes = [i for i in prog.instructions if isinstance(i, CloseScan)]
    opens = [i for i in prog.instructions if isinstance(i, OpenScan)]
    # Every OpenScan is paired with a CloseScan on the same cursor.
    assert {c.cursor_id for c in closes} == {o.cursor_id for o in opens}
