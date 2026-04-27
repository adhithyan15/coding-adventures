from __future__ import annotations

import pytest
from compiler_ir import IDGenerator, IrDataDecl, IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from wasm_assembler import assemble
from wasm_opcodes import get_opcode_by_name
from wasm_runtime import WasiConfig, WasiHost, WasmRuntime
from wasm_types import DataSegment, ExternalKind, GlobalType, Import, Limits, MemoryType, TableType, ValueType, WasmModule

from ir_to_wasm_assembly import (
    WasmAssemblyError,
    _blocktype_name,
    _bytes_csv,
    _format_instruction,
    emit_wasm_assembly,
    print_module,
)
from ir_to_wasm_compiler import FunctionSignature


def test_emit_readable_assembly_and_run_it() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_answer")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_answer")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    assembly = emit_wasm_assembly(
        program,
        [FunctionSignature(label="_fn_answer", param_count=0, export_name="answer")],
    )

    assert ".type 0 params=none results=i32" in assembly
    assert ".func 0 type=0" in assembly
    assert "i32.const 42" in assembly

    result = WasmRuntime().load_and_run(assemble(assembly), "answer", [])
    assert result == [42]


def test_emit_memory_and_data_assembly() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_read")
    program.add_data(IrDataDecl(label="buf", size=1, init=90))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_read")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_ADDR, [IrRegister(2), IrLabel("buf")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_BYTE, [IrRegister(1), IrRegister(2), IrRegister(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    assembly = emit_wasm_assembly(
        program,
        [FunctionSignature(label="_fn_read", param_count=0, export_name="read")],
    )

    assert ".memory 0 min=1 max=none" in assembly
    assert ".data 0 offset=0 bytes=5A" in assembly
    assert "i32.load8_u align=0 offset=0" in assembly
    assert WasmRuntime().load_and_run(assemble(assembly), "read", []) == [90]


def test_print_module_rejects_non_const_data_offset() -> None:
    module = WasmModule()
    module.data = [DataSegment(memory_index=0, offset_expr=b"\x23\x00\x0B", data=b"x")]

    with pytest.raises(WasmAssemblyError, match="only i32.const data offsets"):
        print_module(module)


def test_emit_imported_wasi_function_assembly_and_run_it() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(66)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(4)], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    assembly = emit_wasm_assembly(
        program,
        [FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )

    assert ".import function wasi_snapshot_preview1 fd_write" in assembly

    output: list[str] = []
    runtime = WasmRuntime(host=WasiHost(config=WasiConfig(stdout=output.append)))
    result = runtime.load_and_run(assemble(assembly), "_start", [])
    assert result == [0]
    assert output == ["B"]


def test_print_module_formats_all_import_kinds() -> None:
    module = WasmModule()
    module.imports = [
        Import(
            module_name="env",
            name="memory",
            kind=ExternalKind.MEMORY,
            type_info=MemoryType(limits=Limits(min=1, max=2)),
        ),
        Import(
            module_name="env",
            name="table",
            kind=ExternalKind.TABLE,
            type_info=TableType(limits=Limits(min=0, max=4)),
        ),
        Import(
            module_name="env",
            name="flag",
            kind=ExternalKind.GLOBAL,
            type_info=GlobalType(value_type=ValueType.I32, mutable=True),
        ),
    ]
    module.data = [DataSegment(memory_index=0, offset_expr=b"\x41\x00\x0B", data=b"")]

    assembly = print_module(module)

    assert ".import memory env memory min=1 max=2" in assembly
    assert ".import table env table elem=funcref min=0 max=4" in assembly
    assert ".import global env flag type=i32 mutable=true" in assembly
    assert ".data 0 offset=0 bytes=none" in assembly


def test_helper_formatting_and_errors() -> None:
    assert _blocktype_name(int(ValueType.I32)) == "i32"
    assert _blocktype_name(7) == "7"
    assert _bytes_csv(b"") == "none"

    with pytest.raises(WasmAssemblyError, match="unknown opcode byte"):
        _format_instruction(0xFF, None)

    memarg_opcode = get_opcode_by_name("i32.load").opcode
    with pytest.raises(WasmAssemblyError, match="expected memarg"):
        _format_instruction(memarg_opcode, 3)


def test_emit_wasm_assembly_rejects_invalid_program() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=IDGenerator().next()))

    with pytest.raises(WasmAssemblyError, match="unsupported SYSCALL"):
        emit_wasm_assembly(
            program,
            [FunctionSignature(label="_start", param_count=0, export_name="_start")],
        )
