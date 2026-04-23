"""ir_optimizer.passes — all built-in optimization passes.

This module re-exports the three standard passes so callers can import them
from a single location:

::

    from ir_optimizer.passes import (
        DeadCodeEliminator,
        ConstantFolder,
        PeepholeOptimizer,
    )

Each pass implements the ``IrPass`` Protocol (a ``name`` property and a
``run(IrProgram) -> IrProgram`` method).  They can be used individually or
composed via ``IrOptimizer``.

Pass Summary
------------

DeadCodeEliminator
    Removes instructions that follow an unconditional branch (``JUMP``,
    ``RET``, ``HALT``) without an intervening label.

ConstantFolder
    Folds ``LOAD_IMM vN, k`` followed by ``ADD_IMM vN, vN, d`` or
    ``AND_IMM vN, vN, mask`` into a single ``LOAD_IMM vN, result``.

PeepholeOptimizer
    Merges consecutive ``ADD_IMM`` on the same register, removes no-op
    ``AND_IMM vN, vN, 255``, and folds ``LOAD_IMM 0 + ADD_IMM k`` into
    ``LOAD_IMM k``.
"""

from __future__ import annotations

from ir_optimizer.passes.constant_fold import ConstantFolder
from ir_optimizer.passes.dead_code import DeadCodeEliminator
from ir_optimizer.passes.peephole import PeepholeOptimizer

__all__ = [
    "ConstantFolder",
    "DeadCodeEliminator",
    "PeepholeOptimizer",
]
