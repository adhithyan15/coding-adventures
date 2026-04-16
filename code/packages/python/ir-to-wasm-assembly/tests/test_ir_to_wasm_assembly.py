from __future__ import annotations

import pytest
from compiler_ir import IDGenerator, IrDataDecl, IrImmediate, IrInstruction, IrLabel, IrOp, IrProgram, IrRegister
from wasm_assembler import assemble
from wasm_runtime import WasmRuntime
from wasm_types import DataSegment, WasmModule

from ir_to_wasm_assembly import WasmAssemblyError, emit_wasm_assembly, print_module
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
