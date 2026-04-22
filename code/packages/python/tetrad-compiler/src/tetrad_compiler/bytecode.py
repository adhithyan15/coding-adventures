"""Bytecode data structures for the Tetrad VM (spec TET03).

A ``CodeObject`` is the compiled form of one function or the top-level program.
An ``Instruction`` is a single VM instruction with opcode and operands.

Opcode constants are gathered in the ``Op`` namespace class so callers can
write ``Op.ADD`` instead of bare integers.

Two-path compilation: arithmetic and comparison instructions either carry a
feedback slot operand (untyped path, ``len(operands) == 2``) or omit it
(typed path, ``len(operands) == 1``).  The VM checks ``len(operands)`` at
dispatch time to decide whether to record type observations.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from tetrad_type_checker.types import FunctionTypeStatus


class Op:
    """Opcode constants.

    Layout mirrors the ISA table in spec TET03:
      0x00–0x0F  Accumulator loads
      0x10–0x1F  Accumulator stores
      0x20–0x2F  Arithmetic (feedback-slotted when untyped)
      0x30–0x3F  Bitwise (never slotted — always u8 in v1)
      0x40–0x4F  Comparisons (feedback-slotted when untyped)
      0x50–0x5F  Logical helpers
      0x60–0x6F  Control flow / jumps
      0x70–0x7F  Function calls
      0x80–0x8F  I/O
      0xFF       HALT
    """

    # Loads
    LDA_IMM = 0x00   # acc = imm8
    LDA_ZERO = 0x01  # acc = 0  (fast path for the common LDA_IMM 0)
    LDA_REG = 0x02   # acc = R[r]
    LDA_VAR = 0x03   # acc = vars[idx]

    # Stores
    STA_REG = 0x10   # R[r] = acc
    STA_VAR = 0x11   # vars[idx] = acc

    # Arithmetic — typed variant has 1 operand (r), untyped has 2 (r, slot)
    ADD = 0x20       # acc = (acc + R[r]) % 256
    SUB = 0x21       # acc = (acc - R[r]) % 256
    MUL = 0x22       # acc = (acc * R[r]) % 256
    DIV = 0x23       # acc = acc // R[r]  (halt if R[r]==0)
    MOD = 0x24       # acc = acc % R[r]   (halt if R[r]==0)
    ADD_IMM = 0x25   # acc = (acc + imm8) % 256  (fast path for n+1 style)
    SUB_IMM = 0x26   # acc = (acc - imm8) % 256

    # Bitwise — always 1 operand (r or imm8), never slotted
    AND = 0x30       # acc = acc & R[r]
    OR = 0x31        # acc = acc | R[r]
    XOR = 0x32       # acc = acc ^ R[r]
    NOT = 0x33       # acc = (~acc) & 0xFF
    SHL = 0x34       # acc = (acc << R[r]) & 0xFF
    SHR = 0x35       # acc = acc >> R[r]  (logical, zero fill)
    AND_IMM = 0x36   # acc = acc & imm8  (nibble masking fast path)

    # Comparisons — typed: 1 operand (r); untyped: 2 operands (r, slot)
    EQ = 0x40        # acc = 1 if acc == R[r] else 0
    NEQ = 0x41       # acc = 1 if acc != R[r] else 0
    LT = 0x42        # acc = 1 if acc < R[r] else 0
    LTE = 0x43       # acc = 1 if acc <= R[r] else 0
    GT = 0x44        # acc = 1 if acc > R[r] else 0
    GTE = 0x45       # acc = 1 if acc >= R[r] else 0

    # Logical helpers (short-circuit && and || use JZ/JNZ instead)
    LOGICAL_NOT = 0x50   # acc = 0 if acc != 0 else 1
    LOGICAL_AND = 0x51   # acc = 0 if acc == 0 else (1 if R[r] != 0 else 0)
    LOGICAL_OR = 0x52    # acc = 1 if acc != 0 else (1 if R[r] != 0 else 0)

    # Control flow — operand is signed 16-bit offset (stored as Python int)
    JMP = 0x60       # ip += offset
    JZ = 0x61        # if acc == 0: ip += offset
    JNZ = 0x62       # if acc != 0: ip += offset
    JMP_LOOP = 0x63  # like JMP but marks a loop back-edge for the VM hot counter

    # Function calls
    CALL = 0x70      # operands: [func_idx, argc, slot]
    RET = 0x71       # return acc to caller

    # I/O
    IO_IN = 0x80     # acc = read_io_port()
    IO_OUT = 0x81    # write_io_port(acc)

    # VM control
    HALT = 0xFF      # stop execution


@dataclass
class Instruction:
    """One VM instruction.

    ``opcode`` is a byte from the Op namespace.

    ``operands`` is a list of 0–3 integers:
      - Arithmetic/comparison typed: [r]
      - Arithmetic/comparison untyped: [r, slot]
      - Bitwise: [r]
      - Jumps: [offset]  (signed int, not restricted to 16 bits in Python)
      - CALL: [func_idx, argc, slot]
      - Others: []

    Mutable so jump offsets can be patched after the target is known.
    """

    opcode: int
    operands: list[int] = field(default_factory=list)


@dataclass
class CodeObject:
    """Compiled form of one function or the top-level ``<main>`` program.

    ``name``         — function name or ``"<main>"`` for the top-level program.
    ``params``       — parameter name list (mirrors the AST FnDecl).
    ``instructions`` — the bytecode sequence.
    ``constants``    — u8 constant pool (deduplication opportunity for the JIT).
    ``var_names``    — variable name pool; ``LDA_VAR i`` accesses ``vars[i]``.
    ``functions``    — sub-CodeObjects; ``CALL func_idx`` indexes this list.
    ``register_count``       — max registers used (set after compilation).
    ``feedback_slot_count``  — size of feedback vector; 0 for FULLY_TYPED fns.
    ``type_status``          — FULLY_TYPED | PARTIALLY_TYPED | UNTYPED.
    ``immediate_jit_eligible`` — True iff type_status == FULLY_TYPED.
    ``source_map``   — list of (instruction_index, line, column) triples.
    """

    name: str
    params: list[str]
    instructions: list[Instruction] = field(default_factory=list)
    constants: list[int] = field(default_factory=list)
    var_names: list[str] = field(default_factory=list)
    functions: list[CodeObject] = field(default_factory=list)
    register_count: int = 0
    feedback_slot_count: int = 0
    type_status: FunctionTypeStatus = FunctionTypeStatus.UNTYPED
    immediate_jit_eligible: bool = False
    source_map: list[tuple[int, int, int]] = field(default_factory=list)
