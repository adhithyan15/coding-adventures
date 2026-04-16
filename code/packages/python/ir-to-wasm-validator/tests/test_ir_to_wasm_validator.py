from __future__ import annotations

from compiler_ir import IDGenerator, IrInstruction, IrLabel, IrOp, IrProgram
from ir_to_wasm_compiler import FunctionSignature

from ir_to_wasm_validator import WasmIrValidator


def test_validator_accepts_supported_program() -> None:
    program = IrProgram(entry_label="_fn_main")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_main")], id=-1))
    program.add_instruction(IrInstruction(IrOp.RET, [], id=IDGenerator().next()))

    errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_fn_main", param_count=0, export_name="main")],
    )

    assert errors == []


def test_validator_reports_lowering_error() -> None:
    program = IrProgram(entry_label="_fn_main")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_main")], id=-1))
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [], id=IDGenerator().next()))

    errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_fn_main", param_count=0, export_name="main")],
    )

    assert len(errors) == 1
    assert errors[0].rule == "lowering"
    assert "SYSCALL" in errors[0].message
