"""codegen_core.optimizer — optimizer adapters for both IR families.

Two sub-modules handle the two IR types that flow through this repo:

``cir_optimizer``
    Constant folding + DCE for ``list[CIRInstr]`` (JIT/AOT path).
    Module-level ``run(cir)`` is the entry point; ``CIROptimizer`` wraps
    it into the ``Optimizer`` protocol for use in ``CodegenPipeline``.

``ir_program``
    Wraps the ``ir-optimizer`` package for ``IrProgram`` (compiled-language
    path).  ``IrProgramOptimizer`` implements the ``Optimizer[IrProgram]``
    protocol.

Most callers import the sub-module they need directly:

    from codegen_core.optimizer import cir_optimizer
    optimized = cir_optimizer.run(cir)

    from codegen_core.optimizer.ir_program import IrProgramOptimizer
"""

from codegen_core.optimizer import cir_optimizer
from codegen_core.optimizer.ir_program import IrProgramOptimizer

__all__ = [
    "IrProgramOptimizer",
    "cir_optimizer",
]
