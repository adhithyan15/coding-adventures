"""Backwards-compatibility re-export of the CIR optimizer from codegen-core.

The constant-folding + DCE optimizer that was originally defined here in
``jit_core.optimizer`` has been moved to
``codegen_core.optimizer.cir_optimizer`` (LANG19) so that ``aot-core``
can import the same implementation without a backwards dependency on the
JIT-specific package.

This module re-exports the public ``run()`` function so existing callers
of ``from jit_core import optimizer; optimizer.run(cir)`` continue to
work without any changes.

New code should import from ``codegen_core.optimizer`` directly:

    from codegen_core.optimizer import cir_optimizer
    optimized = cir_optimizer.run(cir)

Or, to use the class-based wrapper in a ``CodegenPipeline``:

    from codegen_core import CIROptimizer, CodegenPipeline
    pipeline = CodegenPipeline(backend=b, optimizer=CIROptimizer())
"""

from codegen_core.optimizer.cir_optimizer import (
    CIROptimizer,
    _constant_fold,
    _dead_code_eliminate,
    _infer_literal_type,
    _try_fold,
    run,
)

__all__ = [
    "CIROptimizer",
    "_constant_fold",
    "_dead_code_eliminate",
    "_infer_literal_type",
    "_try_fold",
    "run",
]
