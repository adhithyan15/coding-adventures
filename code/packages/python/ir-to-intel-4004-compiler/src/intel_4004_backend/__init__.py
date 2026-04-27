"""Compatibility wrapper for the old intel_4004_backend module path."""

from ir_to_intel_4004_compiler import (
    CodeGenerator,
    Intel4004Backend,
    IrToIntel4004Compiler,
    IrValidationError,
    IrValidator,
    generate_asm,
    validate,
)

__all__ = [
    "IrValidationError",
    "IrValidator",
    "CodeGenerator",
    "IrToIntel4004Compiler",
    "Intel4004Backend",
    "validate",
    "generate_asm",
]
