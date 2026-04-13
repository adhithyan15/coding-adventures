"""intel_4004_backend — Two-phase backend: IR validation + 4004 assembly generation.

Overview
--------

This package is PR 9 in the Nib language → Intel 4004 compiler pipeline.
It sits between the frontend (which produces an ``IrProgram``) and the
assembler (which turns assembly text into object code).

The pipeline has two phases:

Phase 1 — **IrValidator**: Checks that the ``IrProgram`` can actually run on
real Intel 4004 hardware.  These are **not** language errors — the type
checker already caught those.  These are *backend feasibility checks*: ISA
limits that no amount of frontend cleverness can work around.

Phase 2 — **CodeGenerator**: Translates the validated ``IrProgram`` into
Intel 4004 assembly text.  The output is a string you can feed directly to
an assembler.

Quick Start
-----------

::

    from compiler_ir import IrProgram, IrInstruction, IrOp, IrImmediate
    from compiler_ir import IrRegister, IrLabel, IDGenerator
    from intel_4004_backend import Intel4004Backend, IrValidationError

    gen = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_instruction(IrInstruction(IrOp.LABEL,    [IrLabel("_start")], id=-1))
    prog.add_instruction(
        IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=gen.next())
    )
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    backend = Intel4004Backend()
    asm = backend.compile(prog)   # raises IrValidationError on failure
    print(asm)

Exports
-------

- ``IrValidationError``  — raised when a validation rule is violated
- ``validate``           — standalone function: validate(prog) → list[IrValidationError]
- ``generate_asm``       — standalone function: generate_asm(prog) → str
- ``IrValidator``        — class with validate(prog) method
- ``CodeGenerator``      — class with generate(prog) method
- ``Intel4004Backend``   — convenience wrapper: compile(prog) → str, raises on error

Submodules
----------

- ``validator`` — ``IrValidator`` class and ``IrValidationError``
- ``codegen``   — ``CodeGenerator`` class
- ``backend``   — ``Intel4004Backend`` class
"""

from intel_4004_backend.backend import Intel4004Backend
from intel_4004_backend.codegen import CodeGenerator
from intel_4004_backend.validator import IrValidationError, IrValidator


def validate(program: object) -> list[IrValidationError]:
    """Validate an IrProgram against Intel 4004 hardware constraints.

    This is a convenience wrapper around ``IrValidator.validate()``.

    Args:
        program: The ``IrProgram`` to validate.

    Returns:
        A list of ``IrValidationError`` objects.  Empty list means the
        program is feasible on Intel 4004 hardware.

    Example::

        errors = validate(prog)
        if errors:
            for e in errors:
                print(e)
    """
    return IrValidator().validate(program)  # type: ignore[arg-type]


def generate_asm(program: object) -> str:
    """Translate an IrProgram into Intel 4004 assembly text.

    This is a convenience wrapper around ``CodeGenerator.generate()``.
    Does NOT run the validator first — call ``validate()`` separately if
    you need to check feasibility before generating.

    Args:
        program: A validated ``IrProgram``.

    Returns:
        A string containing Intel 4004 assembly text, suitable for feeding
        to an assembler.

    Example::

        asm = generate_asm(prog)
        with open("out.asm", "w") as f:
            f.write(asm)
    """
    return CodeGenerator().generate(program)  # type: ignore[arg-type]


__all__ = [
    "IrValidationError",
    "IrValidator",
    "CodeGenerator",
    "Intel4004Backend",
    "validate",
    "generate_asm",
]
