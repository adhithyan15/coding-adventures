"""IR Types — operands, instructions, data declarations, and programs.

Overview
--------

Every IR instruction operates on **operands**. There are four kinds:

  IrRegister  — a virtual register (v0, v1, v2, ...)
  IrImmediate — a literal integer value
  IrFloatImmediate — a literal floating-point value
  IrLabel     — a named jump target or data label

These three operand types are combined into ``IrInstruction`` objects, which
live inside an ``IrProgram``.

The type hierarchy::

  IrOperand (base type — IrRegister | IrImmediate | IrFloatImmediate | IrLabel)
  IrInstruction (opcode + operands + unique ID)
  IrDataDecl (named data segment: label, size, init byte)
  IrProgram (full program: instructions + data + entry label + version)
  IDGenerator (produces unique monotonic instruction IDs)

Virtual Registers
-----------------

Virtual registers are named v0, v1, v2, ... (using the Index field). There
are infinitely many — the backend's register allocator maps them to physical
registers. Brainfuck needs only 7 (v0–v6); a general-purpose language like
BASIC would need more.

Instruction IDs
---------------

Each instruction carries a unique integer ID (the ID field). This ID is the
key that connects IR instructions to the source map chain. The IDGenerator
ensures no two instructions ever share an ID, even across multiple compiler
invocations within the same process.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from compiler_ir.opcodes import IrOp

# ---------------------------------------------------------------------------
# IrOperand — union type for all operand kinds
# ---------------------------------------------------------------------------
#
# Python does not have sealed interfaces like Go, so we use a Union type.
# The concrete operand types are IrRegister, IrImmediate, IrFloatImmediate,
# and IrLabel.
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class IrRegister:
    """A virtual register operand.

    Virtual registers are named v0, v1, v2, ... (using ``index``).
    There are infinitely many; the backend maps them to physical registers.

    Attributes:
        index: The register index (non-negative integer).

    Example::

        r = IrRegister(index=3)
        str(r)    # "v3"
        r.index   # 3
    """

    index: int

    def __str__(self) -> str:
        """Return the canonical text form: ``v<index>``."""
        return f"v{self.index}"


@dataclass(frozen=True)
class IrImmediate:
    """A literal integer value operand.

    Immediates are signed integers that appear directly in instructions.

    Attributes:
        value: The integer value (signed).

    Example::

        imm = IrImmediate(value=42)
        str(imm)   # "42"

        neg = IrImmediate(value=-1)
        str(neg)   # "-1"
    """

    value: int

    def __str__(self) -> str:
        """Return the canonical text form: the integer as a string."""
        return str(self.value)


@dataclass(frozen=True)
class IrFloatImmediate:
    """A literal floating-point value operand.

    Floating immediates are IEEE-754 double-precision values that appear
    directly in instructions.

    Attributes:
        value: The floating-point value.
    """

    value: float

    def __str__(self) -> str:
        """Return the canonical text form: the float as a string."""
        return str(self.value)


@dataclass(frozen=True)
class IrLabel:
    """A named jump target or data reference operand.

    Labels resolve to addresses during code generation. They are strings
    like ``"loop_0_start"``, ``"_start"``, ``"tape"``, ``"__trap_oob"``.

    Attributes:
        name: The label name (a valid identifier string).

    Example::

        lbl = IrLabel(name="_start")
        str(lbl)    # "_start"
    """

    name: str

    def __str__(self) -> str:
        """Return the canonical text form: the label name as-is."""
        return self.name


# The Union type for any IR operand. Used in type annotations throughout
# the compiler and backend packages.
IrOperand = IrRegister | IrImmediate | IrFloatImmediate | IrLabel


# ---------------------------------------------------------------------------
# IrInstruction — a single IR instruction
# ---------------------------------------------------------------------------
#
# Every instruction has:
#   - opcode:   what operation to perform (ADD_IMM, BRANCH_Z, etc.)
#   - operands: the arguments (registers, immediates, labels)
#   - id:       a unique monotonic integer for source mapping
#
# The id field is the key that connects this instruction to the source
# map chain. Each instruction gets a unique id assigned by the IDGenerator,
# and that id flows through all pipeline stages.
#
# Examples:
#   IrInstruction(IrOp.ADD_IMM, [IrRegister(1), IrRegister(1), IrImmediate(1)], id=3)
#     →  ADD_IMM v1, v1, 1  ; #3
#
#   IrInstruction(IrOp.BRANCH_Z, [IrRegister(2), IrLabel("loop_0_end")], id=7)
#     →  BRANCH_Z v2, loop_0_end  ; #7
# ---------------------------------------------------------------------------


@dataclass
class IrInstruction:
    """A single IR instruction.

    Each instruction has an opcode, a list of operands, and a unique integer
    ID. The ID connects this instruction to the source map chain.

    Labels have ID = -1 because they produce no machine code and do not
    appear in the source map.

    Attributes:
        opcode:   The operation to perform.
        operands: The arguments (registers, immediates, labels). May be empty
                  for zero-operand instructions like HALT, RET, NOP.
        id:       Unique monotonic integer. -1 for label pseudo-instructions.

    Example::

        instr = IrInstruction(
            opcode=IrOp.ADD_IMM,
            operands=[IrRegister(1), IrRegister(1), IrImmediate(1)],
            id=3,
        )
    """

    opcode: IrOp
    operands: list[IrOperand] = field(default_factory=list)
    id: int = -1


# ---------------------------------------------------------------------------
# IrDataDecl — a data segment declaration
# ---------------------------------------------------------------------------
#
# Declares a named region of memory with a given size and initial byte value.
# For Brainfuck, this is the tape:
#
#   IrDataDecl(label="tape", size=30000, init=0)
#     →  .data tape 30000 0
#
# The ``init`` value is repeated for every byte in the region. ``init=0``
# means zero-initialized (equivalent to .bss in most formats).
# ---------------------------------------------------------------------------


@dataclass
class IrDataDecl:
    """A data segment declaration.

    Declares a named region of memory with a fixed size and initial byte
    value. Used for the Brainfuck tape and any future static data.

    Attributes:
        label: The name of the data region (e.g., ``"tape"``).
        size:  The number of bytes to allocate.
        init:  The initial byte value for every cell (usually 0).

    Example::

        decl = IrDataDecl(label="tape", size=30000, init=0)
        # Text form: .data tape 30000 0
    """

    label: str
    size: int
    init: int = 0  # initial byte value (usually 0)


# ---------------------------------------------------------------------------
# IrProgram — a complete IR program
# ---------------------------------------------------------------------------
#
# An IrProgram contains:
#   - instructions: the linear sequence of IR instructions
#   - data:         data segment declarations (.bss, .data)
#   - entry_label:  the label where execution begins
#   - version:      IR version number (1 = Brainfuck subset)
#
# The instructions list is ordered — execution flows from index 0 to len-1,
# with jumps/branches altering the flow.
# ---------------------------------------------------------------------------


@dataclass
class IrProgram:
    """A complete IR program.

    Contains everything the backend needs to generate machine code:
    the instruction stream, static data declarations, the entry point,
    and a version tag for forward compatibility.

    Attributes:
        entry_label:  The label where execution begins (e.g., ``"_start"``).
        version:      IR format version. Version 1 = Brainfuck subset.
        instructions: The ordered list of IR instructions.
        data:         Data segment declarations (static memory).

    Example::

        prog = IrProgram(entry_label="_start", version=1)
        prog.add_instruction(IrInstruction(IrOp.HALT, [], id=0))
    """

    entry_label: str
    version: int = 1
    instructions: list[IrInstruction] = field(default_factory=list)
    data: list[IrDataDecl] = field(default_factory=list)

    def add_instruction(self, instr: IrInstruction) -> None:
        """Append an instruction to the program.

        Args:
            instr: The instruction to append.
        """
        self.instructions.append(instr)

    def add_data(self, decl: IrDataDecl) -> None:
        """Append a data declaration to the program.

        Args:
            decl: The data declaration to append.
        """
        self.data.append(decl)


# ---------------------------------------------------------------------------
# IDGenerator — produces unique monotonic instruction IDs
# ---------------------------------------------------------------------------
#
# Every IR instruction in the pipeline needs a unique ID for source mapping.
# The IDGenerator ensures no two instructions ever share an ID, even across
# multiple compiler invocations within the same process.
#
# Usage:
#   gen = IDGenerator()
#   id1 = gen.next()  # 0
#   id2 = gen.next()  # 1
#   id3 = gen.next()  # 2
# ---------------------------------------------------------------------------


class IDGenerator:
    """Produces unique monotonic instruction IDs.

    Every IR instruction in the pipeline needs a unique integer ID for
    source map tracking. This generator ensures IDs are never reused.

    The generator starts at 0 (or a configurable starting value) and
    increments by 1 on each call to ``next()``.

    Example::

        gen = IDGenerator()
        gen.next()     # 0
        gen.next()     # 1
        gen.next()     # 2
        gen.current()  # 3  (next ID that will be returned)

    Starting from a custom value is useful when multiple compilers
    contribute instructions to the same program::

        gen2 = IDGenerator(start=100)
        gen2.next()  # 100
        gen2.next()  # 101
    """

    def __init__(self, start: int = 0) -> None:
        """Create a new ID generator.

        Args:
            start: The first ID to return. Defaults to 0.
        """
        self._next = start

    def next(self) -> int:
        """Return the next unique ID and advance the counter.

        Returns:
            A unique non-negative integer.
        """
        id_ = self._next
        self._next += 1
        return id_

    def current(self) -> int:
        """Return the current counter value without incrementing.

        This is the ID that will be returned by the next call to ``next()``.

        Returns:
            The current counter value.
        """
        return self._next
