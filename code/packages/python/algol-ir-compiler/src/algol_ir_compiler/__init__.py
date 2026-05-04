"""algol-ir-compiler — Lower the Python ALGOL 60 semantic lane to compiler IR

This package is part of the coding-adventures monorepo, a ground-up
implementation of the computing stack from transistors to operating systems.
"""

from algol_ir_compiler.compiler import (
    AlgolIrCompiler,
    CompileError,
    CompileResult,
    IrCompilerLimits,
    ProcedureSignaturePlan,
    compile_algol,
)

__version__ = "0.1.0"

__all__ = [
    "AlgolIrCompiler",
    "CompileError",
    "CompileResult",
    "IrCompilerLimits",
    "ProcedureSignaturePlan",
    "compile_algol",
]
