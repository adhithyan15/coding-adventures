from __future__ import annotations

import pytest
from compiler_ir import (
    IDGenerator,
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOp,
    IrProgram,
    IrRegister,
)
from wasm_module_encoder import encode_module
from wasm_runtime import ProcExitError, WasiConfig, WasiHost, WasmRuntime
from wasm_types import ValueType

from ir_to_wasm_compiler import (
    FunctionSignature,
    IrToWasmCompiler,
    WasmLoweringError,
    infer_function_signatures_from_comments,
    validate_for_wasm,
)


def _runtime_result(module, export_name: str, args: list[int | float], host=None) -> list[int | float]:
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


def test_compile_f64_function_with_typed_signature() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_add_real")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_add_real")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.F64_ADD,
            [IrRegister(31), IrRegister(2), IrRegister(3)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_add_real",
                param_count=2,
                export_name="add_real",
                param_types=(ValueType.F64, ValueType.F64),
                result_types=(ValueType.F64,),
            )
        ],
    )

    assert _runtime_result(module, "add_real", [1.25, 2.5]) == [3.75]


def test_compile_f64_function_can_call_i32_function() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_real_from_double")
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [IrLabel("_fn_real_from_double")], id=-1)
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.CALL, [IrLabel("_fn_double")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.F64_FROM_I32,
            [IrRegister(31), IrRegister(1)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [IrLabel("_fn_double")], id=-1)
    )
    program.add_instruction(
        IrInstruction(
            IrOp.ADD,
            [IrRegister(1), IrRegister(2), IrRegister(2)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_real_from_double",
                param_count=0,
                export_name="real_from_double",
                result_types=(ValueType.F64,),
            ),
            FunctionSignature(label="_fn_double", param_count=1),
        ],
    )

    assert _runtime_result(module, "real_from_double", []) == [14.0]


def test_compile_f64_memory_load_store() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_store_read_real")
    program.add_data(IrDataDecl(label="real_buf", size=8, init=0))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_store_read_real")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_ADDR, [IrRegister(2), IrLabel("real_buf")], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(0)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_F64_IMM,
            [IrRegister(4), IrFloatImmediate(6.5)],
            id=gen.next(),
        )
    )
    program.add_instruction(
        IrInstruction(
            IrOp.STORE_F64,
            [IrRegister(4), IrRegister(2), IrRegister(3)],
            id=gen.next(),
        )
    )
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_F64,
            [IrRegister(31), IrRegister(2), IrRegister(3)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_store_read_real",
                param_count=0,
                export_name="store_read_real",
                result_types=(ValueType.F64,),
            )
        ],
    )

    assert _runtime_result(module, "store_read_real", []) == [6.5]


def test_compile_i32_to_f64_conversion_and_compare() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_gt_three_point_five")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_gt_three_point_five")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.F64_FROM_I32, [IrRegister(3), IrRegister(2)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.LOAD_F64_IMM,
            [IrRegister(4), IrFloatImmediate(3.5)],
            id=gen.next(),
        )
    )
    program.add_instruction(
        IrInstruction(
            IrOp.F64_CMP_GT,
            [IrRegister(1), IrRegister(3), IrRegister(4)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_gt_three_point_five",
                param_count=1,
                export_name="gt_three_point_five",
                param_types=(ValueType.I32,),
                result_types=(ValueType.I32,),
            )
        ],
    )

    assert _runtime_result(module, "gt_three_point_five", [4]) == [1]
    assert _runtime_result(module, "gt_three_point_five", [3]) == [0]


def test_compile_f64_to_i32_truncation() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_trunc_real")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_trunc_real")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.I32_TRUNC_FROM_F64,
            [IrRegister(1), IrRegister(2)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_trunc_real",
                param_count=1,
                export_name="trunc_real",
                param_types=(ValueType.F64,),
                result_types=(ValueType.I32,),
            )
        ],
    )

    assert _runtime_result(module, "trunc_real", [3.75]) == [3]
    assert _runtime_result(module, "trunc_real", [-2.9]) == [-2]


def test_compile_f64_sqrt() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_sqrt_real")
    program.add_instruction(
        IrInstruction(IrOp.LABEL, [IrLabel("_fn_sqrt_real")], id=-1)
    )
    program.add_instruction(
        IrInstruction(IrOp.F64_SQRT, [IrRegister(31), IrRegister(2)], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(
                label="_fn_sqrt_real",
                param_count=1,
                export_name="sqrt_real",
                param_types=(ValueType.F64,),
                result_types=(ValueType.F64,),
            )
        ],
    )

    assert _runtime_result(module, "sqrt_real", [9.0]) == [3.0]
    assert _runtime_result(module, "sqrt_real", [0.25]) == [0.5]


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


def test_compile_function_call_with_explicit_argument_registers() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(10), IrImmediate(7)], id=gen.next())
    )
    program.add_instruction(
        IrInstruction(
            IrOp.CALL,
            [IrLabel("_fn_double"), IrRegister(10)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_double")], id=-1))
    program.add_instruction(
        IrInstruction(
            IrOp.ADD,
            [IrRegister(1), IrRegister(2), IrRegister(2)],
            id=gen.next(),
        )
    )
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    module = IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(label="_start", param_count=0, export_name="_start"),
            FunctionSignature(label="_fn_double", param_count=1, export_name="double"),
        ],
    )

    assert _runtime_result(module, "_start", []) == [14]


def test_compile_function_call_requires_explicit_argument_registers() -> None:
    gen = IDGenerator()
    program = IrProgram(entry_label="_start")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    program.add_instruction(
        IrInstruction(IrOp.CALL, [IrLabel("_fn_double")], id=gen.next())
    )
    program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_double")], id=-1))
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))

    with pytest.raises(WasmLoweringError, match="explicit argument register"):
        IrToWasmCompiler().compile(
            program,
            function_signatures=[
                FunctionSignature(label="_start", param_count=0, export_name="_start"),
                FunctionSignature(
                    label="_fn_double",
                    param_count=1,
                    require_explicit_args=True,
                ),
            ],
        )


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
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(4)], id=gen.next()))
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
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(2), IrRegister(4)], id=gen.next()))
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
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(10), IrRegister(4)], id=gen.next()))

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
    program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(99), IrRegister(4)], id=IDGenerator().next()))

    try:
        IrToWasmCompiler().compile(
            program,
            function_signatures=[FunctionSignature(label="_start", param_count=0, export_name="_start")],
        )
    except WasmLoweringError as exc:
        assert "unsupported SYSCALL" in str(exc)
    else:  # pragma: no cover - defensive branch
        raise AssertionError("expected unsupported syscall error")


# ──────────────────────────────────────────────────────────────────────────────
# Dispatch-loop lowerer tests
#
# These tests exercise the _DispatchLoopLowerer directly via
# IrToWasmCompiler.compile(..., strategy="dispatch_loop").  The lowerer handles
# arbitrary JUMP/BRANCH targets that the structured lowerer would reject.
# ──────────────────────────────────────────────────────────────────────────────


def _dispatch(
    program: IrProgram,
    function_signatures: list[FunctionSignature] | None = None,
) -> object:
    """Compile with the dispatch-loop strategy and return a WasmModule."""
    sigs = function_signatures or [
        FunctionSignature(label="_start", param_count=0, export_name="_start")
    ]
    return IrToWasmCompiler().compile(program, sigs, strategy="dispatch_loop")


def _dispatch_run(
    program: IrProgram,
    function_signatures: list[FunctionSignature] | None = None,
    *,
    host: object = None,
) -> list[int | float]:
    module = _dispatch(program, function_signatures)
    return _runtime_result(module, "_start", [], host=host)


class TestDispatchLoopLowerer:
    """Verify the dispatch-loop strategy handles arbitrary control flow."""

    def test_simple_halt_exits(self) -> None:
        """A single segment with HALT should exit cleanly."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [42]

    def test_unconditional_forward_jump(self) -> None:
        """JUMP to a forward label skips the code in between."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("_end")], id=gen.next()))
        # This block must be skipped — if reached it would clobber r1
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_skip")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(99)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_end")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(7)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [7]

    def test_unconditional_backward_jump(self) -> None:
        """JUMP to a backward label implements a loop.

        This program counts from 0 to 3, incrementing r2 each iteration:
            _start: r2 = 0; r3 = 3
            _loop:  if r2 >= r3, jump _done
                    r2 += 1; jump _loop
            _done:  r1 = r2; HALT
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(3), IrImmediate(3)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_loop")], id=-1))
        # r4 = (r2 >= r3) implemented as r4 = (r3 > r2) = 0 means not-done
        program.add_instruction(
            IrInstruction(IrOp.CMP_GT, [IrRegister(4), IrRegister(3), IrRegister(2)], id=gen.next())
        )
        # BRANCH_Z: if r4 == 0 (r3 not > r2, so r2 >= r3), jump to _done
        program.add_instruction(
            IrInstruction(IrOp.BRANCH_Z, [IrRegister(4), IrLabel("_done")], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(1)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("_loop")], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_done")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [3]

    def test_branch_nz_conditional_jump(self) -> None:
        """BRANCH_NZ jumps when the register is nonzero."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(1)], id=gen.next())
        )
        # r2 is 1 (nonzero) → should jump to _taken
        program.add_instruction(
            IrInstruction(IrOp.BRANCH_NZ, [IrRegister(2), IrLabel("_taken")], id=gen.next())
        )
        # Not taken path
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(0)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_taken")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(99)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [99]

    def test_branch_nz_not_taken(self) -> None:
        """BRANCH_NZ does not jump when the register is zero."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        # r2 is 0 → BRANCH_NZ should NOT fire; fall through to r1 = 55
        program.add_instruction(
            IrInstruction(IrOp.BRANCH_NZ, [IrRegister(2), IrLabel("_taken")], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(55)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_taken")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(99)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [55]

    def test_branch_z_conditional_jump(self) -> None:
        """BRANCH_Z jumps when the register is zero."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        # r2 == 0 → BRANCH_Z should fire
        program.add_instruction(
            IrInstruction(IrOp.BRANCH_Z, [IrRegister(2), IrLabel("_zero_path")], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(11)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_zero_path")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(22)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [22]

    def test_fall_through_between_segments(self) -> None:
        """A segment with no explicit jump falls through to the next segment."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(10)], id=gen.next())
        )
        # _start falls through to _next (no explicit JUMP)
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_next")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(2), IrImmediate(5)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [15]

    def test_unknown_strategy_raises(self) -> None:
        """Passing an unrecognised strategy name raises WasmLoweringError."""
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=IDGenerator().next()))

        with pytest.raises(WasmLoweringError, match="unknown lowering strategy"):
            IrToWasmCompiler().compile(
                program,
                [FunctionSignature(label="_start", param_count=0, export_name="_start")],
                strategy="bad_strategy",
            )

    def test_dispatch_loop_with_syscall_write(self) -> None:
        """SYSCALL fd_write works correctly through the dispatch loop."""
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(4), IrImmediate(72)], id=gen.next())  # 'H'
        )
        program.add_instruction(IrInstruction(IrOp.SYSCALL, [IrImmediate(1), IrRegister(4)], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        output: list[str] = []
        host = WasiHost(config=WasiConfig(stdout=output.append))
        _dispatch_run(
            program,
            [FunctionSignature(label="_start", param_count=0, export_name="_start")],
            host=host,
        )
        assert output == ["H"]

    def test_mixed_jumps_and_fall_throughs(self) -> None:
        """Program with multiple jumps and fall-throughs produces correct result.

        Computes: r2 = 1 + 2 + 3 = 6
            _start:  r2 = 0; jump _a
            _a:      r2 += 1; (fall through to _b)
            _b:      r2 += 2; jump _c
            _skip:   r2 += 100   ← must never be reached
            _c:      r2 += 3; r1 = r2; HALT
        """
        gen = IDGenerator()
        program = IrProgram(entry_label="_start")
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("_a")], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_a")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(1)], id=gen.next())
        )
        # Fall through to _b
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_b")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(2)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.JUMP, [IrLabel("_c")], id=gen.next()))
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_skip")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(100)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_c")], id=-1))
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(2), IrRegister(2), IrImmediate(3)], id=gen.next())
        )
        program.add_instruction(
            IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(2), IrImmediate(0)], id=gen.next())
        )
        program.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

        assert _dispatch_run(program) == [6]


# ---------------------------------------------------------------------------
# Bitwise opcode tests: OR, OR_IMM, XOR, XOR_IMM, NOT
# ---------------------------------------------------------------------------
#
# These tests verify that each of the five new bitwise opcodes compiles to
# correct WASM and produces the expected result at runtime.  Each test builds
# the smallest possible IrProgram: a LABEL, one arithmetic instruction, then
# RET.  The function is exported so _runtime_result() can call it directly.
#
# Notation in comments below uses Python's bitwise operators for clarity.


def _bitwise_fn(op_instr: IrInstruction, param_count: int = 2) -> object:
    """Build a single-instruction function program and compile it to WASM.

    The function is always exported as ``"f"``.  IR registers:
      r1  = scratch / return value
      r2  = first parameter  (arg[0])
      r3  = second parameter (arg[1], when param_count == 2)
    """
    gen = IDGenerator()
    program = IrProgram(entry_label="_fn_f")
    program.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_fn_f")], id=-1))
    program.add_instruction(op_instr)
    program.add_instruction(IrInstruction(IrOp.RET, [], id=gen.next()))
    return IrToWasmCompiler().compile(
        program,
        function_signatures=[
            FunctionSignature(label="_fn_f", param_count=param_count, export_name="f")
        ],
    )


def test_or_register_register() -> None:
    """OR r1, r2, r3  →  r1 = r2 | r3 (register-register form).

    Example: 0b1010 | 0b0101 = 0b1111 = 15
    """
    gen = IDGenerator()
    module = _bitwise_fn(
        IrInstruction(IrOp.OR, [IrRegister(1), IrRegister(2), IrRegister(3)], id=gen.next()),
        param_count=2,
    )
    # 0b1010 | 0b0101 = 0b1111
    assert _runtime_result(module, "f", [0b1010, 0b0101]) == [0b1111]
    # 0xFF00 | 0x00FF = 0xFFFF
    assert _runtime_result(module, "f", [0xFF00, 0x00FF]) == [0xFFFF]


def test_or_imm() -> None:
    """OR_IMM r1, r2, imm  →  r1 = r2 | imm (immediate form).

    Example: 0b1100 | 0b0011 = 0b1111 = 15
    """
    gen = IDGenerator()
    module = _bitwise_fn(
        IrInstruction(
            IrOp.OR_IMM, [IrRegister(1), IrRegister(2), IrImmediate(0b0011)], id=gen.next()
        ),
        param_count=1,
    )
    # 0b1100 | 0b0011 = 0b1111 = 15
    assert _runtime_result(module, "f", [0b1100]) == [0b1111]
    # 0 | 0b0011 = 0b0011 = 3
    assert _runtime_result(module, "f", [0]) == [0b0011]


def test_xor_register_register() -> None:
    """XOR r1, r2, r3  →  r1 = r2 ^ r3 (register-register form).

    XOR is its own inverse: a ^ b ^ b == a.
    Example: 0b1111 ^ 0b0101 = 0b1010 = 10
    """
    gen = IDGenerator()
    module = _bitwise_fn(
        IrInstruction(IrOp.XOR, [IrRegister(1), IrRegister(2), IrRegister(3)], id=gen.next()),
        param_count=2,
    )
    # 0b1111 ^ 0b0101 = 0b1010
    assert _runtime_result(module, "f", [0b1111, 0b0101]) == [0b1010]
    # Any value XOR itself == 0
    assert _runtime_result(module, "f", [42, 42]) == [0]


def test_xor_imm() -> None:
    """XOR_IMM r1, r2, imm  →  r1 = r2 ^ imm (immediate form).

    Example: 0b1010 ^ 0b1111 = 0b0101 = 5
    """
    gen = IDGenerator()
    module = _bitwise_fn(
        IrInstruction(
            IrOp.XOR_IMM, [IrRegister(1), IrRegister(2), IrImmediate(0b1111)], id=gen.next()
        ),
        param_count=1,
    )
    # 0b1010 ^ 0b1111 = 0b0101
    assert _runtime_result(module, "f", [0b1010]) == [0b0101]
    # 0 ^ 0b1111 = 0b1111
    assert _runtime_result(module, "f", [0]) == [0b1111]


def test_not_flips_all_bits() -> None:
    """NOT r1, r2  →  r1 = ~r2 (bitwise complement, 32-bit).

    WASM has no dedicated NOT opcode.  The backend emits ``i32.xor`` with
    the all-ones mask 0xFFFFFFFF, which flips every bit of the i32.

    ~0 as a WASM i32 is 0xFFFFFFFF = -1 in two's complement.
    ~(-1) as a WASM i32 is 0x00000000 = 0.
    """
    gen = IDGenerator()
    module = _bitwise_fn(
        IrInstruction(IrOp.NOT, [IrRegister(1), IrRegister(2)], id=gen.next()),
        param_count=1,
    )
    # WASM i32 uses two's-complement; the runtime returns a Python int.
    # ~0 = 0xFFFFFFFF which as a signed 32-bit int is -1.
    assert _runtime_result(module, "f", [0]) == [-1]
    # ~(-1) = 0x00000000 = 0
    assert _runtime_result(module, "f", [-1]) == [0]


# ---------------------------------------------------------------------------
# Pre-flight validator tests
# ---------------------------------------------------------------------------

def _simple_prog(*instrs: IrInstruction) -> IrProgram:
    """Build a minimal IrProgram from a list of instructions."""
    gen = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=gen.next()))
    for instr in instrs:
        prog.add_instruction(instr)
    return prog


def _imm(v: int) -> IrImmediate:
    return IrImmediate(v)


def _reg(i: int) -> IrRegister:
    return IrRegister(i)


class TestValidateForWasm:
    """Tests for validate_for_wasm() — the pre-flight IR inspector."""

    def test_valid_program_passes(self) -> None:
        """A well-formed program produces no errors."""
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(0)], id=gen.next()),
            IrInstruction(IrOp.SYSCALL, [_imm(1), _reg(0)], id=gen.next()),
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    # ── Rule 1: supported opcodes ────────────────────────────────────────────

    def test_bitwise_opcodes_are_now_supported(self) -> None:
        """OR, OR_IMM, XOR, XOR_IMM, and NOT are implemented in the WASM backend
        as of the bitwise-ops upgrade.  The validator must not reject them."""
        # Build a minimal program with HALT so validation only checks opcode support.
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    # ── Rule 2: constant overflow ────────────────────────────────────────────

    def test_load_imm_overflow_rejected(self) -> None:
        """A 64-bit constant cannot be encoded as a WASM i32."""
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(2**32)], id=gen.next()),
        )
        errors = validate_for_wasm(prog)
        assert len(errors) == 1
        assert "LOAD_IMM" in errors[0]

    def test_load_imm_max_i32_accepted(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(2**31 - 1)], id=gen.next()),
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    def test_load_imm_min_i32_accepted(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(-(2**31))], id=gen.next()),
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    # ── Rule 3: unsupported SYSCALL numbers ─────────────────────────────────

    def test_syscall_1_accepted(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.SYSCALL, [_imm(1), _reg(0)], id=gen.next()),
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    def test_syscall_2_accepted(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.SYSCALL, [_imm(2), _reg(0)], id=gen.next()),
            IrInstruction(IrOp.HALT, [], id=gen.next()),
        )
        assert validate_for_wasm(prog) == []

    def test_syscall_unsupported_number_rejected(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.SYSCALL, [_imm(99), _reg(0)], id=gen.next()),
        )
        errors = validate_for_wasm(prog)
        assert len(errors) == 1
        assert "unsupported SYSCALL" in errors[0]
        assert "99" in errors[0]

    # ── Integration: compiler calls validate first ───────────────────────────

    def test_compile_rejects_oversized_constant_with_preflight_message(self) -> None:
        gen = IDGenerator()
        prog = _simple_prog(
            IrInstruction(IrOp.LOAD_IMM, [_reg(1), _imm(2**40)], id=gen.next()),
        )
        with pytest.raises(WasmLoweringError, match="pre-flight"):
            IrToWasmCompiler().compile(prog)
