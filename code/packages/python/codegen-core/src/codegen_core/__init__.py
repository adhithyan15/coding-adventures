"""codegen-core — the universal IR-to-native compilation layer (LANG19).

``codegen-core`` is the single shared package for lowering any typed IR
to a native binary.  Every compilation path in this repository passes
through it:

**JIT path** (``jit-core`` → ``codegen-core``)::

    IIRFunction
      → jit_core.specialise()   → list[CIRInstr]
      → cir_optimizer.run()     → list[CIRInstr]   (constant fold + DCE)
      → Backend.compile()       → bytes

**AOT path** (``aot-core`` → ``codegen-core``)::

    IIRFunction
      → aot_core.aot_specialise() → list[CIRInstr]
      → cir_optimizer.run()        → list[CIRInstr]
      → Backend.compile()          → bytes

**Compiled-language path** (Nib, BF, Algol-60 → ``codegen-core``)::

    IrProgram
      → IrProgramOptimizer.run()  → IrProgram   (DCE + CF + peephole)
      → Backend.compile()         → bytes

Public API
----------
``CIRInstr``
    Typed intermediate instruction shared by the JIT and AOT paths.
    Produced by ``jit_core.specialise`` and ``aot_core.aot_specialise``.

``Backend[IR]`` / ``BackendProtocol``
    Structural protocol for any backend.  Generic over the IR type.
    ``BackendProtocol`` is an alias kept for backwards compatibility with
    callers that imported it from ``jit_core.backend``.

``CodegenPipeline[IR]``
    Composes an optional ``Optimizer[IR]`` with a ``Backend[IR]``.
    Call ``pipeline.compile(ir)`` for the fast path; call
    ``pipeline.compile_with_stats(ir)`` for a ``CodegenResult`` with
    timing and IR snapshot.

``CodegenResult[IR]``
    Return value of ``compile_with_stats()``.  Contains the binary (or
    ``None``), the post-optimization IR, the backend name, and
    compilation time.

``BackendRegistry``
    Name-to-backend mapping.  Register backends at startup; retrieve
    them by name at compilation time.

``CIROptimizer``
    Class-based wrapper around ``cir_optimizer.run()`` — implements the
    ``Optimizer[list[CIRInstr]]`` protocol for use in ``CodegenPipeline``.

``IrProgramOptimizer``
    Wraps ``ir-optimizer``'s ``IrOptimizer`` into
    ``Optimizer[IrProgram]`` for use in ``CodegenPipeline``.
"""

from __future__ import annotations

from codegen_core.backend import Backend, BackendProtocol, CIRBackend
from codegen_core.cir import CIRInstr
from codegen_core.optimizer.cir_optimizer import CIROptimizer
from codegen_core.optimizer.ir_program import IrProgramOptimizer
from codegen_core.pipeline import CodegenPipeline, Optimizer
from codegen_core.registry import BackendRegistry
from codegen_core.result import CodegenResult

__all__ = [
    "Backend",
    "BackendProtocol",
    "BackendRegistry",
    "CIRBackend",
    "CIRInstr",
    "CIROptimizer",
    "CodegenPipeline",
    "CodegenResult",
    "IrProgramOptimizer",
    "Optimizer",
]
