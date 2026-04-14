"""ir_optimizer — pass-based IR-to-IR optimizer for the AOT compiler pipeline.

Overview
--------

This package sits between the language compiler (which produces an ``IrProgram``)
and the backend code generator (which emits machine code).  It transforms an
``IrProgram`` into a semantically equivalent but more efficient ``IrProgram``
by running a series of optimization passes.

The pipeline looks like this::

    Source Code
        ↓
    Frontend (e.g. nib-compiler)
        ↓ IrProgram
    ir-optimizer   ← this package
        ↓ IrProgram (optimized)
    Backend (e.g. ir-to-intel-4004-compiler)
        ↓
    Machine Code / ROM

Quick Start
-----------

::

    from compiler_ir import IrProgram
    from ir_optimizer import optimize, IrOptimizer

    # Simplest form — run the standard three-pass pipeline:
    result = optimize(program)
    print(f"Eliminated {result.instructions_eliminated} instructions")

    # Or use the class directly for more control:
    from ir_optimizer.passes import DeadCodeEliminator, ConstantFolder
    optimizer = IrOptimizer([DeadCodeEliminator(), ConstantFolder()])
    result = optimizer.optimize(program)

Exports
-------

``optimize(program, passes=None) -> OptimizationResult``
    Convenience function.  With ``passes=None`` uses the default three-pass
    pipeline.  Pass a list of ``IrPass`` instances to customize.

``IrOptimizer``
    Class that chains passes.  Use ``.default_passes()`` for the standard
    pipeline or ``.no_op()`` for a pass-through.

``OptimizationResult``
    Dataclass returned by ``IrOptimizer.optimize()``.  Contains the optimized
    program plus diagnostic counts.

``IrPass``
    Protocol that every pass must satisfy (``name`` property + ``run()``
    method).

Submodules
----------

- ``protocol``  — ``IrPass`` Protocol definition
- ``optimizer`` — ``IrOptimizer`` and ``OptimizationResult``
- ``passes``    — ``DeadCodeEliminator``, ``ConstantFolder``, ``PeepholeOptimizer``
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from compiler_ir import IrProgram

from ir_optimizer.optimizer import IrOptimizer, OptimizationResult
from ir_optimizer.passes import ConstantFolder, DeadCodeEliminator, PeepholeOptimizer
from ir_optimizer.protocol import IrPass

if TYPE_CHECKING:
    pass


def optimize(
    program: IrProgram,
    passes: list[IrPass] | None = None,
) -> OptimizationResult:
    """Optimize an IR program using the given passes (or the default pipeline).

    This is the simplest entry point for the optimizer.  If you need more
    control (e.g., access to the ``IrOptimizer`` instance or custom passes),
    construct ``IrOptimizer`` directly.

    Args:
        program: The IR program to optimize.
        passes:  Optional list of ``IrPass`` instances.  If ``None`` (the
                 default), uses the standard three-pass pipeline:
                 ``DeadCodeEliminator → ConstantFolder → PeepholeOptimizer``.

    Returns:
        An ``OptimizationResult`` containing the optimized program and
        diagnostic counts.

    Example::

        result = optimize(program)
        optimized_program = result.program
        print(result.instructions_eliminated)
    """
    if passes is None:
        optimizer = IrOptimizer.default_passes()
    else:
        optimizer = IrOptimizer(passes)
    return optimizer.optimize(program)


__all__ = [
    "optimize",
    "IrOptimizer",
    "OptimizationResult",
    "IrPass",
    "DeadCodeEliminator",
    "ConstantFolder",
    "PeepholeOptimizer",
]
