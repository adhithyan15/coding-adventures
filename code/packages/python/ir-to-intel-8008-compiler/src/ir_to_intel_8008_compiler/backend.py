"""IrToIntel8008Compiler — validate-then-generate facade for the 8008 backend.

This module provides the single entry point for the Intel 8008 code generation
pipeline:

  1. Validate the IrProgram against 8008 hardware constraints
     (``intel_8008_ir_validator.IrValidator``).
  2. If validation passes, emit Intel 8008 assembly text
     (``ir_to_intel_8008_compiler.codegen.CodeGenerator``).
  3. If validation fails, raise the combined error message as a single
     ``IrValidationError`` so callers get a clear, one-exception result.

Usage::

    from compiler_ir import IrProgram
    from ir_to_intel_8008_compiler import IrToIntel8008Compiler

    compiler = IrToIntel8008Compiler()
    asm = compiler.compile(program)   # raises IrValidationError on bad IR
    print(asm)
"""

from __future__ import annotations

from compiler_ir import IrProgram
from intel_8008_ir_validator import IrValidationError, IrValidator

from ir_to_intel_8008_compiler.codegen import CodeGenerator


class IrToIntel8008Compiler:
    """Validate an IrProgram and emit Intel 8008 assembly text.

    The compiler wraps the validator and code generator in a single
    ``compile()`` call.  Validation errors are surfaced as a single
    ``IrValidationError`` whose message concatenates all individual
    violations, making it easy to surface everything to the user at once.

    Attributes:
        validator: The ``IrValidator`` instance (can be replaced for testing).
        codegen:   The ``CodeGenerator`` instance.
    """

    def __init__(self) -> None:
        self.validator = IrValidator()
        self.codegen = CodeGenerator()

    def compile(self, program: IrProgram) -> str:
        """Validate and compile an IrProgram to Intel 8008 assembly text.

        Args:
            program: The ``IrProgram`` to compile.  Must have been produced
                     by ``oct-ir-compiler`` (or equivalent) and must not
                     contain LOAD_WORD/STORE_WORD or out-of-range SYSCALLs.

        Returns:
            A multi-line string of Intel 8008 assembly text, starting with
            ``ORG 0x0000`` and ending with a newline.

        Raises:
            IrValidationError: If any hardware-constraint check fails.
                The error message lists all violations, separated by newlines.
        """
        errors = self.validator.validate(program)
        if errors:
            combined = "\n".join(str(e) for e in errors)
            raise IrValidationError(
                rule="multiple" if len(errors) > 1 else errors[0].rule,
                message=combined,
            )

        return self.codegen.generate(program)


# Alias so consumers can import either name.
Intel8008Backend = IrToIntel8008Compiler
