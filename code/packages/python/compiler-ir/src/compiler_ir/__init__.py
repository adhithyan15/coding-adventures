"""compiler_ir — General-purpose IR type library for the AOT compiler pipeline.

This package provides the intermediate representation (IR) used by the AOT
native compiler pipeline. The IR is:

  - Linear: no basic blocks, no SSA, no phi nodes
  - Register-based: infinite virtual registers (v0, v1, ...)
  - Target-independent: backends map IR to physical ISA
  - Versioned: ``.version`` directive in text format (v1 = Brainfuck subset)

Quick Start
-----------

::

    from compiler_ir import IrProgram, IrInstruction, IrDataDecl
    from compiler_ir import IrRegister, IrImmediate, IrLabel
    from compiler_ir import IrOp, IDGenerator
    from compiler_ir import print_ir, parse_ir

    # Build a tiny program:
    gen = IDGenerator()
    prog = IrProgram(entry_label="_start")
    prog.add_data(IrDataDecl("tape", 30000, 0))
    prog.add_instruction(IrInstruction(IrOp.HALT, [], id=gen.next()))

    # Print it:
    text = print_ir(prog)

    # Roundtrip parse:
    prog2 = parse_ir(text)

Submodules
----------

- ``opcodes`` — ``IrOp`` enum + ``parse_op()``
- ``types``   — ``IrRegister``, ``IrImmediate``, ``IrLabel``, ``IrInstruction``,
                ``IrDataDecl``, ``IrProgram``, ``IDGenerator``
- ``printer`` — ``print_ir()``
- ``ir_parser`` — ``parse_ir()``, ``IrParseError``
"""

from compiler_ir.ir_parser import IrParseError, parse_ir
from compiler_ir.opcodes import NAME_TO_OP, OP_NAMES, IrOp, parse_op
from compiler_ir.printer import print_ir  # noqa: I001
from compiler_ir.types import (
    IDGenerator,
    IrDataDecl,
    IrFloatImmediate,
    IrImmediate,
    IrInstruction,
    IrLabel,
    IrOperand,
    IrProgram,
    IrRegister,
)

__all__ = [
    # Opcodes
    "IrOp",
    "NAME_TO_OP",
    "OP_NAMES",
    "parse_op",
    # Types
    "IDGenerator",
    "IrDataDecl",
    "IrFloatImmediate",
    "IrImmediate",
    "IrInstruction",
    "IrLabel",
    "IrOperand",
    "IrProgram",
    "IrRegister",
    # Printer
    "print_ir",
    # Parser
    "IrParseError",
    "parse_ir",
]
