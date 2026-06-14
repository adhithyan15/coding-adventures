"""IR to Intel 8008 compiler package.

LANG20: ``Intel8008CodeGenerator`` implements ``CodeGenerator[IrProgram, str]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.

Note on naming: the existing ``CodeGenerator`` class (from ``codegen.py``) is
the *internal* assembly-text generator only (no validation).
``Intel8008CodeGenerator`` (from ``generator.py``) is the *public* LANG20 adapter
that exposes both ``validate()`` and ``generate()`` under the shared protocol.

Public API::

    from ir_to_intel_8008_compiler import (
        CodeGenerator,
        Intel8008CodeGenerator,
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
from ir_to_intel_8008_compiler.generator import Intel8008CodeGenerator


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
    "Intel8008CodeGenerator",
    "IrToIntel8008Compiler",
    "Intel8008Backend",
    "validate",
    "generate_asm",
]
