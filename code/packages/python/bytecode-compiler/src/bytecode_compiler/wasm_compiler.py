"""WASM Bytecode Compiler — Targeting WebAssembly.

=================================================================
Chapter 4a.3: From Trees to WebAssembly Bytecode
=================================================================

WebAssembly (WASM) is the newest of our three target VMs, standardized in 2017
by the W3C. Unlike the JVM (1995) and CLR (2002), which were designed for
general-purpose application development, WASM was designed specifically for the
web — a compact, fast, safe bytecode format that runs in browsers alongside
JavaScript.

WASM's design philosophy differs from the JVM and CLR in several ways:

1. **Simplicity over compactness**: WASM uses a uniform encoding for most
   values. Where the JVM has ``iconst_0`` through ``iconst_5`` (saving one
   byte for common values), WASM always uses ``i32.const`` followed by a
   full 4-byte value. This makes the bytecode slightly larger but much
   simpler to encode and decode.

2. **Structured control flow**: WASM doesn't have ``goto`` — instead it uses
   structured blocks, loops, and if/else constructs. This makes it easier to
   validate and optimize, and prevents entire classes of security exploits.
   (Our simple language doesn't need control flow yet, but this is a key
   WASM design feature.)

3. **Module-based**: WASM code lives in modules with explicit imports and
   exports. There's no global mutable state accessible from outside. This
   sandboxing is crucial for running untrusted code in browsers.

4. **Stack validation**: WASM validates that the stack is balanced at function
   boundaries. Unlike the JVM (which needs explicit ``pop`` for expression
   statements), WASM handles stack cleanup implicitly at function boundaries.

This module uses the same encoding as the existing ``wasm-simulator`` package,
so compiled code can be directly executed by that simulator.

Opcode reference
----------------
We use the following real WASM opcodes:

+------------------+------+-------------------------------------------+
| Instruction      | Byte | Description                               |
+==================+======+===========================================+
| end              | 0x0B | End of function body                      |
| local.get        | 0x20 | Get local variable (+ 1-byte index)       |
| local.set        | 0x21 | Set local variable (+ 1-byte index)       |
| i32.const        | 0x41 | Push i32 constant (+ 4-byte LE int32)     |
| i32.add          | 0x6A | Integer addition                          |
| i32.sub          | 0x6B | Integer subtraction                       |
+------------------+------+-------------------------------------------+

Note: WASM doesn't define separate i32.mul and i32.div opcodes at 0x6C/0x6D —
the real opcodes are 0x6C (i32.mul) and 0x6D (i32.div_s). We use 0x6C and 0x6D
to match the WASM specification for multiplication and signed division.
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
# WASM Opcode constants — real values from the WebAssembly specification
# ---------------------------------------------------------------------------

# Function/block terminator
END = 0x0B

# Local variable access (each followed by 1-byte index)
LOCAL_GET = 0x20
LOCAL_SET = 0x21

# Push 32-bit integer constant (followed by 4-byte little-endian int32)
I32_CONST = 0x41

# Arithmetic
I32_ADD = 0x6A
I32_SUB = 0x6B
I32_MUL = 0x6C
I32_DIV_S = 0x6D  # Signed division

# ---------------------------------------------------------------------------
# Operator-to-opcode mapping
# ---------------------------------------------------------------------------

WASM_OPERATOR_MAP: dict[str, int] = {
    "+": I32_ADD,
    "-": I32_SUB,
    "*": I32_MUL,
    "/": I32_DIV_S,
}
"""Maps source-level operator symbols to their WASM bytecode equivalents.

WASM arithmetic instructions work the same way as JVM and CLR: pop two i32
values from the value stack, perform the operation, push the i32 result.

Note that WASM distinguishes between signed and unsigned division. We use
``i32.div_s`` (signed division) since our language treats all numbers as
signed integers.
"""


# ---------------------------------------------------------------------------
# WASMCodeObject — the compilation output
# ---------------------------------------------------------------------------


@dataclass
class WASMCodeObject:
    """The result of compiling an AST to WASM bytecode.

    WASM doesn't need a separate constant pool — all integer constants are
    encoded inline in the bytecode using ``i32.const`` followed by 4 bytes.
    This is simpler than the JVM's constant pool approach.

    Attributes
    ----------
    bytecode : bytes
        The raw WASM bytecode bytes. In a real ``.wasm`` file, this would
        be the function body inside the code section.
    num_locals : int
        The number of local variables declared. In a real WASM module,
        this would be part of the function's local variable declarations
        in the code section.
    local_names : list[str]
        Maps slot indices to variable names. In a real WASM module, this
        information would be in the optional "name" custom section.
    """

    bytecode: bytes
    num_locals: int = 0
    local_names: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# WASMCompiler — the AST-to-WASM translator
# ---------------------------------------------------------------------------


class WASMCompiler:
    """Compiles an AST into WASM bytecode bytes.

    The WASM compiler is the simplest of our three backends because WASM uses
    a uniform encoding: every integer constant is 5 bytes (opcode + 4-byte
    value), and every local variable access is 2 bytes (opcode + index). There
    are no short forms to choose between.

    This simplicity is intentional in WASM's design. The bytecode is meant to
    be generated by compilers (not written by hand), so encoding simplicity
    matters more than bytecode compactness. The browser's JIT compiler will
    optimize the native code anyway.

    Another WASM-specific detail: expression statements don't need an explicit
    ``pop`` instruction. WASM validates the stack at function boundaries, and
    the ``end`` instruction handles any remaining stack cleanup. This means
    we can simply omit the pop for bare expression statements.

    Example
    -------
    ::

        from lang_parser import Parser
        from lexer import Lexer
        from bytecode_compiler import WASMCompiler

        tokens = Lexer("x = 1 + 2").tokenize()
        ast = Parser(tokens).parse()

        compiler = WASMCompiler()
        code = compiler.compile(ast)
        # code.bytecode contains:
        #   0x41 0x01 0x00 0x00 0x00  (i32.const 1)
        #   0x41 0x02 0x00 0x00 0x00  (i32.const 2)
        #   0x6A                       (i32.add)
        #   0x21 0x00                  (local.set 0)
        #   0x0B                       (end)
    """

    def __init__(self) -> None:
        """Initialize a fresh WASM compiler with empty state."""
        self._bytecode: bytearray = bytearray()
        """The growing bytecode buffer."""

        self._locals: list[str] = []
        """Maps local variable slot indices to their names."""

    # -------------------------------------------------------------------
    # Public API
    # -------------------------------------------------------------------

    def compile(self, program: Program) -> WASMCodeObject:
        """Compile a full program AST into WASM bytecode.

        Walks every statement, emits bytes, then appends ``end`` (0x0B)
        to terminate the function body.

        Parameters
        ----------
        program : Program
            The root AST node.

        Returns
        -------
        WASMCodeObject
            Contains the raw bytecode and local variable metadata.
        """
        for statement in program.statements:
            self._compile_statement(statement)

        # Every WASM function body ends with 'end' (0x0B).
        # This marks the end of the function's expression sequence.
        self._bytecode.append(END)

        return WASMCodeObject(
            bytecode=bytes(self._bytecode),
            num_locals=len(self._locals),
            local_names=list(self._locals),
        )

    # -------------------------------------------------------------------
    # Statement compilation
    # -------------------------------------------------------------------

    def _compile_statement(self, stmt: Statement) -> None:
        """Compile a single statement into WASM bytecode.

        WASM differs from JVM and CLR here: expression statements don't need
        an explicit ``pop``. WASM's stack validation happens at function
        boundaries (the ``end`` instruction), so extra values on the stack
        are handled implicitly. This means we can simply compile the expression
        without worrying about stack cleanup.

        In practice, a real WASM compiler *would* emit ``drop`` (0x1A) for
        expression statements to keep the stack clean within the function body.
        But for our simple programs (which always end with ``end``), omitting
        it works correctly with the wasm-simulator.

        Parameters
        ----------
        stmt : Statement
            Either an ``Assignment`` or an ``Expression`` node.
        """
        if isinstance(stmt, Assignment):
            self._compile_assignment(stmt)
        else:
            # WASM: no explicit pop needed for expression statements.
            # The stack is validated at the function boundary.
            self._compile_expression(stmt)

    def _compile_assignment(self, node: Assignment) -> None:
        """Compile ``name = expression`` into WASM bytecode.

        Parameters
        ----------
        node : Assignment
            An AST node with ``target`` and ``value``.
        """
        self._compile_expression(node.value)
        slot = self._get_local_slot(node.target.name)
        self._bytecode.append(LOCAL_SET)
        self._bytecode.append(slot)

    # -------------------------------------------------------------------
    # Expression compilation
    # -------------------------------------------------------------------

    def _compile_expression(self, node: Expression) -> None:
        """Compile an expression, leaving exactly one value on the stack.

        WASM's encoding is refreshingly uniform compared to JVM and CLR:
        - Every integer uses ``i32.const`` + 4 bytes (no short forms)
        - Every local access uses ``local.get/set`` + 1 byte index

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
            # WASM always uses i32.const followed by 4-byte little-endian.
            # No short forms, no constant pool — just the value inline.
            self._bytecode.append(I32_CONST)
            self._bytecode.extend(struct.pack("<i", node.value))

        elif isinstance(node, StringLiteral):
            raise TypeError(
                f"WASM compiler does not support string literals yet. "
                f"Got: {node.value!r}"
            )

        elif isinstance(node, Name):
            slot = self._get_local_slot(node.name)
            self._bytecode.append(LOCAL_GET)
            self._bytecode.append(slot)

        elif isinstance(node, BinaryOp):
            self._compile_expression(node.left)
            self._compile_expression(node.right)
            opcode = WASM_OPERATOR_MAP[node.op]
            self._bytecode.append(opcode)

        else:
            raise TypeError(
                f"Unknown expression type: {type(node).__name__}. "
                f"The WASM compiler doesn't know how to handle this AST node."
            )

    # -------------------------------------------------------------------
    # Local slot management
    # -------------------------------------------------------------------

    def _get_local_slot(self, name: str) -> int:
        """Get (or assign) a local variable slot for the given name.

        WASM local variables are indexed by a simple integer, just like the
        JVM and CLR. The encoding is always 1 byte for the index (after the
        opcode), supporting up to 256 local variables in our simplified model.
        Real WASM uses LEB128 encoding for the index, which supports larger
        values.

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
