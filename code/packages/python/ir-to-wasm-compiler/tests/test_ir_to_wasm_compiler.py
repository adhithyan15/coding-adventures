from __future__ import annotations

import pytest
from compiler_ir import (
    IDGenerator,
    IrDataDecl,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from wasm_module_encoder import encode_module
from wasm_runtime import ProcExitError, WasiConfig, WasiHost, WasmRuntime

from ir_to_wasm_compiler import (
    FunctionSignature,
    IrToWasmCompiler,
    WasmLoweringError,
    infer_function_signatures_from_comments,
)


def _runtime_result(module, export_name: str, args: list[int], host=None) -> list[int | float]:
    runtime = WasmRuntime(host=host)
    wasm_bytes = encode_module(module)
    return runtime.load_and_run(wasm_bytes, export_name, args)


def test_compile_simple_function_with_addition() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_double")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_double")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(2)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_fn_double", param_count=1, export_name="double")],
    )

    assert _runtime_result(module, "double", [6]) == [12]


def test_compile_if_pattern_to_wasm() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_choose")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_choose")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.BRANCH_Z, [IrRegister(2), IrLabel("if_0_else")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(10)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("if_0_end")], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("if_0_else")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(20)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("if_0_end")], id=-1))
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_fn_choose", param_count=1, export_name="choose")],
    )

    assert _runtime_result(module, "choose", [1]) == [10]
    assert _runtime_result(module, "choose", [0]) == [20]


def test_compile_loop_pattern_to_wasm() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_count")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_count")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [IrRegister(5), IrRegister(2), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("loop_0_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.CMP_LT, [IrRegister(6), IrRegister(4), IrRegister(5)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.BRANCH_Z, [IrRegister(6), IrLabel("loop_0_end")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [IrRegister(3), IrRegister(3), IrImmediate(1)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [IrRegister(4), IrRegister(4), IrImmediate(1)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("loop_0_start")], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("loop_0_end")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(3), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_fn_count", param_count=1, export_name="count")],
    )

    assert _runtime_result(module, "count", [4]) == [4]


def test_compile_memory_ops_and_data_layout() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_store_read")
    program.add_data(IrDataDecl(label="buf", size=1, init=0))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_store_read")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_ADDR, [IrRegister(2), IrLabel("buf")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(90)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.STORE_BYTE, [IrRegister(4), IrRegister(2), IrRegister(3)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_BYTE, [IrRegister(1), IrRegister(2), IrRegister(3)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(label="_fn_store_read", param_count=0, export_name="store_read")
        ],
    )

    assert any(export.name == "memory" for export in module.exports)
    assert _runtime_result(module, "store_read", []) == [90]


def test_compile_function_call_and_run_it() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.CALL, [IrLabel("_fn_double")], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_double")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.ADD, [IrRegister(1), IrRegister(2), IrRegister(2)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(label="_start", param_count=0, export_name="_start"),
            FunctionSignature(label="_fn_double", param_count=1, export_name="double"),
        ],
    )

    assert _runtime_result(module, "_start", []) == [10]
    assert _runtime_result(module, "double", [8]) == [16]


def test_infer_function_signatures_from_comments() -> None:
    program = IrProgram(entry_label="_fn_add")
    program.add_instruction(IrInstruction(IrOp.COMMENT, [IrLabel("function: add(a: u4, b: u4)")], id=-1))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_add")], id=-1))
    signatures = infer_function_signatures_from_comments(program)

    assert signatures["_fn_add"].param_count == 2
    assert signatures["_fn_add"].export_name == "add"


def test_missing_function_signature_raises() -> None:
    program = IrProgram(entry_label="_fn_add")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_add")], id=-1))

    try:
        IrToWasmCompiler().compile(program)
    except WasmLoweringError as exc:
        assert "missing function signature" in str(exc)
    else:  # pragma: no cover - defensive branch
        raise AssertionError("expected missing signature error")


def test_compile_syscall_write_uses_wasi_fd_write() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(65)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(1)], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )

    output: list[str] = []
    host = WasiHost(config=WasiConfig(stdout=output.append))
    assert _runtime_result(module, "_start", [], host=host) == [0]
    assert output == ["A"]
    assert [imp.name for imp in module.imports] == ["fd_write"]


def test_compile_syscall_read_uses_wasi_fd_read() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(2)], id=gen.next()))
    program.add_instruction(
        IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(4), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )

    host = WasiHost(config=WasiConfig(stdin=lambda _n: b"Z"))
    assert _runtime_result(module, "_start", [], host=host) == [90]
    assert [imp.name for imp in module.imports] == ["fd_read"]


def test_compile_syscall_exit_uses_wasi_proc_exit() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(7)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(10)], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[FunctionSignature(label="_start", param_count=0, export_name="_start")],
    )

    with pytest.raises(ProcExitError) as exc_info:
        _runtime_result(module, "_start", [], host=WasiHost())
    assert exc_info.value.exit_code == 7
    assert [imp.name for imp in module.imports] == ["proc_exit"]


def test_unsupported_syscall_raises() -> None:
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(99)], id=IDGenerator().next()))

    try:
        IrToWasmCompiler().compile(
            program,
            function_signatures=[FunctionSignature(label="_start", param_count=0, export_name="_start")],
        )
    except WasmLoweringError as exc:
        assert "unsupported SYSCALL" in str(exc)
    else:  # pragma: no cover - defensive branch
        raise AssertionError("expected unsupported syscall error")
