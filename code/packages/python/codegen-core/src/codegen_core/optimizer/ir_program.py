"""IrProgramOptimizer — adapts ``ir-optimizer`` to the ``Optimizer`` protocol.

The ``ir-optimizer`` package already provides a well-tested three-pass
pipeline (DeadCodeEliminator → ConstantFolder → PeepholeOptimizer) for
``IrProgram`` objects produced by compiled-language front-ends (Nib,
Brainfuck, Algol-60, …).

This module wraps ``IrOptimizer`` from that package into the
``Optimizer[IrProgram]`` protocol that ``CodegenPipeline`` expects:

::

    Optimizer.run(ir: IrProgram) -> IrProgram

Why a wrapper?
--------------
``IrOptimizer.optimize()`` returns an ``OptimizationResult`` (a dataclass
that bundles the optimized program with diagnostic metadata), not the
plain ``IrProgram`` directly.  ``CodegenPipeline`` needs a plain ``IR``
value from ``Optimizer.run()`` so it can forward it to
``Backend.compile()``.  The wrapper extracts ``result.program`` and
discards the metadata for the fast path; it exposes
``optimize_with_stats()`` for callers that need the full result.

Usage
-----
To create a ``CodegenPipeline[IrProgram]``:

::

    from codegen_core import CodegenPipeline
    from codegen_core.optimizer.ir_program import IrProgramOptimizer
    from my_wasm_backend import WasmBackend

    pipeline = CodegenPipeline(
        backend=WasmBackend(),
        optimizer=IrProgramOptimizer(),          # default 3-pass pipeline
    )

To use a custom set of passes:

::

    from ir_optimizer import IrOptimizer
    from ir_optimizer.passes import ConstantFolder, DeadCodeEliminator

    optimizer = IrProgramOptimizer(
        IrOptimizer([DeadCodeEliminator(), ConstantFolder()])
    )
"""

from __future__ import annotations

from compiler_ir import IrProgram
from ir_optimizer import IrOptimizer, OptimizationResult


class IrProgramOptimizer:
    """Wraps ``IrOptimizer`` as an ``Optimizer[IrProgram]``.

    Parameters
    ----------
    inner:
        An ``IrOptimizer`` instance.  If ``None``, the default three-pass
        pipeline is used: ``DeadCodeEliminator → ConstantFolder →
        PeepholeOptimizer``.
    """

    def __init__(self, inner: IrOptimizer | None = None) -> None:
        self._inner = inner if inner is not None else IrOptimizer.default_passes()

    def run(self, ir: IrProgram) -> IrProgram:
        """Apply optimization passes to ``ir`` and return the optimized program.

        This is the ``Optimizer[IrProgram]`` protocol method.  It discards
        the ``OptimizationResult`` diagnostics and returns the plain
        ``IrProgram`` so ``CodegenPipeline`` can forward it to the backend.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to optimize.

        Returns
        -------
        IrProgram
            Optimized program (new object; input is not mutated).
        """
        return self._inner.optimize(ir).program

    def optimize_with_stats(self, ir: IrProgram) -> OptimizationResult:
        """Apply optimization passes and return the full ``OptimizationResult``.

        Unlike ``run()``, this method preserves the diagnostic metadata
        (passes run, instruction counts before / after).  Use this when you
        need to inspect optimizer behavior for profiling or debugging.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to optimize.

        Returns
        -------
        OptimizationResult
            Dataclass with ``program``, ``passes_run``,
            ``instructions_before``, ``instructions_after``.
        """
        return self._inner.optimize(ir)
