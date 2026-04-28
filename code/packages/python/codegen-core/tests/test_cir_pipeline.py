"""Tests for CodegenPipeline[list[CIRInstr]] — the JIT/AOT concrete instantiation.

Uses mock backends and optimizers so no real compiler infrastructure is
required.  Covers the compile(), compile_with_stats(), run(), and
optimizer integration paths.
"""

from __future__ import annotations

from typing import Any

from codegen_core import CIRInstr, CodegenPipeline, CodegenResult
from codegen_core.optimizer import cir_optimizer
from codegen_core.optimizer.cir_optimizer import CIROptimizer

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _cir(*ops: str) -> list[CIRInstr]:
    """Build a trivial CIR list with the given op names."""
    return [CIRInstr(op=op, dest=None if "ret" in op else "v", srcs=[], type="any")
            for op in ops]


class _OkBackend:
    """Always compiles to a constant binary."""
    name = "ok"

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        return b"\xde\xad"

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return 42


class _NullBackend:
    """Always returns None from compile (e.g., unsupported instructions)."""
    name = "null"

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        return None

    def run(self, binary: bytes, args: list[Any]) -> Any:  # pragma: no cover
        return None


class _RecordingBackend:
    """Records the CIR it received from compile()."""
    name = "recording"

    def __init__(self) -> None:
        self.received: list[CIRInstr] | None = None

    def compile(self, cir: list[CIRInstr]) -> bytes | None:
        self.received = list(cir)
        return b"\x01"

    def run(self, binary: bytes, args: list[Any]) -> Any:
        return 0


# ---------------------------------------------------------------------------
# Basic compile() path
# ---------------------------------------------------------------------------

class TestCodegenPipelineCompile:

    def test_compile_returns_bytes_on_success(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile(_cir("const_u8", "ret_u8"))
        assert result == b"\xde\xad"

    def test_compile_returns_none_when_backend_declines(self) -> None:
        pipeline = CodegenPipeline(backend=_NullBackend())
        result = pipeline.compile(_cir("const_u8", "ret_u8"))
        assert result is None

    def test_backend_name_accessible(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        assert pipeline.backend_name == "ok"

    def test_run_delegates_to_backend(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        assert pipeline.run(b"\xde\xad", []) == 42


# ---------------------------------------------------------------------------
# compile_with_stats() path
# ---------------------------------------------------------------------------

class TestCodegenPipelineStats:

    def test_returns_codegen_result(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert isinstance(result, CodegenResult)

    def test_success_true_when_binary(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.success

    def test_success_false_when_no_binary(self) -> None:
        pipeline = CodegenPipeline(backend=_NullBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert not result.success

    def test_binary_size_positive_on_success(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.binary_size > 0

    def test_binary_size_zero_on_failure(self) -> None:
        pipeline = CodegenPipeline(backend=_NullBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.binary_size == 0

    def test_compilation_time_captured(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.compilation_time_ns >= 0

    def test_backend_name_in_result(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.backend_name == "ok"

    def test_optimizer_applied_false_without_optimizer(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert not result.optimizer_applied

    def test_optimizer_applied_true_with_optimizer(self) -> None:
        pipeline = CodegenPipeline(backend=_OkBackend(), optimizer=CIROptimizer())
        result = pipeline.compile_with_stats(_cir("ret_void"))
        assert result.optimizer_applied


# ---------------------------------------------------------------------------
# Optimizer integration
# ---------------------------------------------------------------------------

class TestCodegenPipelineOptimizer:

    def test_optimizer_runs_before_backend(self) -> None:
        """A constant-fold optimizer should reduce a 2-literal add to a const."""
        backend = _RecordingBackend()
        pipeline = CodegenPipeline(backend=backend, optimizer=CIROptimizer())
        cir = [
            CIRInstr(op="add_u8", dest="v", srcs=[3, 4], type="u8"),
            CIRInstr(op="ret_u8", dest=None, srcs=["v"], type="u8"),
        ]
        pipeline.compile(cir)
        assert backend.received is not None
        # The add should have been folded to a const.
        assert backend.received[0].op == "const_u8"
        assert backend.received[0].srcs == [7]

    def test_no_optimizer_passes_cir_unchanged(self) -> None:
        backend = _RecordingBackend()
        pipeline = CodegenPipeline(backend=backend)
        cir = [CIRInstr(op="add_u8", dest="v", srcs=[3, 4], type="u8")]
        pipeline.compile(cir)
        # Without optimizer, the add stays as-is.
        assert backend.received is not None
        assert backend.received[0].op == "add_u8"

    def test_module_level_cir_optimizer_run(self) -> None:
        """cir_optimizer.run() folds constants and removes dead code."""
        cir = [
            CIRInstr(op="add_u8", dest="v", srcs=[10, 20], type="u8"),
            CIRInstr(op="ret_u8", dest=None, srcs=["v"], type="u8"),
        ]
        optimized = cir_optimizer.run(cir)
        assert optimized[0].op == "const_u8"
        assert optimized[0].srcs[0] == 30

    def test_dce_removes_unused_dest(self) -> None:
        cir = [
            CIRInstr(op="const_u8", dest="unused", srcs=[99], type="u8"),  # dead
            CIRInstr(op="const_u8", dest="v", srcs=[1], type="u8"),
            CIRInstr(op="ret_u8", dest=None, srcs=["v"], type="u8"),
        ]
        optimized = cir_optimizer.run(cir)
        ops = [i.op for i in optimized]
        assert "const_u8" in ops  # v is live
        # unused should be gone
        dests = [i.dest for i in optimized]
        assert "unused" not in dests
