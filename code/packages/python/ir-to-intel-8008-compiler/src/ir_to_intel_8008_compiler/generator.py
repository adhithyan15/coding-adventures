"""Intel8008CodeGenerator — CodeGenerator[IrProgram, str] adapter (LANG20).

This module adapts the existing ``IrToIntel8008Compiler`` /
``IrValidator`` to the ``CodeGenerator[IR, Assembly]`` protocol defined
in ``codegen-core``.

Note on naming
--------------
This package already contains a ``CodeGenerator`` class (in ``codegen.py``)
that performs *only* the assembly-text generation step (no validation).
That class is an internal implementation detail.

``Intel8008CodeGenerator`` (this module) is the *public* LANG20 protocol
adapter that exposes both ``validate()`` and ``generate()`` under the
shared ``CodeGenerator[IR, Assembly]`` interface.

Pipeline context
----------------

The ``Intel8008CodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [Intel8008CodeGenerator.validate()]   — check hardware constraints
    [Intel8008CodeGenerator.generate()]   — emit Intel 8008 assembly text
        ↓ str  (multi-line assembly, e.g. "    ORG 0x0000\\n_start:\\n...")
        ├─→ intel-8008-assembler(text) → bytes   (AOT pipeline, future)
        └─→ intel-8008-simulator(text)            (simulator pipeline, future)

``name = "intel8008"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``str`` as the Assembly type?
    Same reason as the Intel 4004 backend: the 8008 backend is a pure text
    assembler.  A downstream binary assembler converts text → bytes.

Validation differences from other backends
------------------------------------------
The GE-225, JVM, WASM, and CIL backends expose ``validate_*()`` functions
that return ``list[str]``.  The Intel backends use ``IrValidator`` objects
that return ``list[IrValidationError]``.  This adapter converts by extracting
``error.message`` from each ``IrValidationError``.
"""

from __future__ import annotations

from compiler_ir import IrProgram
from intel_8008_ir_validator import IrValidationError, IrValidator

from ir_to_intel_8008_compiler.backend import IrToIntel8008Compiler


class Intel8008CodeGenerator:
    """Validate-and-generate adapter for the Intel 8008 backend.

    Satisfies ``CodeGenerator[IrProgram, str]`` structurally.

    The Intel 8008 is an 8-bit microprocessor from 1972 — the world's first
    single-chip 8-bit CPU.  The backend emits plain text assembly targeting
    the 8008 instruction set.

    Attributes
    ----------
    name:
        ``"intel8008"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> prog.add_instruction(IrInstruction(IrOp.LABEL, [], id=0))
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(1), IrImmediate(42)], id=1)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=2))
    >>> gen = Intel8008CodeGenerator()
    >>> gen.validate(prog)
    []
    >>> asm = gen.generate(prog)
    >>> isinstance(asm, str)
    True
    >>> "ORG" in asm
    True
    """

    name = "intel8008"

    def __init__(self) -> None:
        self._validator = IrValidator()
        self._compiler = IrToIntel8008Compiler()

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for Intel 8008 hardware constraints.

        Checks performed (see ``intel-8008-ir-validator`` for full details):

        - Opcode support for the 8008 instruction set.
        - Virtual register count within the 8008 physical register limit.
        - Immediate value ranges and syscall number restrictions.

        Parameters
        ----------
        ir:
            The ``IrProgram`` to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  Empty list = compatible.

        Notes
        -----
        ``IrValidator.validate()`` returns ``list[IrValidationError]``.  This
        adapter converts to ``list[str]`` by extracting ``error.message`` from
        each element, consistent with other backends.
        """
        errors: list[IrValidationError] = self._validator.validate(ir)
        return [e.message for e in errors]

    def generate(self, ir: IrProgram) -> str:
        """Compile ``ir`` to Intel 8008 assembly text.

        Runs ``validate()`` internally (via ``IrToIntel8008Compiler.compile()``).
        Raises ``IrValidationError`` if the program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        str
            Multi-line Intel 8008 assembly text.  Starts with
            ``    ORG 0x0000`` followed by one instruction per line.
            Labels are at column 0; instructions are indented 4 spaces.

        Raises
        ------
        IrValidationError
            If the IR fails Intel 8008 hardware-constraint validation.
        """
        return self._compiler.compile(ir)
