"""Facade that validates IR then emits Intel 4004 assembly."""

from __future__ import annotations

from compiler_ir import IrProgram

from intel_4004_ir_validator import IrValidationError, IrValidator
from ir_to_intel_4004_compiler.codegen import CodeGenerator


class IrToIntel4004Compiler:
    """Validate a generic IrProgram and emit Intel 4004 assembly text."""

    def __init__(self) -> None:
        self.validator = IrValidator()
        self.codegen = CodeGenerator()

    def compile(self, program: IrProgram) -> str:
        """Validate and compile an IrProgram to Intel 4004 assembly."""

        errors = self.validator.validate(program)
        if errors:
            combined = "\n".join(str(error) for error in errors)
            raise IrValidationError(
                rule="multiple" if len(errors) > 1 else errors[0].rule,
                message=combined,
            )

        return self.codegen.generate(program)


Intel4004Backend = IrToIntel4004Compiler
