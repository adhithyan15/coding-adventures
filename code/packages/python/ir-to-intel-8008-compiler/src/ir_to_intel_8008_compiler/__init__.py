"""IR to Intel 8008 compiler package.

Public API::

    from ir_to_intel_8008_compiler import (
        CodeGenerator,
        IrToIntel8008Compiler,
        Intel8008Backend,
        IrValidationError,
        IrValidator,
        validate,
        generate_asm,
    )
"""

from intel_8008_ir_validator import IrValidationError, IrValidator

from ir_to_intel_8008_compiler.backend import (
    Intel8008Backend,
    IrToIntel8008Compiler,
)
from ir_to_intel_8008_compiler.codegen import CodeGenerator


def validate(program: object) -> list[IrValidationError]:
    """Validate an IrProgram against Intel 8008 hardware constraints.

    Args:
        program: An ``IrProgram`` to validate.

    Returns:
        A list of ``IrValidationError`` objects; empty if program is valid.
    """
    return IrValidator().validate(program)  # type: ignore[arg-type]


def generate_asm(program: object) -> str:
    """Translate an IrProgram into Intel 8008 assembly text.

    Does NOT run validation first.  Call ``validate()`` separately or use
    ``IrToIntel8008Compiler.compile()`` for the combined validate+generate path.

    Args:
        program: A validated ``IrProgram``.

    Returns:
        Multi-line Intel 8008 assembly string.
    """
    return CodeGenerator().generate(program)  # type: ignore[arg-type]


__all__ = [
    "IrValidationError",
    "IrValidator",
    "CodeGenerator",
    "IrToIntel8008Compiler",
    "Intel8008Backend",
    "validate",
    "generate_asm",
]
