"""IR Opcodes — the instruction set for the general-purpose AOT compiler IR.

Design Philosophy
-----------------

This IR is **general-purpose** — designed to serve as the compilation target
for any compiled language, not just Brainfuck. The current v1 instruction set
is sufficient for Brainfuck; BASIC (the next planned frontend) will add opcodes
for multiplication, division, floating-point arithmetic, and string operations.

Key rules:
  1. Existing opcodes never change semantics — only new ones are appended.
  2. A new opcode is added only when a frontend needs it AND it cannot be
     efficiently expressed as a sequence of existing opcodes.
  3. All frontends and backends remain forward-compatible.

Opcode Groups
-------------

The opcodes are grouped by category:

  Constants:    LOAD_IMM, LOAD_ADDR
  Memory:       LOAD_BYTE, STORE_BYTE, LOAD_WORD, STORE_WORD
  Arithmetic:   ADD, ADD_IMM, SUB, AND, AND_IMM, MUL, DIV
  Bitwise:      OR, OR_IMM, XOR, XOR_IMM, NOT
  Comparison:   CMP_EQ, CMP_NE, CMP_LT, CMP_GT
  Floating:     LOAD_F64_IMM, LOAD_F64, STORE_F64, F64_ADD, ..., F64_FROM_I32,
                I32_TRUNC_FROM_F64
  Control Flow: LABEL, JUMP, BRANCH_Z, BRANCH_NZ, CALL, RET
  System:       SYSCALL, HALT
  Meta:         NOP, COMMENT
  Closures:     MAKE_CLOSURE, APPLY_CLOSURE   (TW03 Phase 2)

Text Names
----------

Each opcode has a canonical text name (e.g., ``ADD_IMM``, ``BRANCH_Z``).
These names are used by the IR printer and parser for roundtrip fidelity.
The ``IrOp`` enum values map directly to integer codes (0, 1, 2, ...) so
they can be stored compactly.
"""

from __future__ import annotations

from enum import IntEnum


class IrOp(IntEnum):
    """Opcodes for the general-purpose IR instruction set.

    Each opcode is an integer for compact storage and fast comparison.
    The integer values are stable — new opcodes are appended, never inserted.

    Example usage::

        op = IrOp.ADD_IMM
        print(op.name)   # "ADD_IMM"
        print(int(op))   # 7

        # Convert from name:
        op2 = IrOp[op.name]  # IrOp.ADD_IMM
    """

    # ── Constants ──────────────────────────────────────────────────────────────
    # Load an immediate integer value into a register.
    #   LOAD_IMM  v0, 42    →  v0 = 42
    LOAD_IMM = 0

    # Load the address of a data label into a register.
    #   LOAD_ADDR v0, tape  →  v0 = &tape
    LOAD_ADDR = 1

    # ── Memory ────────────────────────────────────────────────────────────────
    # Load a byte from memory: dst = mem[base + offset] (zero-extended).
    #   LOAD_BYTE v2, v0, v1  →  v2 = mem[v0 + v1] & 0xFF
    LOAD_BYTE = 2

    # Store a byte to memory: mem[base + offset] = src & 0xFF.
    #   STORE_BYTE v2, v0, v1  →  mem[v0 + v1] = v2 & 0xFF
    STORE_BYTE = 3

    # Load a machine word from memory: dst = *(word*)(base + offset).
    #   LOAD_WORD v2, v0, v1  →  v2 = *(int*)(v0 + v1)
    LOAD_WORD = 4

    # Store a machine word to memory: *(word*)(base + offset) = src.
    #   STORE_WORD v2, v0, v1  →  *(int*)(v0 + v1) = v2
    STORE_WORD = 5

    # ── Arithmetic ────────────────────────────────────────────────────────────
    # Register-register addition: dst = lhs + rhs.
    #   ADD v3, v1, v2  →  v3 = v1 + v2
    ADD = 6

    # Register-immediate addition: dst = src + immediate.
    #   ADD_IMM v1, v1, 1  →  v1 = v1 + 1
    ADD_IMM = 7

    # Register-register subtraction: dst = lhs - rhs.
    #   SUB v3, v1, v2  →  v3 = v1 - v2
    SUB = 8

    # Register-register bitwise AND: dst = lhs & rhs.
    #   AND v3, v1, v2  →  v3 = v1 & v2
    AND = 9

    # Register-immediate bitwise AND: dst = src & immediate.
    #   AND_IMM v2, v2, 255  →  v2 = v2 & 0xFF
    AND_IMM = 10

    # Register-register multiplication: dst = lhs * rhs (signed integer).
    # For 20-bit targets the result is the low 20 bits of the product.
    #   MUL v3, v1, v2  →  v3 = v1 * v2
    MUL = 25

    # Register-register integer division: dst = lhs / rhs (truncates toward zero).
    # Division by zero is a runtime error; the backend is responsible for detection.
    #   DIV v3, v1, v2  →  v3 = v1 / v2
    DIV = 26

    # ── Bitwise ───────────────────────────────────────────────────────────────
    # Register-register bitwise OR: dst = lhs | rhs.
    # Clears the carry flag on targets where AND/OR/XOR affect flags (e.g. 8008 ORA).
    #   OR v3, v1, v2  →  v3 = v1 | v2
    OR = 27

    # Register-immediate bitwise OR: dst = src | immediate.
    #   OR_IMM v2, v2, 0x80  →  v2 = v2 | 0x80
    OR_IMM = 28

    # Register-register bitwise XOR: dst = lhs ^ rhs.
    # Also clears the carry flag on flag-setting targets (e.g. 8008 XRA).
    #   XOR v3, v1, v2  →  v3 = v1 ^ v2
    XOR = 29

    # Register-immediate bitwise XOR: dst = src ^ immediate.
    # The canonical NOT-a-byte idiom is XOR_IMM dst, src, 0xFF (flip all 8 bits).
    #   XOR_IMM v2, v2, 0xFF  →  v2 = v2 ^ 0xFF
    XOR_IMM = 30

    # Bitwise NOT (complement): dst = ~src.
    # Flips every bit in src.  On platforms with no single NOT instruction
    # (e.g. Intel 8008), the backend lowers this to XRI 0xFF (XOR-immediate 255).
    # On WASM i32, it becomes i32.xor with 0xFFFF_FFFF.
    #   NOT v2, v1  →  v2 = ~v1
    NOT = 31

    # ── Floating-point ───────────────────────────────────────────────────────
    # Load an immediate 64-bit float into a register.
    #   LOAD_F64_IMM v0, 1.5  →  v0 = 1.5
    LOAD_F64_IMM = 32

    # Load a 64-bit float from memory: dst = *(double*)(base + offset).
    #   LOAD_F64 v2, v0, v1  →  v2 = *(double*)(v0 + v1)
    LOAD_F64 = 33

    # Store a 64-bit float to memory: *(double*)(base + offset) = src.
    #   STORE_F64 v2, v0, v1  →  *(double*)(v0 + v1) = v2
    STORE_F64 = 34

    # Register-register f64 addition: dst = lhs + rhs.
    F64_ADD = 35

    # Register-register f64 subtraction: dst = lhs - rhs.
    F64_SUB = 36

    # Register-register f64 multiplication: dst = lhs * rhs.
    F64_MUL = 37

    # Register-register f64 division: dst = lhs / rhs.
    F64_DIV = 38

    # Set dst = 1 if lhs == rhs, else 0.
    F64_CMP_EQ = 39

    # Set dst = 1 if lhs != rhs, else 0.
    F64_CMP_NE = 40

    # Set dst = 1 if lhs < rhs, else 0.
    F64_CMP_LT = 41

    # Set dst = 1 if lhs > rhs, else 0.
    F64_CMP_GT = 42

    # Set dst = 1 if lhs <= rhs, else 0.
    F64_CMP_LE = 43

    # Set dst = 1 if lhs >= rhs, else 0.
    F64_CMP_GE = 44

    # Convert a signed i32 value to f64.
    #   F64_FROM_I32 v1, v2  →  v1 = float(v2)
    F64_FROM_I32 = 45

    # Truncate an f64 value toward zero into a signed i32.
    #   I32_TRUNC_FROM_F64 v1, v2  →  v1 = trunc(v2)
    I32_TRUNC_FROM_F64 = 46

    # ── Comparison ────────────────────────────────────────────────────────────
    # Set dst = 1 if lhs == rhs, else 0.
    #   CMP_EQ v4, v1, v2  →  v4 = (v1 == v2) ? 1 : 0
    CMP_EQ = 11

    # Set dst = 1 if lhs != rhs, else 0.
    #   CMP_NE v4, v1, v2  →  v4 = (v1 != v2) ? 1 : 0
    CMP_NE = 12

    # Set dst = 1 if lhs < rhs (signed), else 0.
    #   CMP_LT v4, v1, v2  →  v4 = (v1 < v2) ? 1 : 0
    CMP_LT = 13

    # Set dst = 1 if lhs > rhs (signed), else 0.
    #   CMP_GT v4, v1, v2  →  v4 = (v1 > v2) ? 1 : 0
    CMP_GT = 14

    # ── Control Flow ──────────────────────────────────────────────────────────
    # Define a label at this point in the instruction stream.
    # Labels produce no machine code — they just record an address.
    #   LABEL loop_start
    LABEL = 15

    # Unconditional jump to a label.
    #   JUMP loop_start  →  PC = &loop_start
    JUMP = 16

    # Conditional branch: jump to label if register == 0.
    #   BRANCH_Z v2, loop_end  →  if v2 == 0 then PC = &loop_end
    BRANCH_Z = 17

    # Conditional branch: jump to label if register != 0.
    #   BRANCH_NZ v2, loop_end  →  if v2 != 0 then PC = &loop_end
    BRANCH_NZ = 18

    # Call a subroutine at the given label. Pushes return address.
    #   CALL my_func
    CALL = 19

    # Return from a subroutine. Pops return address.
    #   RET
    RET = 20

    # ── System ────────────────────────────────────────────────────────────────
    # Invoke a system call. The syscall number is an immediate operand.
    # Arguments and return values follow the platform's syscall ABI.
    #   SYSCALL 1  →  ecall with a7=1 (write)
    SYSCALL = 21

    # Halt execution. The program terminates.
    #   HALT  →  ecall with a7=10 (exit)
    HALT = 22

    # ── Meta ──────────────────────────────────────────────────────────────────
    # No operation. Produces a single NOP instruction in the backend.
    #   NOP
    NOP = 23

    # A human-readable comment. Produces no machine code.
    # Useful for debugging IR output.
    #   COMMENT "load tape base address"
    COMMENT = 24

    # ── Closures (TW03 Phase 2 — cross-backend Lisp closure support) ─────
    # Construct a closure value that captures values from the enclosing
    # lexical scope.  The closure can later be invoked via APPLY_CLOSURE.
    #
    # Operand layout:
    #   MAKE_CLOSURE dst, fn_label, num_captured, capt0, capt1, ...
    # where:
    #   - ``dst``         — register receiving the resulting closure handle
    #   - ``fn_label``    — IR label of the lifted lambda body (a top-level
    #                       region just like a user-defined function)
    #   - ``num_captured`` — IrImmediate count of captured values
    #   - ``capt0..captN-1`` — registers holding the captured values
    #
    # Backend lowering strategies (see TW03 spec):
    #   - JVM/CLR:  allocate a closure object whose fields hold the captures;
    #               the resulting reference is the "closure handle".
    #   - BEAM:     emit ``make_fun2`` referencing a FunT-table entry whose
    #               free-variable list maps to the captured registers.
    #   - vm-core:  delegate to the host-side ``make_closure`` builtin
    #               (already implemented in TW00).
    MAKE_CLOSURE = 47

    # Apply a closure value to zero or more arguments.
    #
    # Operand layout:
    #   APPLY_CLOSURE dst, closure_reg, num_args, arg0, arg1, ...
    # where:
    #   - ``dst``         — register receiving the call's return value
    #   - ``closure_reg`` — register holding the closure handle from
    #                        a prior MAKE_CLOSURE
    #   - ``num_args``    — IrImmediate count of arguments
    #   - ``arg0..argN-1`` — registers holding the argument values
    #
    # Backend lowering strategies:
    #   - JVM/CLR:  invoke the closure object's ``apply(int...)`` method.
    #   - BEAM:     emit ``call_fun`` (or ``call_fun2``) on the closure
    #               handle.
    #   - vm-core:  delegate to the host-side ``apply_closure`` builtin.
    APPLY_CLOSURE = 48


# Canonical name → opcode mapping. Built from the enum at module load time.
# Used by the IR parser to convert text opcode names back to IrOp values.
#
# Example:
#   NAME_TO_OP["ADD_IMM"]  →  IrOp.ADD_IMM
#   NAME_TO_OP["HALT"]     →  IrOp.HALT
NAME_TO_OP: dict[str, IrOp] = {op.name: op for op in IrOp}

# Opcode → canonical name mapping. The inverse of NAME_TO_OP.
# Used by the IR printer.
#
# Example:
#   OP_NAMES[IrOp.ADD_IMM]  →  "ADD_IMM"
OP_NAMES: dict[IrOp, str] = {op: op.name for op in IrOp}


def parse_op(name: str) -> IrOp | None:
    """Convert a text opcode name to its IrOp value.

    Returns the opcode if found, or ``None`` if the name is not recognized.
    This is the inverse of ``IrOp.name``.

    Args:
        name: The canonical opcode name (e.g., ``"ADD_IMM"``).

    Returns:
        The ``IrOp`` value, or ``None`` if not found.

    Example::

        op = parse_op("ADD_IMM")   # IrOp.ADD_IMM
        bad = parse_op("FROBNITZ") # None
    """
    return NAME_TO_OP.get(name)
