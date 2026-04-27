from __future__ import annotations

from compiler_ir import (
    IDGenerator,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
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
    program.add_instruction(
        IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=IDGenerator().next())
    )

    errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_fn_main", param_count=0, export_name="main")],
    )

    assert len(errors) == 1
    assert errors[0].rule == "lowering"
    assert "unsupported SYSCALL" in errors[0].message


def test_validator_accepts_supported_syscall_program() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.SYSCALL,
            [IrImmediate(1), IrRegister(4)],
            id=IDGenerator().next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=IDGenerator().next()))

    errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )

    assert errors == []


def test_validator_accepts_dispatch_loop_strategy() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.JUMP, [IrLabel("_done")], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_skip")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM,
            [IrRegister(1), IrImmediate(99)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_done")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_IMM,
            [IrRegister(1), IrImmediate(7)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    structured_errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )
    dispatch_errors = WasmIrValidator().validate(
        program,
        [FunctionSignature(label="_start", param_count=0, export_name="_start")],
        strategy="dispatch_loop",
    )

    assert "unexpected unstructured control flow" in structured_errors[0].message
    assert dispatch_errors == []
