"""JVM Bytecode Compiler — Targeting the Java Virtual Machine.

=================================================================
Chapter 4a.1: From Trees to JVM Bytecode
=================================================================

The Java Virtual Machine (JVM) is one of the most successful virtual machines
ever built. Created by James Gosling at Sun Microsystems in 1995, it has become
the runtime for not just Java, but also Kotlin, Scala, Clojure, and many other
languages.

This module compiles our AST into *real* JVM bytecode bytes — the same format
that ``javac`` produces when it compiles ``.java`` files into ``.class`` files.
While we don't produce complete ``.class`` files (which need headers, constant
pool tables, method descriptors, etc.), we do emit the actual instruction bytes
that the JVM would execute inside a method body.

How JVM bytecode works
----------------------
The JVM is a **stack machine**, just like our custom VM. But where our VM uses
high-level instructions like ``LOAD_CONST 0`` (opcode + index), the JVM uses
compact byte-level encodings designed to minimize class file size. This was a
deliberate design choice in 1995 when bandwidth was expensive — Java applets
needed to download quickly over dial-up connections.

The JVM has several clever encoding tricks:

1. **Short-form instructions**: Instead of always using ``bipush N`` (2 bytes)
   for small numbers, the JVM has dedicated single-byte opcodes for the most
   common values: ``iconst_0`` through ``iconst_5``. Since most programs use
   small numbers frequently, this saves significant space.

2. **Local variable slots**: Similarly, instead of always using ``istore N``
   (2 bytes), there are single-byte forms ``istore_0`` through ``istore_3``
   for the first four local variables. Most methods have fewer than four locals.

3. **Constant pool**: For values too large for ``bipush`` (-128 to 127), the
   JVM stores them in a constant pool and uses ``ldc`` (load constant) to
   reference them by index.

These optimizations are why JVM bytecode is remarkably compact — a design
principle that influenced later VMs like the CLR and Dalvik.

Opcode reference
----------------
We use the following real JVM opcodes (values from the JVM specification):

+------------------+------+-------------------------------------------+
| Instruction      | Byte | Description                               |
+==================+======+===========================================+
| iconst_0         | 0x03 | Push int constant 0                       |
| iconst_1         | 0x04 | Push int constant 1                       |
| iconst_2         | 0x05 | Push int constant 2                       |
| iconst_3         | 0x06 | Push int constant 3                       |
| iconst_4         | 0x07 | Push int constant 4                       |
| iconst_5         | 0x08 | Push int constant 5                       |
| bipush           | 0x10 | Push byte-sized int (-128 to 127)         |
| ldc              | 0x12 | Load from constant pool by index          |
| iload            | 0x15 | Load int from local variable (2 bytes)    |
| iload_0          | 0x1A | Load int from local variable 0            |
| iload_1          | 0x1B | Load int from local variable 1            |
| iload_2          | 0x1C | Load int from local variable 2            |
| iload_3          | 0x1D | Load int from local variable 3            |
| istore           | 0x36 | Store int to local variable (2 bytes)     |
| istore_0         | 0x3B | Store int to local variable 0             |
| istore_1         | 0x3C | Store int to local variable 1             |
| istore_2         | 0x3D | Store int to local variable 2             |
| istore_3         | 0x3E | Store int to local variable 3             |
| pop              | 0x57 | Pop top value from stack                  |
| iadd             | 0x60 | Integer addition                          |
| isub             | 0x64 | Integer subtraction                       |
| imul             | 0x68 | Integer multiplication                    |
| idiv             | 0x6C | Integer division                          |
| return           | 0xB1 | Return void from method                   |
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
# JVM Opcode constants — real values from the JVM specification
# ---------------------------------------------------------------------------

# Push integer constants 0-5 (single-byte instructions)
ICONST_0 = 0x03
ICONST_1 = 0x04
ICONST_2 = 0x05
ICONST_3 = 0x06
ICONST_4 = 0x07
ICONST_5 = 0x08

# Push byte-sized integer (-128 to 127)
BIPUSH = 0x10

# Load from constant pool
LDC = 0x12

# Load integer from local variable
ILOAD = 0x15  # Generic form: iload + index byte
ILOAD_0 = 0x1A
ILOAD_1 = 0x1B
ILOAD_2 = 0x1C
ILOAD_3 = 0x1D

# Store integer to local variable
ISTORE = 0x36  # Generic form: istore + index byte
ISTORE_0 = 0x3B
ISTORE_1 = 0x3C
ISTORE_2 = 0x3D
ISTORE_3 = 0x3E

# Stack manipulation
POP = 0x57

# Arithmetic
IADD = 0x60
ISUB = 0x64
IMUL = 0x68
IDIV = 0x6C

# Return void
RETURN = 0xB1

# ---------------------------------------------------------------------------
# Operator-to-opcode mapping
# ---------------------------------------------------------------------------

JVM_OPERATOR_MAP: dict[str, int] = {
    "+": IADD,
    "-": ISUB,
    "*": IMUL,
    "/": IDIV,
}
"""Maps source-level operator symbols to their JVM bytecode equivalents.

Just like the original compiler's OPERATOR_MAP, this table separates the data
(which opcode corresponds to which operator) from the logic (how to compile a
binary operation). The JVM arithmetic opcodes operate on the top two values of
the operand stack, popping both and pushing the result — exactly the same
semantics as our custom VM.
"""


# ---------------------------------------------------------------------------
# JVMCodeObject — the compilation output
# ---------------------------------------------------------------------------


@dataclass
class JVMCodeObject:
    """The result of compiling an AST to JVM bytecode.

    This is analogous to the ``CodeObject`` from our custom VM compiler, but
    instead of high-level ``Instruction`` objects, it contains raw bytes — the
    actual byte sequence that a JVM would execute.

    Attributes
    ----------
    bytecode : bytes
        The raw JVM bytecode bytes. This is the method body that would appear
        inside a ``.class`` file's ``Code`` attribute.
    constants : list[int | str]
        The constant pool — values referenced by ``ldc`` instructions. In a
        real ``.class`` file, this would be part of the class-level constant
        pool with type tags and UTF-8 encoding. We simplify it to a flat list.
    num_locals : int
        The number of local variable slots used. In a real ``.class`` file,
        this would be the ``max_locals`` field of the ``Code`` attribute.
    local_names : list[str]
        Maps slot indices to variable names. Slot 0 is the first variable
        assigned, slot 1 is the second, etc. In a real ``.class`` file, this
        information would be in the optional ``LocalVariableTable`` attribute.
    """

    bytecode: bytes
    constants: list[int | str] = field(default_factory=list)
    num_locals: int = 0
    local_names: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# JVMCompiler — the AST-to-bytecode translator
# ---------------------------------------------------------------------------


class JVMCompiler:
    """Compiles an AST into JVM bytecode bytes.

    This compiler walks the same AST that our custom ``BytecodeCompiler`` uses,
    but instead of emitting ``Instruction`` objects, it emits raw bytes using
    real JVM opcode values. The result is a ``JVMCodeObject`` containing the
    bytecode bytes, a constant pool, and local variable metadata.

    The compiler uses the JVM's compact encoding scheme:

    - Small integers (0-5) use dedicated single-byte ``iconst_N`` instructions
    - Medium integers (-128 to 127) use two-byte ``bipush N`` instructions
    - Larger integers use ``ldc`` with a constant pool reference
    - The first four local variables use single-byte ``istore_N``/``iload_N``
    - Additional locals use two-byte ``istore N``/``iload N``

    This tiered encoding is a hallmark of JVM design — optimize for the common
    case (small numbers and few variables) while still supporting the general
    case.

    Example
    -------
    ::

        from lang_parser import Parser
        from lexer import Lexer
        from bytecode_compiler import JVMCompiler

        tokens = Lexer("x = 1 + 2").tokenize()
        ast = Parser(tokens).parse()

        compiler = JVMCompiler()
        code = compiler.compile(ast)
        # code.bytecode == bytes([0x04, 0x05, 0x60, 0x3B, 0xB1])
        #   iconst_1, iconst_2, iadd, istore_0, return
    """

    def __init__(self) -> None:
        """Initialize a fresh compiler with empty state.

        Each ``JVMCompiler`` instance compiles exactly one ``Program``. Create
        a new instance for each program you want to compile.
        """
        self._bytecode: bytearray = bytearray()
        """The growing bytecode buffer. We use a bytearray for efficient
        appending, then convert to immutable bytes at the end."""

        self._constants: list[int | str] = []
        """The constant pool for values too large for inline encoding."""

        self._locals: list[str] = []
        """Maps local variable slot indices to their names. The first variable
        assigned gets slot 0, the second gets slot 1, etc."""

    # -------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------

    def compile(self, program: Program) -> JVMCodeObject:
        """Compile a full program AST into JVM bytecode.

        Walks every statement in the program, emitting bytes for each one,
        then appends a ``return`` instruction (0xB1) to end the method.

        Parameters
        ----------
        program : Program
            The root AST node, as produced by ``Parser.parse()``.

        Returns
        -------
        JVMCodeObject
            Contains the raw bytecode, constant pool, and local variable info.
        """
        for statement in program.statements:
            self._compile_statement(statement)

        # Every JVM method must end with a return instruction.
        # We use 'return' (0xB1) which returns void — appropriate since our
        # programs don't have an explicit return value.
        self._bytecode.append(RETURN)

        return JVMCodeObject(
            bytecode=bytes(self._bytecode),
            constants=self._constants,
            num_locals=len(self._locals),
            local_names=list(self._locals),
        )

    # -------------------------------------------------------------------
    # Statement compilation
    # -------------------------------------------------------------------

    def _compile_statement(self, stmt: Statement) -> None:
        """Compile a single statement into JVM bytecode.

        Assignment statements compile the value expression and then store the
        result into a local variable slot. Expression statements compile the
        expression and then pop the result off the stack (since nobody captures
        it). The JVM requires the operand stack to be balanced — you can't leave
        stray values on it.

        Parameters
        ----------
        stmt : Statement
            Either an ``Assignment`` or an ``Expression`` node.
        """
        if isinstance(stmt, Assignment):
            self._compile_assignment(stmt)
        else:
            # Expression statement: evaluate for side effects, then discard.
            # The JVM's pop instruction (0x57) removes the top value from the
            # operand stack, keeping things tidy.
            self._compile_expression(stmt)
            self._bytecode.append(POP)

    def _compile_assignment(self, node: Assignment) -> None:
        """Compile ``name = expression`` into JVM bytecode.

        First compiles the right-hand side (pushes value onto stack), then
        emits an ``istore`` instruction to pop and store into a local slot.

        The local slot is determined by the order in which variables are first
        seen: the first variable gets slot 0, the second gets slot 1, etc.
        This mirrors how ``javac`` assigns local variable slots.

        Parameters
        ----------
        node : Assignment
            An AST node with ``target`` (a Name) and ``value`` (an Expression).
        """
        # Compile the value expression — puts result on the operand stack.
        self._compile_expression(node.value)

        # Determine which local slot this variable maps to.
        slot = self._get_local_slot(node.target.name)

        # Emit the appropriate istore instruction.
        self._emit_istore(slot)

    # -------------------------------------------------------------------
    # Expression compilation — the recursive heart
    # -------------------------------------------------------------------

    def _compile_expression(self, node: Expression) -> None:
        """Compile an expression, leaving exactly one value on the stack.

        This is the recursive core of the compiler. Each expression type has
        its own compilation strategy, but they all share the same contract:
        after compilation, exactly one new value sits on top of the operand
        stack.

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
            # Strings always go through the constant pool.
            # The JVM's ldc instruction can load both integers and strings
            # from the constant pool.
            const_index = self._add_constant(node.value)
            self._bytecode.append(LDC)
            self._bytecode.append(const_index)

        elif isinstance(node, Name):
            # Load a local variable onto the stack.
            slot = self._get_local_slot(node.name)
            self._emit_iload(slot)

        elif isinstance(node, BinaryOp):
            # Post-order traversal: compile left, compile right, emit operator.
            # The JVM arithmetic instructions (iadd, isub, etc.) pop two values
            # and push the result, just like our custom VM.
            self._compile_expression(node.left)
            self._compile_expression(node.right)
            opcode = JVM_OPERATOR_MAP[node.op]
            self._bytecode.append(opcode)

        else:
            raise TypeError(
                f"Unknown expression type: {type(node).__name__}. "
                f"The JVM compiler doesn't know how to handle this AST node."
            )

    # -------------------------------------------------------------------
    # Number encoding — the JVM's tiered approach
    # -------------------------------------------------------------------

    def _emit_number(self, value: int) -> None:
        """Emit the most compact bytecode to push an integer onto the stack.

        The JVM has three ways to push an integer, each suited to a different
        range:

        1. **iconst_N** (1 byte): For values 0 through 5. These are the most
           common integer values in programs, so they get dedicated single-byte
           opcodes. ``iconst_0`` is 0x03, ``iconst_5`` is 0x08.

        2. **bipush N** (2 bytes): For values -128 through 127. The ``bipush``
           opcode (0x10) is followed by a single signed byte. This covers most
           loop counters, array indices, and small constants.

        3. **ldc index** (2 bytes): For anything else. The value is stored in
           the constant pool, and ``ldc`` (0x12) loads it by index. This
           handles large numbers at the cost of an extra constant pool entry.

        Parameters
        ----------
        value : int
            The integer value to push onto the JVM operand stack.
        """
        if 0 <= value <= 5:
            # Tier 1: Single-byte iconst_N.
            # iconst_0 is at 0x03, iconst_1 at 0x04, ..., iconst_5 at 0x08.
            self._bytecode.append(ICONST_0 + value)

        elif -128 <= value <= 127:
            # Tier 2: Two-byte bipush.
            # The value is encoded as a signed byte after the opcode.
            self._bytecode.append(BIPUSH)
            self._bytecode.append(value & 0xFF)

        else:
            # Tier 3: Constant pool reference via ldc.
            # Store the value in the constant pool and emit ldc + index.
            const_index = self._add_constant(value)
            self._bytecode.append(LDC)
            self._bytecode.append(const_index)

    # -------------------------------------------------------------------
    # Local variable encoding — another tiered approach
    # -------------------------------------------------------------------

    def _emit_istore(self, slot: int) -> None:
        """Emit an istore instruction for the given local variable slot.

        Like number encoding, local variable access has two tiers:

        1. **istore_N** (1 byte): For slots 0 through 3. These cover the first
           four local variables, which is enough for most simple methods.

        2. **istore N** (2 bytes): For slot 4 and above. The generic form uses
           the ``istore`` opcode (0x36) followed by the slot index as a byte.

        Parameters
        ----------
        slot : int
            The local variable slot index (0-based).
        """
        if slot <= 3:
            # Short form: istore_0 (0x3B) through istore_3 (0x3E)
            self._bytecode.append(ISTORE_0 + slot)
        else:
            # Long form: istore + slot index byte
            self._bytecode.append(ISTORE)
            self._bytecode.append(slot)

    def _emit_iload(self, slot: int) -> None:
        """Emit an iload instruction for the given local variable slot.

        Same tiered approach as istore:

        1. **iload_N** (1 byte): For slots 0 through 3.
        2. **iload N** (2 bytes): For slot 4 and above.

        Parameters
        ----------
        slot : int
            The local variable slot index (0-based).
        """
        if slot <= 3:
            # Short form: iload_0 (0x1A) through iload_3 (0x1D)
            self._bytecode.append(ILOAD_0 + slot)
        else:
            # Long form: iload + slot index byte
            self._bytecode.append(ILOAD)
            self._bytecode.append(slot)

    # -------------------------------------------------------------------
    # Pool management
    # -------------------------------------------------------------------

    def _add_constant(self, value: int | str) -> int:
        """Add a value to the constant pool, returning its index. Deduplicates.

        The constant pool stores values that are too large or complex to encode
        inline in the bytecode. Each unique value is stored once, and subsequent
        references reuse the same index.

        In a real JVM ``.class`` file, the constant pool is much more elaborate,
        with type tags (CONSTANT_Integer, CONSTANT_String, etc.) and cross-
        references between entries. Our simplified version is a flat list.

        Parameters
        ----------
        value : int | str
            The value to store in the constant pool.

        Returns
        -------
        int
            The index of the value in the constant pool.
        """
        if value in self._constants:
            return self._constants.index(value)
        self._constants.append(value)
        return len(self._constants) - 1

    def _get_local_slot(self, name: str) -> int:
        """Get (or assign) a local variable slot for the given name.

        Local variables in the JVM are stored in a numbered array of slots.
        Each variable gets a unique slot index, assigned in the order they
        are first encountered. This is the same strategy ``javac`` uses for
        local variables in a method.

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
