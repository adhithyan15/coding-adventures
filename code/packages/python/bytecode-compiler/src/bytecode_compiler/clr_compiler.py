"""CLR IL Compiler — Targeting the Common Language Runtime.

=================================================================
Chapter 4a.2: From Trees to CLR Intermediate Language
=================================================================

The Common Language Runtime (CLR) is Microsoft's virtual machine, introduced
in 2002 as part of the .NET Framework. Like the JVM, it's a stack-based VM
that runs bytecode — but Microsoft calls it "Intermediate Language" (IL) or
sometimes "MSIL" (Microsoft Intermediate Language) or "CIL" (Common
Intermediate Language).

The CLR was designed *after* the JVM, and its designers learned from both the
JVM's strengths and its limitations. Some notable differences:

- **Richer short forms**: The CLR has dedicated opcodes for constants 0 through
  8 (the JVM only goes to 5). This reflects the observation that small numbers
  like 6, 7, 8 appear in real code more often than you'd expect.

- **Signed byte encoding**: The CLR's ``ldc.i4.s`` uses a signed byte for
  values -128 to 127, just like the JVM's ``bipush``.

- **Full 32-bit encoding**: For larger values, ``ldc.i4`` embeds a full 4-byte
  little-endian integer directly in the bytecode stream (5 bytes total). The
  JVM's ``ldc`` instead references a constant pool entry. The CLR approach is
  simpler but uses more space for large constants.

This module compiles our AST into real CLR IL bytes — the same instruction
format that the C# compiler (``csc`` / ``dotnet build``) produces.

Opcode reference
----------------
We use the following real CLR IL opcodes:

+------------------+------+-------------------------------------------+
| Instruction      | Byte | Description                               |
+==================+======+===========================================+
| ldloc.0          | 0x06 | Load local variable 0                     |
| ldloc.1          | 0x07 | Load local variable 1                     |
| ldloc.2          | 0x08 | Load local variable 2                     |
| ldloc.3          | 0x09 | Load local variable 3                     |
| stloc.0          | 0x0A | Store to local variable 0                 |
| stloc.1          | 0x0B | Store to local variable 1                 |
| stloc.2          | 0x0C | Store to local variable 2                 |
| stloc.3          | 0x0D | Store to local variable 3                 |
| ldloc.s          | 0x11 | Load local variable (short form, 2 bytes) |
| stloc.s          | 0x13 | Store to local variable (short form)      |
| ldc.i4.0         | 0x16 | Push int constant 0                       |
| ldc.i4.1         | 0x17 | Push int constant 1                       |
| ldc.i4.2         | 0x18 | Push int constant 2                       |
| ldc.i4.3         | 0x19 | Push int constant 3                       |
| ldc.i4.4         | 0x1A | Push int constant 4                       |
| ldc.i4.5         | 0x1B | Push int constant 5                       |
| ldc.i4.6         | 0x1C | Push int constant 6                       |
| ldc.i4.7         | 0x1D | Push int constant 7                       |
| ldc.i4.8         | 0x1E | Push int constant 8                       |
| ldc.i4.s         | 0x1F | Push signed byte (-128 to 127)            |
| ldc.i4           | 0x20 | Push 4-byte int32 (little-endian)         |
| pop              | 0x26 | Pop top value from stack                  |
| ret              | 0x2A | Return from method                        |
| add              | 0x58 | Integer addition                          |
| sub              | 0x59 | Integer subtraction                       |
| mul              | 0x5A | Integer multiplication                    |
| div              | 0x5B | Integer division                          |
+------------------+------+-------------------------------------------+
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

from lang_parser import (
    Assignment,
    BinaryOp,
    Expression,
    Name,
    NumberLiteral,
    Program,
    Statement,
    StringLiteral,
)


# ---------------------------------------------------------------------------
# CLR IL Opcode constants — real values from the ECMA-335 specification
# ---------------------------------------------------------------------------

# Load local variable (short forms for slots 0-3)
LDLOC_0 = 0x06
LDLOC_1 = 0x07
LDLOC_2 = 0x08
LDLOC_3 = 0x09

# Store to local variable (short forms for slots 0-3)
STLOC_0 = 0x0A
STLOC_1 = 0x0B
STLOC_2 = 0x0C
STLOC_3 = 0x0D

# Generic load/store with index byte
LDLOC_S = 0x11
STLOC_S = 0x13

# Push integer constants 0-8 (single-byte instructions)
LDC_I4_0 = 0x16
LDC_I4_1 = 0x17
LDC_I4_2 = 0x18
LDC_I4_3 = 0x19
LDC_I4_4 = 0x1A
LDC_I4_5 = 0x1B
LDC_I4_6 = 0x1C
LDC_I4_7 = 0x1D
LDC_I4_8 = 0x1E

# Push signed byte integer
LDC_I4_S = 0x1F

# Push full 4-byte int32 (little-endian)
LDC_I4 = 0x20

# Stack manipulation
POP = 0x26

# Return from method
RET = 0x2A

# Arithmetic
ADD = 0x58
SUB = 0x59
MUL = 0x5A
DIV = 0x5B

# ---------------------------------------------------------------------------
# Operator-to-opcode mapping
# ---------------------------------------------------------------------------

CLR_OPERATOR_MAP: dict[str, int] = {
    "+": ADD,
    "-": SUB,
    "*": MUL,
    "/": DIV,
}
"""Maps source-level operator symbols to their CLR IL bytecode equivalents.

The CLR arithmetic instructions work identically to the JVM's: pop two values
from the evaluation stack, perform the operation, push the result. The opcodes
are different (0x58 vs 0x60 for add), but the semantics are the same.
"""


# ---------------------------------------------------------------------------
# CLRCodeObject — the compilation output
# ---------------------------------------------------------------------------


@dataclass
class CLRCodeObject:
    """The result of compiling an AST to CLR IL bytecode.

    Unlike the JVM's ``JVMCodeObject``, the CLR code object does not need a
    separate constant pool for integers. The CLR embeds integer constants
    directly in the bytecode stream (using ``ldc.i4`` with 4 inline bytes),
    rather than referencing a pool. This is one of the key design differences
    between the two VMs.

    Attributes
    ----------
    bytecode : bytes
        The raw CLR IL bytecode bytes. In a real .NET assembly, this would
        be the method body inside the ``MethodBody`` structure.
    num_locals : int
        The number of local variable slots used. In a real assembly, this
        would be declared in the method's local variable signature.
    local_names : list[str]
        Maps slot indices to variable names. In a real assembly, this
        information would be in the PDB (Program Database) debug symbols.
    """

    bytecode: bytes
    num_locals: int = 0
    local_names: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# CLRCompiler — the AST-to-IL translator
# ---------------------------------------------------------------------------


class CLRCompiler:
    """Compiles an AST into CLR IL bytecode bytes.

    The CLR compiler follows the same pattern as the JVM compiler: walk the
    AST in post-order, emitting stack-machine instructions for each node.
    The differences are in the encoding details:

    - **Wider short-form range**: Constants 0-8 have dedicated single-byte
      opcodes (vs. 0-5 on the JVM).
    - **Inline integers**: Large constants are embedded directly as 4-byte
      little-endian values, not stored in a separate constant pool.
    - **Different opcode values**: ``add`` is 0x58 (vs. JVM's 0x60), etc.

    Example
    -------
    ::

        from lang_parser import Parser
        from lexer import Lexer
        from bytecode_compiler import CLRCompiler

        tokens = Lexer("x = 1 + 2").tokenize()
        ast = Parser(tokens).parse()

        compiler = CLRCompiler()
        code = compiler.compile(ast)
        # code.bytecode == bytes([0x17, 0x18, 0x58, 0x0A, 0x2A])
        #   ldc.i4.1, ldc.i4.2, add, stloc.0, ret
    """

    def __init__(self) -> None:
        """Initialize a fresh CLR compiler with empty state."""
        self._bytecode: bytearray = bytearray()
        """The growing bytecode buffer."""

        self._locals: list[str] = []
        """Maps local variable slot indices to their names."""

    # -------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------

    def compile(self, program: Program) -> CLRCodeObject:
        """Compile a full program AST into CLR IL bytecode.

        Walks every statement, emits bytes, then appends ``ret`` (0x2A).

        Parameters
        ----------
        program : Program
            The root AST node.

        Returns
        -------
        CLRCodeObject
            Contains the raw IL bytecode and local variable metadata.
        """
        for statement in program.statements:
            self._compile_statement(statement)

        # Every CLR method body must end with a ret instruction.
        self._bytecode.append(RET)

        return CLRCodeObject(
            bytecode=bytes(self._bytecode),
            num_locals=len(self._locals),
            local_names=list(self._locals),
        )

    # -------------------------------------------------------------------
    # Statement compilation
    # -------------------------------------------------------------------

    def _compile_statement(self, stmt: Statement) -> None:
        """Compile a single statement into CLR IL.

        Assignment statements compile the value and store it. Expression
        statements compile the expression and pop the result. The CLR's
        pop instruction (0x26) discards the top of the evaluation stack.

        Parameters
        ----------
        stmt : Statement
            Either an ``Assignment`` or an ``Expression`` node.
        """
        if isinstance(stmt, Assignment):
            self._compile_assignment(stmt)
        else:
            self._compile_expression(stmt)
            self._bytecode.append(POP)

    def _compile_assignment(self, node: Assignment) -> None:
        """Compile ``name = expression`` into CLR IL.

        Parameters
        ----------
        node : Assignment
            An AST node with ``target`` and ``value``.
        """
        self._compile_expression(node.value)
        slot = self._get_local_slot(node.target.name)
        self._emit_stloc(slot)

    # -------------------------------------------------------------------
    # Expression compilation
    # -------------------------------------------------------------------

    def _compile_expression(self, node: Expression) -> None:
        """Compile an expression, leaving exactly one value on the stack.

        Parameters
        ----------
        node : Expression
            Any AST expression node.

        Raises
        ------
        TypeError
            If the node type is not recognized.
        """
        if isinstance(node, NumberLiteral):
            self._emit_number(node.value)

        elif isinstance(node, StringLiteral):
            # The CLR handles strings via the ldstr instruction in real IL,
            # but for our purposes we treat string values like large constants
            # and encode them with ldc.i4 (which would be incorrect for real
            # .NET, but consistent with our simplified model).
            # For now, we raise an error since our language primarily handles
            # integers and the CLR doesn't have a simple string constant pool
            # like the JVM.
            raise TypeError(
                f"CLR compiler does not support string literals yet. "
                f"Got: {node.value!r}"
            )

        elif isinstance(node, Name):
            slot = self._get_local_slot(node.name)
            self._emit_ldloc(slot)

        elif isinstance(node, BinaryOp):
            self._compile_expression(node.left)
            self._compile_expression(node.right)
            opcode = CLR_OPERATOR_MAP[node.op]
            self._bytecode.append(opcode)

        else:
            raise TypeError(
                f"Unknown expression type: {type(node).__name__}. "
                f"The CLR compiler doesn't know how to handle this AST node."
            )

    # -------------------------------------------------------------------
    # Number encoding — the CLR's three tiers
    # -------------------------------------------------------------------

    def _emit_number(self, value: int) -> None:
        """Emit the most compact IL to push an integer onto the stack.

        The CLR has three encoding tiers, similar to the JVM but with a
        wider short-form range:

        1. **ldc.i4.N** (1 byte): For values 0 through 8. The CLR extends
           the short-form range by three compared to the JVM (which stops at
           5). This is because 6, 7, and 8 appear frequently enough in real
           code to justify dedicated opcodes.

        2. **ldc.i4.s N** (2 bytes): For values -128 through 127. Like the
           JVM's ``bipush``, this uses a single signed byte after the opcode.

        3. **ldc.i4 N** (5 bytes): For everything else. Unlike the JVM (which
           uses a constant pool), the CLR embeds the full 4-byte little-endian
           integer directly in the bytecode stream. This is simpler (no pool
           management) but uses more space for large constants that appear
           repeatedly.

        Parameters
        ----------
        value : int
            The integer value to push onto the CLR evaluation stack.
        """
        if 0 <= value <= 8:
            # Tier 1: Single-byte ldc.i4.N.
            # ldc.i4.0 is at 0x16, ldc.i4.1 at 0x17, ..., ldc.i4.8 at 0x1E.
            self._bytecode.append(LDC_I4_0 + value)

        elif -128 <= value <= 127:
            # Tier 2: Two-byte ldc.i4.s.
            # The value is encoded as a signed byte after the opcode.
            self._bytecode.append(LDC_I4_S)
            self._bytecode.append(value & 0xFF)

        else:
            # Tier 3: Five-byte ldc.i4.
            # The opcode (0x20) is followed by the value as a 4-byte
            # little-endian signed int32. This is a key difference from the
            # JVM: no constant pool needed, but 5 bytes per large constant.
            self._bytecode.append(LDC_I4)
            self._bytecode.extend(struct.pack("<i", value))

    # -------------------------------------------------------------------
    # Local variable encoding
    # -------------------------------------------------------------------

    def _emit_stloc(self, slot: int) -> None:
        """Emit a stloc instruction for the given local variable slot.

        1. **stloc.N** (1 byte): For slots 0 through 3.
        2. **stloc.s N** (2 bytes): For slot 4 and above.

        Parameters
        ----------
        slot : int
            The local variable slot index (0-based).
        """
        if slot <= 3:
            self._bytecode.append(STLOC_0 + slot)
        else:
            self._bytecode.append(STLOC_S)
            self._bytecode.append(slot)

    def _emit_ldloc(self, slot: int) -> None:
        """Emit a ldloc instruction for the given local variable slot.

        1. **ldloc.N** (1 byte): For slots 0 through 3.
        2. **ldloc.s N** (2 bytes): For slot 4 and above.

        Parameters
        ----------
        slot : int
            The local variable slot index (0-based).
        """
        if slot <= 3:
            self._bytecode.append(LDLOC_0 + slot)
        else:
            self._bytecode.append(LDLOC_S)
            self._bytecode.append(slot)

    # -------------------------------------------------------------------
    # Local slot management
    # -------------------------------------------------------------------

    def _get_local_slot(self, name: str) -> int:
        """Get (or assign) a local variable slot for the given name.

        Parameters
        ----------
        name : str
            The variable name.

        Returns
        -------
        int
            The slot index for this variable.
        """
        if name in self._locals:
            return self._locals.index(name)
        self._locals.append(name)
        return len(self._locals) - 1
