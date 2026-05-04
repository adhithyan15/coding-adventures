"""IR to Intel 4004 compiler package.

LANG20: ``Intel4004CodeGenerator`` implements ``CodeGenerator[IrProgram, str]``
from ``codegen-core``, providing a shared ``validate() / generate()`` interface.

Note on naming: the existing ``CodeGenerator`` class (from ``codegen.py``) is
the *internal* assembly-text generator only (no validation).
``Intel4004CodeGenerator`` (from ``generator.py``) is the *public* LANG20 adapter
that exposes both ``validate()`` and ``generate()`` under the shared protocol.
"""

from intel_4004_ir_validator import IrValidationError, IrValidator

from ir_to_intel_4004_compiler.backend import IrToIntel4004Compiler
from ir_to_intel_4004_compiler.codegen import CodeGenerator
from ir_to_intel_4004_compiler.generator import Intel4004CodeGenerator

Intel4004Backend = IrToIntel4004Compiler


def validate(program: object) -> list[IrValidationError]:
    """Validate an IrProgram against Intel 4004 hardware constraints."""

    return IrValidator().validate(program)  # type: ignore[arg-type]


def generate_asm(program: object) -> str:
    """Translate an IrProgram into Intel 4004 assembly text."""

    return CodeGenerator().generate(program)  # type: ignore[arg-type]


__all__ = [
    "IrValidationError",
    "IrValidator",
    "CodeGenerator",
    "Intel4004CodeGenerator",
    "IrToIntel4004Compiler",
    "Intel4004Backend",
    "validate",
    "generate_asm",
]
