"""Intel4004CodeGenerator — CodeGenerator[IrProgram, str] adapter (LANG20).

This module adapts the existing ``IrToIntel4004Compiler`` /
``IrValidator`` to the ``CodeGenerator[IR, Assembly]`` protocol defined
in ``codegen-core``.

Note on naming
--------------
This package already contains a ``CodeGenerator`` class (in ``codegen.py``)
that performs *only* the assembly-text generation step (no validation).
That class is an internal implementation detail.

``Intel4004CodeGenerator`` (this module) is the *public* LANG20 protocol
adapter that exposes both ``validate()`` and ``generate()`` under the
shared ``CodeGenerator[IR, Assembly]`` interface.

Pipeline context
----------------

The ``Intel4004CodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [Intel4004CodeGenerator.validate()]   — check hardware constraints
    [Intel4004CodeGenerator.generate()]   — emit Intel 4004 assembly text
        ↓ str  (multi-line assembly text, e.g.  "    ORG 0x000\\n_start:\\n...")
        ├─→ intel-4004-assembler(text) → bytes   (AOT pipeline, future)
        └─→ intel-4004-simulator(text)            (simulator pipeline, future)

``name = "intel4004"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``str`` as the Assembly type?
    The Intel 4004 backend is a pure text assembler — it does not include a
    binary encoder.  The text output is intended for a downstream assembler
    (text → binary) or directly for a text-accepting simulator.

Validation differences from other backends
------------------------------------------
The GE-225, JVM, WASM, and CIL backends expose ``validate_*()`` functions
that return ``list[str]``.  The Intel backends use ``IrValidator`` objects
that return ``list[IrValidationError]``.  This adapter converts by extracting
``error.message`` from each ``IrValidationError``.
"""

from __future__ import annotations

from compiler_ir import IrProgram
from intel_4004_ir_validator import IrValidationError, IrValidator

from ir_to_intel_4004_compiler.backend import IrToIntel4004Compiler


class Intel4004CodeGenerator:
    """Validate-and-generate adapter for the Intel 4004 backend.

    Satisfies ``CodeGenerator[IrProgram, str]`` structurally.

    The Intel 4004 is a 4-bit microprocessor from 1971 — the world's first
    commercially available single-chip CPU.  The backend emits plain text
    assembly targeting the 4004 instruction set.

    Attributes
    ----------
    name:
        ``"intel4004"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> prog.add_instruction(IrInstruction(IrOp.LABEL, [], id=0))
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(2), IrImmediate(5)], id=1)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=2))
    >>> gen = Intel4004CodeGenerator()
    >>> gen.validate(prog)
    []
    >>> asm = gen.generate(prog)
    >>> isinstance(asm, str)
    True
    >>> "ORG" in asm
    True
    """

    name = "intel4004"

    def __init__(self) -> None:
        self._validator = IrValidator()
        self._compiler = IrToIntel4004Compiler()

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for Intel 4004 hardware constraints.

        Checks performed (see ``intel-4004-ir-validator`` for full details):

        - No ``LOAD_WORD`` or ``STORE_WORD`` opcodes (4004 has no 16-bit ops).
        - Total static RAM ≤ 160 bytes (4 chips × 40 bytes).
        - Static call-graph depth ≤ 2 (3-level hardware stack minus _start).
        - Distinct virtual registers ≤ 12.
        - Every ``LOAD_IMM`` immediate fits in u8 (0–255).

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
        each element, keeping the error format consistent with other backends.
        """
        errors: list[IrValidationError] = self._validator.validate(ir)
        return [e.message for e in errors]

    def generate(self, ir: IrProgram) -> str:
        """Compile ``ir`` to Intel 4004 assembly text.

        Runs ``validate()`` internally (via ``IrToIntel4004Compiler.compile()``).
        Raises ``IrValidationError`` if the program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        str
            Multi-line Intel 4004 assembly text.  Starts with
            ``    ORG 0x000`` followed by one instruction per line.
            Labels are at column 0; instructions are indented 4 spaces.

        Raises
        ------
        IrValidationError
            If the IR fails Intel 4004 hardware-constraint validation.
        """
        return self._compiler.compile(ir)
