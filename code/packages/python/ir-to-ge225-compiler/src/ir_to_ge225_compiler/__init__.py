"""IR to GE-225 compiler backend.

Translates a target-independent IrProgram into a binary image of GE-225
20-bit machine words ready to load into the GE-225 simulator.

LANG20: ``GE225CodeGenerator`` implements ``CodeGenerator[IrProgram, CompileResult]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.
"""

from ir_to_ge225_compiler.codegen import (
    CodeGenError,
    CompileResult,
    compile_to_ge225,
    validate_for_ge225,
)
from ir_to_ge225_compiler.generator import GE225CodeGenerator

__all__ = [
    "CodeGenError",
    "CompileResult",
    "GE225CodeGenerator",
    "compile_to_ge225",
    "validate_for_ge225",
]
