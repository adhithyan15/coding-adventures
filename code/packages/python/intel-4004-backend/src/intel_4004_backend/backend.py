"""Intel4004Backend — combines IrValidator and CodeGenerator into a single entry point.

Overview
--------

The ``Intel4004Backend`` class is the top-level interface for the Intel 4004
code generation pipeline.  It combines the two phases:

1. **Validation** — ``IrValidator`` checks that the program fits within the
   Intel 4004 hardware constraints (RAM ≤ 160 bytes, call depth ≤ 2, etc.).

2. **Code generation** — ``CodeGenerator`` translates the validated
   ``IrProgram`` into Intel 4004 assembly text.

If validation fails, the backend raises ``IrValidationError`` with a message
listing all constraint violations.  This "fail fast" design means you never
get partially-generated assembly for an infeasible program.

Design Pattern
--------------

This class follows the **façade pattern** — it hides the two-step process
behind a single ``compile()`` method.  Callers who need fine-grained control
can use ``IrValidator`` and ``CodeGenerator`` directly.

Usage
-----

::

    from compiler_ir import IrProgram, IrInstruction, IrOp, IrImmediate
    from compiler_ir import IrRegister, IrLabel, IDGenerator
    from intel_4004_backend import Intel4004Backend, IrValidationError

    gen = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL, [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=gen.next())
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    backend = Intel4004Backend()
    try:
        asm = backend.compile(prog)
        print(asm)
    except IrValidationError as e:
        print(f"Hardware constraint violated: {e}")
"""

from __future__ import annotations

from compiler_ir import IrProgram

from intel_4004_backend.codegen import CodeGenerator
from intel_4004_backend.validator import IrValidationError, IrValidator


class Intel4004Backend:
    """Two-phase backend: validate then generate Intel 4004 assembly.

    Phase 1 — ``IrValidator`` checks hardware constraints.
    Phase 2 — ``CodeGenerator`` emits assembly text.

    Raises ``IrValidationError`` if the program violates any constraint.

    Attributes:
        validator:  The ``IrValidator`` instance used for Phase 1.
        codegen:    The ``CodeGenerator`` instance used for Phase 2.

    Example::

        backend = Intel4004Backend()
        asm = backend.compile(prog)  # raises IrValidationError on failure
    """

    def __init__(self) -> None:
        """Create a new Intel4004Backend with default validator and codegen."""
        self.validator = IrValidator()
        self.codegen = CodeGenerator()

    def compile(self, program: IrProgram) -> str:
        """Validate and compile an IrProgram to Intel 4004 assembly.

        Runs the validator first.  If any hardware constraints are violated,
        raises ``IrValidationError`` with a combined message listing all
        violations.  Only proceeds to code generation if the program is clean.

        Args:
            program: The ``IrProgram`` to compile.

        Returns:
            A string containing Intel 4004 assembly text.

        Raises:
            IrValidationError: If one or more hardware constraints are violated.

        Example::

            backend = Intel4004Backend()
            asm = backend.compile(prog)
        """
        errors = self.validator.validate(program)
        if errors:
            # Combine all error messages into one exception so the programmer
            # sees the full picture at once — like a compiler showing all
            # errors rather than stopping at the first one.
            combined = "\n".join(str(e) for e in errors)
            raise IrValidationError(
                rule="multiple" if len(errors) > 1 else errors[0].rule,
                message=combined,
            )

        return self.codegen.generate(program)
