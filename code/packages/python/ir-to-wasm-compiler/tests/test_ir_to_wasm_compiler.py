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
    validate_for_wasm,
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
    # The V1 WASM backend supports all IrOp values *except* the five bitwise
    # opcodes added in compiler-ir v0.3.0 (OR, OR_IMM, XOR, XOR_IMM, NOT).
    # Those are deferred to V2 — WASM i32.or / i32.xor are easy to add once a
    # frontend actually needs them.  The validator must reject them cleanly.

    def test_bitwise_opcodes_are_intentionally_unsupported(self) -> None:
        """OR, OR_IMM, XOR, XOR_IMM, and NOT are not yet implemented in the V1
        WASM backend.  Each must produce an 'unsupported opcode' diagnostic."""
        unsupported = (IrOp.OR, IrOp.OR_IMM, IrOp.XOR, IrOp.XOR_IMM, IrOp.NOT)
        for op in unsupported:
            prog = _simple_prog(IrInstruction(op, [], id=1))
            errors = validate_for_wasm(prog)
            rule1_errors = [e for e in errors if "unsupported opcode" in e]
            assert rule1_errors != [], (
                f"IrOp.{op.name} should be rejected by the WASM opcode-support "
                f"check but was accepted"
            )
            assert op.name in rule1_errors[0], (
                f"Error for IrOp.{op.name} does not mention the opcode name: "
                f"{rule1_errors[0]!r}"
            )

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
