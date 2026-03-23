"""Opcodes and Instructions — the vocabulary of GPU core programs.

=== What is an Opcode? ===

An opcode (operation code) is a number or name that tells the processor what
to do. It's like a verb in a sentence:

    English:  "Add the first two numbers and store in the third"
    Assembly: FADD R2, R0, R1

The opcode is FADD. The registers R0, R1, R2 are the operands.

=== Instruction Representation ===

Real GPU hardware represents instructions as binary words (32 or 64 bits of
1s and 0s packed together). But at this layer — the processing element
simulator — we use a structured Python dataclass instead:

    Binary (real hardware): 01001000_00000010_00000000_00000001
    Our representation:     Instruction(opcode=FADD, rd=2, rs1=0, rs2=1)

Why? Because binary encoding is the job of the *assembler* layer above us.
The processing element receives already-decoded instructions from the
instruction cache. We're simulating what happens *after* decode.

=== The Instruction Set ===

Our GenericISA has 16 opcodes organized into four categories:

    Arithmetic:  FADD, FSUB, FMUL, FFMA, FNEG, FABS  (6 opcodes)
    Memory:      LOAD, STORE                           (2 opcodes)
    Data move:   MOV, LIMM                             (2 opcodes)
    Control:     BEQ, BLT, BNE, JMP, NOP, HALT         (6 opcodes)

This is deliberately minimal. Real ISAs have hundreds of opcodes, but these
16 are enough to write any floating-point program (they're Turing-complete
when combined with branches and memory).

=== Helper Constructors ===

Writing programs as raw Instruction(...) calls is verbose. The helper
functions (fadd, fmul, ffma, load, store, limm, halt, etc.) make programs
readable:

    # Without helpers (verbose):
    program = [
        Instruction(Opcode.LIMM, rd=0, immediate=2.0),
        Instruction(Opcode.LIMM, rd=1, immediate=3.0),
        Instruction(Opcode.FMUL, rd=2, rs1=0, rs2=1),
        Instruction(Opcode.HALT),
    ]

    # With helpers (clean):
    program = [
        limm(0, 2.0),
        limm(1, 3.0),
        fmul(2, 0, 1),
        halt(),
    ]
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class Opcode(Enum):
    """The set of operations a GPU core can perform.

    Organized by category:

    Floating-point arithmetic (uses fp-arithmetic package):
        FADD  — add two registers
        FSUB  — subtract two registers
        FMUL  — multiply two registers
        FFMA  — fused multiply-add (three source registers)
        FNEG  — negate a register
        FABS  — absolute value of a register

    Memory operations:
        LOAD  — load float from memory into register
        STORE — store register value to memory

    Data movement:
        MOV   — copy one register to another
        LIMM  — load an immediate (literal) float value

    Control flow:
        BEQ   — branch if equal
        BLT   — branch if less than
        BNE   — branch if not equal
        JMP   — unconditional jump
        NOP   — no operation
        HALT  — stop execution
    """

    # Arithmetic
    FADD = "fadd"
    FSUB = "fsub"
    FMUL = "fmul"
    FFMA = "ffma"
    FNEG = "fneg"
    FABS = "fabs"

    # Memory
    LOAD = "load"
    STORE = "store"

    # Data movement
    MOV = "mov"
    LIMM = "limm"

    # Control flow
    BEQ = "beq"
    BLT = "blt"
    BNE = "bne"
    JMP = "jmp"
    NOP = "nop"
    HALT = "halt"


@dataclass(frozen=True)
class Instruction:
    """A single GPU core instruction.

    This is a structured representation of an instruction, not a binary
    encoding. It contains all the information needed to execute the
    instruction: the opcode and up to four operands.

    Fields:
        opcode:    What operation to perform (see Opcode enum).
        rd:        Destination register index (0-255).
        rs1:       First source register index (0-255).
        rs2:       Second source register index (0-255).
        rs3:       Third source register (used only by FFMA).
        immediate: A literal float value (used by LIMM, branch offsets,
                   memory offsets). For branches, this is the number of
                   instructions to skip (positive = forward, negative = back).
    """

    opcode: Opcode
    rd: int = 0
    rs1: int = 0
    rs2: int = 0
    rs3: int = 0
    immediate: float = 0.0

    def __repr__(self) -> str:
        """Pretty-print the instruction in assembly-like syntax."""
        op = self.opcode.value.upper()
        match self.opcode:
            case Opcode.FADD | Opcode.FSUB | Opcode.FMUL:
                return f"{op} R{self.rd}, R{self.rs1}, R{self.rs2}"
            case Opcode.FFMA:
                return (
                    f"{op} R{self.rd}, R{self.rs1}, "
                    f"R{self.rs2}, R{self.rs3}"
                )
            case Opcode.FNEG | Opcode.FABS:
                return f"{op} R{self.rd}, R{self.rs1}"
            case Opcode.LOAD:
                return f"{op} R{self.rd}, [R{self.rs1}+{self.immediate}]"
            case Opcode.STORE:
                return f"{op} [R{self.rs1}+{self.immediate}], R{self.rs2}"
            case Opcode.MOV:
                return f"{op} R{self.rd}, R{self.rs1}"
            case Opcode.LIMM:
                return f"{op} R{self.rd}, {self.immediate}"
            case Opcode.BEQ | Opcode.BLT | Opcode.BNE:
                sign = "+" if self.immediate >= 0 else ""
                return (
                    f"{op} R{self.rs1}, R{self.rs2}, "
                    f"{sign}{int(self.immediate)}"
                )
            case Opcode.JMP:
                return f"{op} {int(self.immediate)}"
            case Opcode.NOP:
                return "NOP"
            case Opcode.HALT:
                return "HALT"
            case _:
                return f"{op} rd={self.rd} rs1={self.rs1} rs2={self.rs2}"


# ---------------------------------------------------------------------------
# Helper constructors — make programs readable
# ---------------------------------------------------------------------------


def fadd(rd: int, rs1: int, rs2: int) -> Instruction:
    """FADD Rd, Rs1, Rs2 — floating-point addition: Rd = Rs1 + Rs2."""
    return Instruction(Opcode.FADD, rd=rd, rs1=rs1, rs2=rs2)


def fsub(rd: int, rs1: int, rs2: int) -> Instruction:
    """FSUB Rd, Rs1, Rs2 — floating-point subtraction: Rd = Rs1 - Rs2."""
    return Instruction(Opcode.FSUB, rd=rd, rs1=rs1, rs2=rs2)


def fmul(rd: int, rs1: int, rs2: int) -> Instruction:
    """FMUL Rd, Rs1, Rs2 — floating-point multiplication: Rd = Rs1 × Rs2."""
    return Instruction(Opcode.FMUL, rd=rd, rs1=rs1, rs2=rs2)


def ffma(rd: int, rs1: int, rs2: int, rs3: int) -> Instruction:
    """FFMA Rd, Rs1, Rs2, Rs3 — fused multiply-add: Rd = Rs1 × Rs2 + Rs3."""
    return Instruction(Opcode.FFMA, rd=rd, rs1=rs1, rs2=rs2, rs3=rs3)


def fneg(rd: int, rs1: int) -> Instruction:
    """FNEG Rd, Rs1 — negate: Rd = -Rs1."""
    return Instruction(Opcode.FNEG, rd=rd, rs1=rs1)


def fabs(rd: int, rs1: int) -> Instruction:
    """FABS Rd, Rs1 — absolute value: Rd = |Rs1|."""
    return Instruction(Opcode.FABS, rd=rd, rs1=rs1)


def load(rd: int, rs1: int, offset: float = 0.0) -> Instruction:
    """LOAD Rd, [Rs1+offset] — load float from memory into register."""
    return Instruction(Opcode.LOAD, rd=rd, rs1=rs1, immediate=offset)


def store(rs1: int, rs2: int, offset: float = 0.0) -> Instruction:
    """STORE [Rs1+offset], Rs2 — store register value to memory."""
    return Instruction(Opcode.STORE, rs1=rs1, rs2=rs2, immediate=offset)


def mov(rd: int, rs1: int) -> Instruction:
    """MOV Rd, Rs1 — copy register: Rd = Rs1."""
    return Instruction(Opcode.MOV, rd=rd, rs1=rs1)


def limm(rd: int, value: float) -> Instruction:
    """LIMM Rd, value — load immediate float: Rd = value."""
    return Instruction(Opcode.LIMM, rd=rd, immediate=value)


def beq(rs1: int, rs2: int, offset: int) -> Instruction:
    """BEQ Rs1, Rs2, offset — branch if equal."""
    return Instruction(Opcode.BEQ, rs1=rs1, rs2=rs2, immediate=float(offset))


def blt(rs1: int, rs2: int, offset: int) -> Instruction:
    """BLT Rs1, Rs2, offset — branch if less than."""
    return Instruction(Opcode.BLT, rs1=rs1, rs2=rs2, immediate=float(offset))


def bne(rs1: int, rs2: int, offset: int) -> Instruction:
    """BNE Rs1, Rs2, offset — branch if not equal."""
    return Instruction(Opcode.BNE, rs1=rs1, rs2=rs2, immediate=float(offset))


def jmp(target: int) -> Instruction:
    """JMP target — unconditional jump to absolute address."""
    return Instruction(Opcode.JMP, immediate=float(target))


def nop() -> Instruction:
    """NOP — no operation, advance program counter."""
    return Instruction(Opcode.NOP)


def halt() -> Instruction:
    """HALT — stop execution."""
    return Instruction(Opcode.HALT)
