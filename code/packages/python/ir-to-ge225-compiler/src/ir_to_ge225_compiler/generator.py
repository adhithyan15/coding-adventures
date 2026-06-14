"""GE225CodeGenerator — CodeGenerator[IrProgram, CompileResult] adapter (LANG20).

This module adapts the existing ``compile_to_ge225`` / ``validate_for_ge225``
functions to the ``CodeGenerator[IR, Assembly]`` protocol defined in
``codegen-core``.

Pipeline context
----------------

The ``GE225CodeGenerator`` sits at the *codegen boundary*::

    IrProgram
        ↓
    [GE225CodeGenerator.validate()]   — check opcode support, constant range, etc.
    [GE225CodeGenerator.generate()]   — emit GE-225 binary image + metadata
        ↓ CompileResult(binary: bytes, halt_address, data_base, label_map)
        ├─→ ge225-simulator.load_program_bytes(binary)   (simulator pipeline)
        └─→ file.write(binary)                            (AOT pipeline, future)

The generator does **not** run the simulator.  Execution is the concern of
the downstream pipeline (simulator or AOT packager).

``name = "ge225"``
    Short identifier used by ``CodeGeneratorRegistry`` lookups.

Why ``CompileResult`` and not plain ``bytes``?
    ``CompileResult`` carries ``halt_address`` and ``label_map`` alongside
    ``binary``.  The simulator needs ``halt_address`` to detect program
    termination; the debugger needs ``label_map`` for source mapping.
    Wrapping in a dataclass preserves all metadata for downstream consumers.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from compiler_ir import IrProgram

from ir_to_ge225_compiler.codegen import (
    CompileResult,
    compile_to_ge225,
    validate_for_ge225,
)

if TYPE_CHECKING:
    # Import for type annotations only — no runtime dependency on codegen-core.
    # ``GE225CodeGenerator`` is structurally compatible with
    # ``CodeGenerator[IrProgram, CompileResult]`` without explicit inheritance.
    pass


class GE225CodeGenerator:
    """Validate-and-generate adapter for the GE-225 backend.

    Satisfies ``CodeGenerator[IrProgram, CompileResult]`` structurally — no
    inheritance from ``codegen-core`` is required at runtime.

    The GE-225 is a 1960-era General Electric mainframe.  It is an
    accumulator machine with 20-bit words, a fixed data segment, and no
    hardware halt instruction (the backend emits a self-loop stub).

    Attributes
    ----------
    name:
        ``"ge225"`` — used by ``CodeGeneratorRegistry`` for lookup.

    Examples
    --------
    >>> from compiler_ir import IrImmediate, IrInstruction, IrOp, IrProgram, IrRegister
    >>> prog = IrProgram(entry_label="_start")
    >>> load = IrInstruction(IrOp.LOAD_IMM, [IrRegister(0), IrImmediate(1)], id=0)
    >>> prog.add_instruction(load)
    >>> prog.add_instruction(IrInstruction(IrOp.HALT, [], id=1))
    >>> gen = GE225CodeGenerator()
    >>> gen.validate(prog)
    []
    >>> result = gen.generate(prog)
    >>> isinstance(result.binary, bytes)
    True
    """

    name = "ge225"

    def validate(self, ir: IrProgram) -> list[str]:
        """Validate ``ir`` for GE-225 compatibility.

        Checks performed (see ``validate_for_ge225`` for full details):

        - Every opcode is in the V1 GE-225 supported set.
        - Every ``LOAD_IMM`` / ``ADD_IMM`` constant fits in a 20-bit signed
          word (−524 288 to 524 287).
        - Only ``SYSCALL 1`` (print character) is used.
        - ``AND_IMM`` uses only immediate ``1`` (odd-bit test).

        Parameters
        ----------
        ir:
            The ``IrProgram`` to inspect.

        Returns
        -------
        list[str]
            Human-readable error messages.  Empty list = compatible.
        """
        return validate_for_ge225(ir)

    def generate(self, ir: IrProgram) -> CompileResult:
        """Compile ``ir`` to a GE-225 binary image.

        Runs ``validate()`` internally.  Raises ``CodeGenError`` if the
        program fails validation.

        Parameters
        ----------
        ir:
            A validated (or to-be-validated) ``IrProgram``.

        Returns
        -------
        CompileResult
            ``binary`` — packed GE-225 binary (3 bytes / 20-bit word).
            ``halt_address`` — word address of the halt stub.
            ``data_base`` — first data-segment word address.
            ``label_map`` — label name → resolved code address.

        Raises
        ------
        CodeGenError
            If the IR fails validation or references an undefined label.
        """
        return compile_to_ge225(ir)
