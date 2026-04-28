"""Tests for CodegenPipeline[IrProgram] and IrProgramOptimizer.

Exercises the compiled-language pipeline path:

    IrProgram → IrProgramOptimizer.run() → Backend[IrProgram].compile() → bytes

Uses a minimal IrProgram constructed from compiler-ir primitives.
"""

from __future__ import annotations

from typing import Any

from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister

from codegen_core import CodegenPipeline, IrProgramOptimizer

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _noop_program() -> IrProgram:
    """Minimal IrProgram: LOAD_IMM v0, 1; RET."""
    p = IrProgram(entry_label="main")
    p.add_instruction(IrInstruction(
        opcode=IrOp.LABEL,
        operands=[IrRegister(0)],  # label marker — use register as proxy
        id=0,
    ))
    p.add_instruction(IrInstruction(
        opcode=IrOp.LOAD_IMM,
        operands=[IrRegister(0), IrImmediate(1)],
        id=1,
    ))
    p.add_instruction(IrInstruction(
        opcode=IrOp.RET,
        operands=[IrRegister(0)],
        id=2,
    ))
    return p


class _IrOkBackend:
    """Backend[IrProgram] that always compiles to a constant binary."""
    name = "ir-ok"

    def compile(self, ir: IrProgram) -> bytes | None:
        return b"\xbe\xef"

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return 1


class _IrNullBackend:
    """Backend[IrProgram] that always declines to compile."""
    name = "ir-null"

    def compile(self, ir: IrProgram) -> bytes | None:
        return None

    def run(self, binary: bytes, args: list[Any]) -> Any:  # pragma: no cover
        return None


class _IrRecordingBackend:
    """Records what IrProgram it received."""
    name = "ir-recording"

    def __init__(self) -> None:
        self.received: IrProgram | None = None

    def compile(self, ir: IrProgram) -> bytes | None:
        self.received = ir
        return b"\x01"

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return 0


# ---------------------------------------------------------------------------
# IrProgramOptimizer
# ---------------------------------------------------------------------------

class TestIrProgramOptimizer:

    def test_run_returns_ir_program(self) -> None:
        opt = IrProgramOptimizer()
        result = opt.run(_noop_program())
        assert isinstance(result, IrProgram)

    def test_run_preserves_entry_label(self) -> None:
        opt = IrProgramOptimizer()
        prog = _noop_program()
        result = opt.run(prog)
        assert result.entry_label == prog.entry_label

    def test_optimize_with_stats_returns_result(self) -> None:
        from ir_optimizer import OptimizationResult
        opt = IrProgramOptimizer()
        stats = opt.optimize_with_stats(_noop_program())
        assert isinstance(stats, OptimizationResult)

    def test_optimize_with_stats_has_passes_run(self) -> None:
        opt = IrProgramOptimizer()
        stats = opt.optimize_with_stats(_noop_program())
        assert isinstance(stats.passes_run, list)
        assert len(stats.passes_run) > 0

    def test_custom_inner_optimizer(self) -> None:
        from ir_optimizer import IrOptimizer
        inner = IrOptimizer.no_op()
        opt = IrProgramOptimizer(inner)
        result = opt.run(_noop_program())
        assert isinstance(result, IrProgram)


# ---------------------------------------------------------------------------
# CodegenPipeline[IrProgram]
# ---------------------------------------------------------------------------

class TestIrProgramPipeline:

    def test_compile_returns_bytes(self) -> None:
        pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
            backend=_IrOkBackend()
        )
        assert pipeline.compile(_noop_program()) == b"\xbe\xef"

    def test_compile_returns_none_when_backend_declines(self) -> None:
        pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
            backend=_IrNullBackend()
        )
        assert pipeline.compile(_noop_program()) is None

    def test_optimizer_runs_before_backend(self) -> None:
        backend = _IrRecordingBackend()
        opt = IrProgramOptimizer()
        pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
            backend=backend, optimizer=opt
        )
        pipeline.compile(_noop_program())
        # Backend should have received an IrProgram (the optimized one).
        assert isinstance(backend.received, IrProgram)

    def test_compile_with_stats_optimizer_applied(self) -> None:
        pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
            backend=_IrOkBackend(), optimizer=IrProgramOptimizer()
        )
        result = pipeline.compile_with_stats(_noop_program())
        assert result.optimizer_applied

    def test_compile_with_stats_backend_name(self) -> None:
        pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
            backend=_IrOkBackend()
        )
        result = pipeline.compile_with_stats(_noop_program())
        assert result.backend_name == "ir-ok"
