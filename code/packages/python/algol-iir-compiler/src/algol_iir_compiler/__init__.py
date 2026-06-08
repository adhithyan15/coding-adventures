"""ALGOL 60 frontend for the generic InterpreterIR pipeline."""

from __future__ import annotations

from algol_iir_compiler.compiler import compile_source, compile_to_iir
from algol_iir_compiler.errors import (
    AlgolIIRCompileError,
    AlgolIIRUnsupportedError,
)
from algol_iir_compiler.vm import AlgolVM

__version__ = "0.1.0"

__all__ = [
    "AlgolIIRCompileError",
    "AlgolIIRUnsupportedError",
    "AlgolVM",
    "__version__",
    "compile_source",
    "compile_to_iir",
]
