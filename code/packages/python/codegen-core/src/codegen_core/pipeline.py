"""CodegenPipeline — the universal optimize-then-compile pipeline.

``CodegenPipeline[IR]`` is the single place that defines what *code
generation* means in this repository:

    1. Optionally run an ``Optimizer[IR]`` to clean up the IR.
    2. Hand the (possibly optimized) IR to a ``Backend[IR]``.
    3. Return either ``bytes | None`` (the fast path) or a
       ``CodegenResult[IR]`` with full diagnostics (the stats path).

Two concrete instantiations exist in this repo:

``CodegenPipeline[list[CIRInstr]]``
    Used by both ``JITCore`` and ``AOTCore``.  The optimizer is the
    ``cir_optimizer`` module (constant folding + DCE).  The backend is any
    ``CIRBackend`` implementation (``Intel4004Backend``, …).

``CodegenPipeline[IrProgram]``
    Used by compiled-language compilers (Nib, Brainfuck, …).  The optimizer
    wraps the ``ir-optimizer`` package (DCE + constant fold + peephole).
    The backend is any ``Backend[IrProgram]`` implementation.

Optimizer protocol
------------------
The ``Optimizer`` protocol requires only a single ``run(ir: IR) -> IR``
method.  Any callable object that satisfies this shape works — no
inheritance required.  Passing ``optimizer=None`` skips optimization
entirely (``optimizer_applied`` is ``False`` in the result).

Example: JIT pipeline
---------------------
::

    from codegen_core import CodegenPipeline, CIRInstr
    from codegen_core.optimizer import cir_optimizer
    from intel4004_backend import Intel4004Backend

    pipeline: CodegenPipeline[list[CIRInstr]] = CodegenPipeline(
        backend=Intel4004Backend(),
        optimizer=cir_optimizer,
    )

    binary = pipeline.compile(cir_list)
    result = pipeline.compile_with_stats(cir_list)

Example: Compiled-language pipeline
------------------------------------
::

    from codegen_core import CodegenPipeline
    from codegen_core.optimizer.ir_program import IrProgramOptimizer
    from compiler_ir import IrProgram
    from my_wasm_backend import WasmBackend

    pipeline: CodegenPipeline[IrProgram] = CodegenPipeline(
        backend=WasmBackend(),
        optimizer=IrProgramOptimizer(),
    )
"""

from __future__ import annotations

import time
from typing import Any, Protocol, TypeVar

from codegen_core.backend import Backend
from codegen_core.result import CodegenResult

IR = TypeVar("IR")


class Optimizer(Protocol[IR]):
    """Protocol for optimizer passes that transform an IR value.

    Only one method is required: ``run(ir: IR) -> IR``.  The method must
    be pure — it should return a new IR value rather than mutating the
    input.

    ``jit_core.optimizer`` (constant folding + DCE for CIR) and
    ``ir-optimizer``'s ``IrOptimizer`` both satisfy this protocol.
    """

    def run(self, ir: IR) -> IR:
        """Apply optimization passes to ``ir`` and return the result.

        Parameters
        ----------
        ir:
            The typed IR to optimize.

        Returns
        -------
        IR
            A new (possibly reduced) IR value.  Must be the same type as
            the input so it can be forwarded to ``Backend.compile()``.
        """
        ...


class CodegenPipeline[IR]:
    """Universal IR → native binary pipeline.

    ``CodegenPipeline`` composes an optional optimizer with a backend:

    ::

        IR → [optimizer.run(IR)] → backend.compile(IR) → bytes | None

    Type parameters
    ---------------
    IR
        The IR type flowing through this pipeline.

    Parameters
    ----------
    backend:
        A ``Backend[IR]`` implementation.  Called last; its ``compile()``
        method receives the (possibly optimized) IR.
    optimizer:
        An ``Optimizer[IR]`` implementation, or ``None`` to skip
        optimization.  Any object with a ``run(ir: IR) -> IR`` method
        satisfies the protocol.

    Thread safety
    -------------
    Not thread-safe.  Each concurrent compilation should use its own
    ``CodegenPipeline`` instance.
    """

    def __init__(
        self,
        backend: Backend[IR],
        optimizer: Optimizer[IR] | None = None,
    ) -> None:
        self._backend = backend
        self._optimizer = optimizer

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    @property
    def backend_name(self) -> str:
        """Short identifier of the backend used by this pipeline."""
        return self._backend.name

    def compile(self, ir: IR) -> bytes | None:
        """Compile ``ir`` to a native binary.

        This is the fast path — no timing or IR snapshot is taken.

        Parameters
        ----------
        ir:
            The typed IR to compile.

        Returns
        -------
        bytes
            Opaque native binary ready for ``Backend.run()``.
        None
            If the backend declined to compile the IR (e.g., unsupported
            instructions).
        """
        optimized = self._run_optimizer(ir)
        return self._backend.compile(optimized)

    def compile_with_stats(self, ir: IR) -> CodegenResult[IR]:
        """Compile ``ir`` and return a ``CodegenResult`` with diagnostics.

        Unlike the plain ``compile()`` method, this path:
        - Captures wall-clock compilation time in nanoseconds.
        - Stores the post-optimization IR snapshot in the result.
        - Records whether an optimizer was applied.

        Use this path when the caller needs to store or display the
        post-optimization IR (e.g., for a JIT cache entry or an IR dump
        tool).

        Parameters
        ----------
        ir:
            The typed IR to compile.

        Returns
        -------
        CodegenResult[IR]
            Full result including binary (or None), IR snapshot, and
            diagnostics.
        """
        t0 = time.monotonic_ns()
        optimized = self._run_optimizer(ir)
        binary = self._backend.compile(optimized)
        t1 = time.monotonic_ns()

        return CodegenResult(
            binary=binary,
            ir=optimized,
            backend_name=self._backend.name,
            compilation_time_ns=t1 - t0,
            optimizer_applied=self._optimizer is not None,
        )

    def run(self, binary: bytes, args: list[Any]) -> Any:
        """Execute a previously compiled binary via the backend.

        Convenience pass-through so callers don't need to keep a
        reference to the backend separately.

        Parameters
        ----------
        binary:
            The bytes returned by a prior ``compile()`` call.
        args:
            Positional arguments in calling-convention order.

        Returns
        -------
        Any
            Return value, or ``None`` for void functions.
        """
        return self._backend.run(binary, args)

    # ------------------------------------------------------------------
    # Internal helper
    # ------------------------------------------------------------------

    def _run_optimizer(self, ir: IR) -> IR:
        """Apply the optimizer if one is present, otherwise return ``ir`` unchanged."""
        if self._optimizer is None:
            return ir
        return self._optimizer.run(ir)
