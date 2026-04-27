"""
arm1_simulator — ARM1 (ARMv1) Behavioral Instruction Set Simulator
=========================================================================

The ARM1 was designed by Sophie Wilson and Steve Furber at Acorn Computers
in Cambridge, UK. First silicon powered on April 26, 1985 — and worked
correctly on the very first attempt. This module provides a complete
behavioral simulator for the ARM1 processor.

Architecture Summary
--------------------

- 32-bit RISC processor, 25,000 transistors
- 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
- R15 = combined Program Counter + Status Register
- 3-stage pipeline: Fetch -> Decode -> Execute
- Every instruction is conditional (4-bit condition code)
- Inline barrel shifter on Operand2 (shift for free)
- No multiply instruction (added in ARM2)
- No cache, no MMU
- 26-bit address space (64 MiB)

R15: The Combined PC + Status Register
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

ARMv1's most distinctive architectural feature is that the program counter
and processor status flags share a single 32-bit register (R15)::

    Bit 31: N (Negative)     Bit 27: I (IRQ disable)
    Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
    Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
    Bit 28: V (Overflow)     Bits 1:0: Processor Mode

Because instructions are 32-bit and word-aligned, the bottom 2 bits of
the address are always 0 — so they're repurposed for the mode bits.

Usage
-----

    >>> cpu = ARM1(memory_size=1024*1024)
    >>> cpu.load_program(machine_code, 0)
    >>> traces = cpu.run(max_steps=10000)

For the gate-level version that routes every operation through logic gates,
see the arm1_gatelevel package.
"""

from __future__ import annotations

import ctypes
import struct
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

from arm1_simulator.state import ARM1State

if TYPE_CHECKING:
    from simulator_protocol import ExecutionResult, StepTrace

__version__ = "0.1.0"


# =========================================================================
# Processor Modes
# =========================================================================
#
# The ARM1 supports 4 processor modes. Each mode has its own banked copies
# of certain registers, allowing fast context switching (especially for FIQ,
# which banks 7 registers to avoid saving/restoring them in the handler).
#
#   Mode  M1:M0  Banked Registers
#   ----  -----  ----------------
#   USR   0b00   (none -- base set)
#   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
#   IRQ   0b10   R13_irq, R14_irq
#   SVC   0b11   R13_svc, R14_svc

MODE_USR: int = 0  # User mode — normal program execution
MODE_FIQ: int = 1  # Fast Interrupt — banks R8-R14 for zero-overhead handlers
MODE_IRQ: int = 2  # Normal Interrupt — banks R13-R14
MODE_SVC: int = 3  # Supervisor — entered via SWI or Reset


def mode_string(mode: int) -> str:
    """Return a human-readable name for a processor mode."""
    return {MODE_USR: "USR", MODE_FIQ: "FIQ", MODE_IRQ: "IRQ", MODE_SVC: "SVC"}.get(
        mode, "???"
    )


# =========================================================================
# Condition Codes
# =========================================================================
#
# Every ARM instruction has a 4-bit condition code in bits 31:28.
# The instruction only executes if the condition is met. This is ARM's
# signature feature — even data processing and load/store instructions
# can be conditional, eliminating many branches.
#
#   Code  Suffix  Meaning                  Test
#   ----  ------  -------                  ----
#   0000  EQ      Equal                    Z == 1
#   0001  NE      Not Equal                Z == 0
#   0010  CS/HS   Carry Set / Unsigned >=  C == 1
#   0011  CC/LO   Carry Clear / Unsigned < C == 0
#   0100  MI      Minus (Negative)         N == 1
#   0101  PL      Plus (Non-negative)      N == 0
#   0110  VS      Overflow Set             V == 1
#   0111  VC      Overflow Clear           V == 0
#   1000  HI      Unsigned Higher          C == 1 AND Z == 0
#   1001  LS      Unsigned Lower or Same   C == 0 OR  Z == 1
#   1010  GE      Signed >=                N == V
#   1011  LT      Signed <                 N != V
#   1100  GT      Signed >                 Z == 0 AND N == V
#   1101  LE      Signed <=                Z == 1 OR  N != V
#   1110  AL      Always                   true
#   1111  NV      Never (reserved)         false

COND_EQ: int = 0x0  # Equal — Z set
COND_NE: int = 0x1  # Not equal — Z clear
COND_CS: int = 0x2  # Carry set / unsigned higher or same
COND_CC: int = 0x3  # Carry clear / unsigned lower
COND_MI: int = 0x4  # Minus / negative — N set
COND_PL: int = 0x5  # Plus / positive or zero — N clear
COND_VS: int = 0x6  # Overflow set
COND_VC: int = 0x7  # Overflow clear
COND_HI: int = 0x8  # Unsigned higher — C set AND Z clear
COND_LS: int = 0x9  # Unsigned lower or same — C clear OR Z set
COND_GE: int = 0xA  # Signed greater or equal — N == V
COND_LT: int = 0xB  # Signed less than — N != V
COND_GT: int = 0xC  # Signed greater than — Z clear AND N == V
COND_LE: int = 0xD  # Signed less or equal — Z set OR N != V
COND_AL: int = 0xE  # Always (unconditional)
COND_NV: int = 0xF  # Never (reserved — do not use)

_COND_STRINGS: dict[int, str] = {
    COND_EQ: "EQ",
    COND_NE: "NE",
    COND_CS: "CS",
    COND_CC: "CC",
    COND_MI: "MI",
    COND_PL: "PL",
    COND_VS: "VS",
    COND_VC: "VC",
    COND_HI: "HI",
    COND_LS: "LS",
    COND_GE: "GE",
    COND_LT: "LT",
    COND_GT: "GT",
    COND_LE: "LE",
    COND_AL: "",
    COND_NV: "NV",
}


def cond_string(cond: int) -> str:
    """Return the assembly-language suffix for a condition code."""
    return _COND_STRINGS.get(cond, "??")


# =========================================================================
# ALU Opcodes
# =========================================================================
#
# The ARM1's ALU supports 16 operations, selected by bits 24:21 of a data
# processing instruction. Four of these (TST, TEQ, CMP, CMN) only set flags
# and do not write a result to the destination register.

OP_AND: int = 0x0  # Rd = Rn AND Op2
OP_EOR: int = 0x1  # Rd = Rn XOR Op2
OP_SUB: int = 0x2  # Rd = Rn - Op2
OP_RSB: int = 0x3  # Rd = Op2 - Rn
OP_ADD: int = 0x4  # Rd = Rn + Op2
OP_ADC: int = 0x5  # Rd = Rn + Op2 + Carry
OP_SBC: int = 0x6  # Rd = Rn - Op2 - NOT(Carry)
OP_RSC: int = 0x7  # Rd = Op2 - Rn - NOT(Carry)
OP_TST: int = 0x8  # Rn AND Op2, flags only
OP_TEQ: int = 0x9  # Rn XOR Op2, flags only
OP_CMP: int = 0xA  # Rn - Op2, flags only
OP_CMN: int = 0xB  # Rn + Op2, flags only
OP_ORR: int = 0xC  # Rd = Rn OR Op2
OP_MOV: int = 0xD  # Rd = Op2
OP_BIC: int = 0xE  # Rd = Rn AND NOT(Op2)
OP_MVN: int = 0xF  # Rd = NOT(Op2)

_OP_NAMES: list[str] = [
    "AND", "EOR", "SUB", "RSB", "ADD", "ADC", "SBC", "RSC",
    "TST", "TEQ", "CMP", "CMN", "ORR", "MOV", "BIC", "MVN",
]


def op_string(opcode: int) -> str:
    """Return the mnemonic for an ALU opcode."""
    if 0 <= opcode < 16:
        return _OP_NAMES[opcode]
    return "???"


def is_test_op(opcode: int) -> bool:
    """Return True if the ALU opcode is a test-only operation (TST/TEQ/CMP/CMN).

    These operations only set flags — they do not write a result to
    the destination register.
    """
    return OP_TST <= opcode <= OP_CMN


def is_logical_op(opcode: int) -> bool:
    """Return True if the ALU opcode is a logical operation.

    For logical ops, the C flag comes from the barrel shifter carry-out
    rather than the ALU's adder carry.
    """
    return opcode in (OP_AND, OP_EOR, OP_TST, OP_TEQ, OP_ORR, OP_MOV, OP_BIC, OP_MVN)


# =========================================================================
# Shift Types
# =========================================================================
#
# The barrel shifter supports 4 shift types, encoded in bits 6:5 of the
# operand2 field. The barrel shifter is the ARM1's most distinctive hardware
# feature — it allows one operand to be shifted or rotated FOR FREE as part
# of any data processing instruction.
#
#   00 = LSL (Logical Shift Left):     Fills vacated bits with 0
#   01 = LSR (Logical Shift Right):    Fills vacated bits with 0
#   10 = ASR (Arithmetic Shift Right): Fills vacated bits with sign bit
#   11 = ROR (Rotate Right):           Bits shifted out re-enter at top

SHIFT_LSL: int = 0  # Logical Shift Left
SHIFT_LSR: int = 1  # Logical Shift Right
SHIFT_ASR: int = 2  # Arithmetic Shift Right (sign-extending)
SHIFT_ROR: int = 3  # Rotate Right (ROR #0 encodes RRX)

_SHIFT_NAMES: dict[int, str] = {
    SHIFT_LSL: "LSL",
    SHIFT_LSR: "LSR",
    SHIFT_ASR: "ASR",
    SHIFT_ROR: "ROR",
}


def shift_string(shift_type: int) -> str:
    """Return the mnemonic for a shift type."""
    return _SHIFT_NAMES.get(shift_type, "???")


# =========================================================================
# R15 Bit Positions
# =========================================================================

FLAG_N: int = 1 << 31    # Negative flag
FLAG_Z: int = 1 << 30    # Zero flag
FLAG_C: int = 1 << 29    # Carry flag
FLAG_V: int = 1 << 28    # Overflow flag
FLAG_I: int = 1 << 27    # IRQ disable
FLAG_F: int = 1 << 26    # FIQ disable
PC_MASK: int = 0x03FFFFFC  # Bits 25:2 — the 24-bit PC field
MODE_MASK: int = 0x3       # Bits 1:0 — processor mode

# HaltSWI is the SWI comment field we use as a halt instruction.
# The simulator intercepts SWI with this value to stop execution.
HALT_SWI: int = 0x123456

# Mask for 32-bit arithmetic: Python ints are arbitrary precision,
# so we must mask to 32 bits after every arithmetic operation.
MASK_32: int = 0xFFFFFFFF

# =========================================================================
# Instruction types
# =========================================================================

INST_DATA_PROCESSING: int = 0
INST_LOAD_STORE: int = 1
INST_BLOCK_TRANSFER: int = 2
INST_BRANCH: int = 3
INST_SWI: int = 4
INST_COPROCESSOR: int = 5
INST_UNDEFINED: int = 6


# =========================================================================
# Data Classes
# =========================================================================


@dataclass
class Flags:
    """Represents the ARM1's four condition flags.

    These flags are stored in the top 4 bits of R15 and control
    conditional instruction execution.

    Attributes:
        n: Negative — set when result's bit 31 is 1
        z: Zero — set when result is 0
        c: Carry — set on unsigned overflow or shifter carry-out
        v: Overflow — set on signed overflow
    """

    n: bool = False
    z: bool = False
    c: bool = False
    v: bool = False


@dataclass
class MemoryAccess:
    """Records a single memory read or write.

    Attributes:
        address: The memory address accessed.
        value: The 32-bit value read or written.
    """

    address: int = 0
    value: int = 0


@dataclass
class ALUResult:
    """Holds the output of an ALU operation.

    Attributes:
        result: The 32-bit result.
        n: Negative flag (bit 31 of result).
        z: Zero flag (result == 0).
        c: Carry flag.
        v: Overflow flag.
        write_result: Should the result be written to Rd?
    """

    result: int = 0
    n: bool = False
    z: bool = False
    c: bool = False
    v: bool = False
    write_result: bool = True


@dataclass
class Trace:
    """Records the state change caused by executing one instruction.

    It captures the complete before/after snapshot for debugging and
    cross-language validation.
    """

    address: int = 0
    raw: int = 0
    mnemonic: str = ""
    condition: str = ""
    condition_met: bool = False
    regs_before: list[int] = field(default_factory=lambda: [0] * 16)
    regs_after: list[int] = field(default_factory=lambda: [0] * 16)
    flags_before: Flags = field(default_factory=Flags)
    flags_after: Flags = field(default_factory=Flags)
    memory_reads: list[MemoryAccess] = field(default_factory=list)
    memory_writes: list[MemoryAccess] = field(default_factory=list)


@dataclass
class DecodedInstruction:
    """Holds all fields extracted from a 32-bit ARM instruction.

    The ARM1's instruction decoder takes a 32-bit instruction word and
    extracts the fields that control execution: which operation, which
    registers, what shift, what offset, etc.

    Instruction Classes (classified by bits 27:25)::

        Bits 27:26  Bit 25  Class
        ----------  ------  -----
        00          --      Data Processing / PSR Transfer
        01          --      Single Data Transfer (LDR/STR)
        10          0       Block Data Transfer (LDM/STM)
        10          1       Branch (B/BL)
        11          --      Coprocessor / SWI
    """

    raw: int = 0
    inst_type: int = INST_UNDEFINED

    # Condition (bits 31:28)
    cond: int = 0

    # Data Processing fields
    opcode: int = 0
    s: bool = False
    rn: int = 0
    rd: int = 0
    immediate: bool = False

    # Operand2 — immediate form
    imm8: int = 0
    rotate: int = 0

    # Operand2 — register form
    rm: int = 0
    shift_type: int = 0
    shift_by_reg: bool = False
    shift_imm: int = 0
    rs: int = 0

    # Load/Store fields
    load: bool = False
    byte: bool = False
    pre_index: bool = False
    up: bool = False
    write_back: bool = False
    offset12: int = 0

    # Block Transfer fields
    register_list: int = 0
    force_user: bool = False

    # Branch fields
    link: bool = False
    branch_offset: int = 0

    # SWI fields
    swi_comment: int = 0


# =========================================================================
# Condition Evaluator
# =========================================================================
#
# Every ARM instruction has a 4-bit condition code in bits 31:28. The
# instruction only executes if the condition is satisfied by the current
# flags (N, Z, C, V). This is one of the most distinctive features of the
# ARM architecture — no other RISC architecture of the era made every
# instruction conditional.


def evaluate_condition(cond: int, flags: Flags) -> bool:
    """Test whether the given condition code is satisfied by the current flags.

    This function is the behavioral equivalent of the ARM1's condition
    evaluation hardware — a small block of combinational logic that gates
    instruction execution.

    Condition Truth Table::

        Code  Suffix  Meaning                  Test
        ----  ------  -------                  ----
        0000  EQ      Equal                    Z == 1
        0001  NE      Not Equal                Z == 0
        0010  CS/HS   Carry Set / Unsigned >=  C == 1
        0011  CC/LO   Carry Clear / Unsigned < C == 0
        0100  MI      Minus (Negative)         N == 1
        0101  PL      Plus (Non-negative)      N == 0
        0110  VS      Overflow Set             V == 1
        0111  VC      Overflow Clear           V == 0
        1000  HI      Unsigned Higher          C == 1 AND Z == 0
        1001  LS      Unsigned Lower or Same   C == 0 OR  Z == 1
        1010  GE      Signed >=                N == V
        1011  LT      Signed <                 N != V
        1100  GT      Signed >                 Z == 0 AND N == V
        1101  LE      Signed <=                Z == 1 OR  N != V
        1110  AL      Always                   true
        1111  NV      Never (reserved)         false
    """
    if cond == COND_EQ:
        return flags.z
    if cond == COND_NE:
        return not flags.z
    if cond == COND_CS:
        return flags.c
    if cond == COND_CC:
        return not flags.c
    if cond == COND_MI:
        return flags.n
    if cond == COND_PL:
        return not flags.n
    if cond == COND_VS:
        return flags.v
    if cond == COND_VC:
        return not flags.v
    if cond == COND_HI:
        return flags.c and not flags.z
    if cond == COND_LS:
        return not flags.c or flags.z
    if cond == COND_GE:
        return flags.n == flags.v
    if cond == COND_LT:
        return flags.n != flags.v
    if cond == COND_GT:
        return not flags.z and (flags.n == flags.v)
    if cond == COND_LE:
        return flags.z or (flags.n != flags.v)
    if cond == COND_AL:
        return True
    if cond == COND_NV:
        return False
    return False


# =========================================================================
# Barrel Shifter
# =========================================================================
#
# The barrel shifter is the ARM1's most distinctive hardware feature. On the
# real chip, it was implemented as a 32x32 crossbar network of pass
# transistors — each of the 32 output bits could be connected to any of the
# 32 input bits. This allowed shifting and rotating a value by any amount
# in a single clock cycle, at zero additional cost.


def barrel_shift(
    value: int, shift_type: int, amount: int, carry_in: bool, by_register: bool
) -> tuple[int, bool]:
    """Apply a shift operation to a 32-bit value.

    Parameters:
        value:       the 32-bit input (from register Rm)
        shift_type:  0=LSL, 1=LSR, 2=ASR, 3=ROR
        amount:      number of positions to shift (0-31 for immediate encoding)
        carry_in:    current carry flag (used for RRX and amount=0 cases)
        by_register: True if the shift amount comes from a register

    Returns:
        (result, carry_out) — the shifted value and the carry output
    """
    value = value & MASK_32

    # When shifting by a register value, if the amount is 0, the value
    # passes through unchanged and the carry flag is unaffected.
    if by_register and amount == 0:
        return value, carry_in

    if shift_type == SHIFT_LSL:
        return _shift_lsl(value, amount, carry_in, by_register)
    if shift_type == SHIFT_LSR:
        return _shift_lsr(value, amount, carry_in, by_register)
    if shift_type == SHIFT_ASR:
        return _shift_asr(value, amount, carry_in, by_register)
    if shift_type == SHIFT_ROR:
        return _shift_ror(value, amount, carry_in, by_register)
    return value, carry_in


def _shift_lsl(
    value: int, amount: int, carry_in: bool, by_register: bool
) -> tuple[int, bool]:
    """Logical Shift Left.

    Before (LSL #3)::

        [b31 b30 ... b3 b2 b1 b0]

    After::

        [b28 b27 ... b0  0  0  0]

    Carry out: b29 (the last bit shifted out).

    Special case: LSL #0 means "no shift" — value unchanged, carry unchanged.
    """
    if amount == 0:
        return value, carry_in
    if amount >= 32:
        if amount == 32:
            return 0, (value & 1) != 0
        return 0, False
    carry = ((value >> (32 - amount)) & 1) != 0
    return (value << amount) & MASK_32, carry


def _shift_lsr(
    value: int, amount: int, carry_in: bool, by_register: bool
) -> tuple[int, bool]:
    """Logical Shift Right.

    Special case: immediate LSR #0 encodes LSR #32 (result = 0, carry = bit 31).
    """
    if amount == 0 and not by_register:
        # Immediate LSR #0 encodes LSR #32
        return 0, (value >> 31) != 0
    if amount == 0:
        return value, carry_in
    if amount >= 32:
        if amount == 32:
            return 0, (value >> 31) != 0
        return 0, False
    carry = ((value >> (amount - 1)) & 1) != 0
    return (value >> amount) & MASK_32, carry


def _shift_asr(
    value: int, amount: int, carry_in: bool, by_register: bool
) -> tuple[int, bool]:
    """Arithmetic Shift Right (sign-extending).

    The sign bit (bit 31) is replicated into the vacated positions.
    This preserves the sign of a two's complement number.

    Special case: immediate ASR #0 encodes ASR #32:
        - If bit 31 = 0: result = 0x00000000, carry = 0
        - If bit 31 = 1: result = 0xFFFFFFFF, carry = 1
    """
    sign_bit = (value >> 31) != 0

    if amount == 0 and not by_register:
        # Immediate ASR #0 encodes ASR #32
        if sign_bit:
            return MASK_32, True
        return 0, False
    if amount == 0:
        return value, carry_in
    if amount >= 32:
        if sign_bit:
            return MASK_32, True
        return 0, False

    # Arithmetic right shift: use ctypes.c_int32 for sign extension
    signed = ctypes.c_int32(value).value
    result = (signed >> amount) & MASK_32
    carry = ((value >> (amount - 1)) & 1) != 0
    return result, carry


def _shift_ror(
    value: int, amount: int, carry_in: bool, by_register: bool
) -> tuple[int, bool]:
    """Rotate Right.

    Special case: immediate ROR #0 encodes RRX (Rotate Right Extended):
    33-bit rotation through carry flag. Old carry -> bit 31, old bit 0 -> new carry.
    """
    if amount == 0 and not by_register:
        # RRX — Rotate Right Extended (33-bit rotation through carry)
        carry = (value & 1) != 0
        result = value >> 1
        if carry_in:
            result |= 0x80000000
        return result & MASK_32, carry
    if amount == 0:
        return value, carry_in

    # Normalize rotation amount to 0-31
    amount = amount & 31
    if amount == 0:
        # ROR by 32 (or multiple of 32): value unchanged, carry = bit 31
        return value, (value >> 31) != 0

    result = ((value >> amount) | (value << (32 - amount))) & MASK_32
    carry = ((result >> 31) & 1) != 0
    return result, carry


def decode_immediate(imm8: int, rotate: int) -> tuple[int, bool]:
    """Decode a rotated immediate value from the Operand2 field.

    The encoding packs a wide range of constants into 12 bits:
        - Bits 7:0:   8-bit immediate value
        - Bits 11:8:  4-bit rotation amount (actual rotation = 2 x this value)

    The 8-bit value is rotated right by an even number of positions (0, 2, ..., 30).

    Examples::

        imm8=0xFF, rotate=0  -> 0x000000FF (255)
        imm8=0xFF, rotate=4  -> 0xFF000000 (rotated right by 8)
        imm8=0x01, rotate=1  -> 0x40000000 (1 rotated right by 2)
    """
    rotate_amount = rotate * 2
    if rotate_amount == 0:
        return imm8, False  # carry is unchanged (return False as default)
    value = ((imm8 >> rotate_amount) | (imm8 << (32 - rotate_amount))) & MASK_32
    carry_out = (value >> 31) != 0
    return value, carry_out


# =========================================================================
# ALU
# =========================================================================
#
# The ARM1's ALU performs 16 operations selected by a 4-bit opcode. It takes
# two 32-bit inputs (Rn and the barrel-shifted Operand2) and produces a
# 32-bit result plus four condition flags (N, Z, C, V).
#
# Flag computation:
#
# Arithmetic (ADD, SUB, ADC, SBC, RSB, RSC, CMP, CMN):
#   N = result bit 31
#   Z = result == 0
#   C = carry out from the 32-bit adder
#   V = signed overflow (carry into bit 31 != carry out of bit 31)
#
# Logical (AND, EOR, TST, TEQ, ORR, MOV, BIC, MVN):
#   N = result bit 31
#   Z = result == 0
#   C = carry out from the barrel shifter (not from the ALU)
#   V = unchanged (the ALU does not modify V for logical ops)
#
# Subtraction via addition:
#   A - B = A + NOT(B) + 1
# This means SUB sets carry=1 when there is NO borrow.


def add32(a: int, b: int, carry_in: bool) -> tuple[int, bool, bool]:
    """Perform a 32-bit addition with carry-in, returning (result, carry, overflow).

    We compute this using 64-bit arithmetic for clarity. The real ARM1 uses
    a 32-stage ripple-carry adder.

    Overflow detection: both operands have the same sign, but the result has
    a different sign.
    """
    a = a & MASK_32
    b = b & MASK_32
    cin = 1 if carry_in else 0
    total = a + b + cin
    result = total & MASK_32
    carry = (total >> 32) != 0

    # Overflow: ((a ^ result) & (b ^ result)) >> 31
    overflow = (((a ^ result) & (b ^ result)) >> 31) != 0
    return result, carry, overflow


def alu_execute(
    opcode: int,
    a: int,
    b: int,
    carry_in: bool,
    shifter_carry: bool,
    old_v: bool,
) -> ALUResult:
    """Perform one of the 16 ALU operations.

    Parameters:
        opcode:        4-bit ALU operation (0x0=AND ... 0xF=MVN)
        a:             first operand (value of Rn)
        b:             second operand (barrel-shifted Operand2)
        carry_in:      current carry flag (used by ADC, SBC, RSC)
        shifter_carry: carry output from the barrel shifter (for logical ops)
        old_v:         current overflow flag (preserved for logical ops)

    Returns:
        An ALUResult with the computed result and flags.
    """
    a = a & MASK_32
    b = b & MASK_32
    write_result = not is_test_op(opcode)

    result: int = 0
    carry: bool = False
    overflow: bool = False

    # -- Logical operations -------------------------------------------------
    # C flag comes from the barrel shifter, V flag is preserved.

    if opcode in (OP_AND, OP_TST):
        result = a & b
        carry = shifter_carry
        overflow = old_v

    elif opcode in (OP_EOR, OP_TEQ):
        result = a ^ b
        carry = shifter_carry
        overflow = old_v

    elif opcode == OP_ORR:
        result = a | b
        carry = shifter_carry
        overflow = old_v

    elif opcode == OP_MOV:
        result = b
        carry = shifter_carry
        overflow = old_v

    elif opcode == OP_BIC:
        result = a & (~b & MASK_32)
        carry = shifter_carry
        overflow = old_v

    elif opcode == OP_MVN:
        result = ~b & MASK_32
        carry = shifter_carry
        overflow = old_v

    # -- Arithmetic operations -----------------------------------------------
    # C flag comes from the adder carry-out, V flag detects signed overflow.

    elif opcode in (OP_ADD, OP_CMN):
        # A + B
        result, carry, overflow = add32(a, b, False)

    elif opcode == OP_ADC:
        # A + B + C
        result, carry, overflow = add32(a, b, carry_in)

    elif opcode in (OP_SUB, OP_CMP):
        # A - B = A + NOT(B) + 1
        result, carry, overflow = add32(a, ~b & MASK_32, True)

    elif opcode == OP_SBC:
        # A - B - NOT(C) = A + NOT(B) + C
        result, carry, overflow = add32(a, ~b & MASK_32, carry_in)

    elif opcode == OP_RSB:
        # B - A = B + NOT(A) + 1
        result, carry, overflow = add32(b, ~a & MASK_32, True)

    elif opcode == OP_RSC:
        # B - A - NOT(C) = B + NOT(A) + C
        result, carry, overflow = add32(b, ~a & MASK_32, carry_in)

    result = result & MASK_32

    return ALUResult(
        result=result,
        n=(result >> 31) != 0,
        z=result == 0,
        c=carry,
        v=overflow,
        write_result=write_result,
    )


# =========================================================================
# Instruction Decoder
# =========================================================================


def decode(instruction: int) -> DecodedInstruction:
    """Extract all fields from a 32-bit ARM instruction.

    This is the behavioral equivalent of the ARM1's PLA decoder. The real
    hardware uses combinational gate trees to extract these fields in
    parallel. We do the same thing with bit masking and shifting.
    """
    instruction = instruction & MASK_32
    d = DecodedInstruction(
        raw=instruction,
        cond=(instruction >> 28) & 0xF,
    )

    # Classify by bits 27:25
    bits2726 = (instruction >> 26) & 0x3
    bit25 = (instruction >> 25) & 0x1

    if bits2726 == 0:
        d.inst_type = INST_DATA_PROCESSING
        _decode_data_processing(d, instruction)
    elif bits2726 == 1:
        d.inst_type = INST_LOAD_STORE
        _decode_load_store(d, instruction)
    elif bits2726 == 2 and bit25 == 0:
        d.inst_type = INST_BLOCK_TRANSFER
        _decode_block_transfer(d, instruction)
    elif bits2726 == 2 and bit25 == 1:
        d.inst_type = INST_BRANCH
        _decode_branch(d, instruction)
    elif bits2726 == 3:
        if (instruction >> 24) & 0xF == 0xF:
            d.inst_type = INST_SWI
            d.swi_comment = instruction & 0x00FFFFFF
        else:
            d.inst_type = INST_COPROCESSOR
    else:
        d.inst_type = INST_UNDEFINED

    return d


def _decode_data_processing(d: DecodedInstruction, inst: int) -> None:
    """Decode data processing instruction fields."""
    d.immediate = ((inst >> 25) & 1) == 1
    d.opcode = (inst >> 21) & 0xF
    d.s = ((inst >> 20) & 1) == 1
    d.rn = (inst >> 16) & 0xF
    d.rd = (inst >> 12) & 0xF

    if d.immediate:
        d.imm8 = inst & 0xFF
        d.rotate = (inst >> 8) & 0xF
    else:
        d.rm = inst & 0xF
        d.shift_type = (inst >> 5) & 0x3
        d.shift_by_reg = ((inst >> 4) & 1) == 1
        if d.shift_by_reg:
            d.rs = (inst >> 8) & 0xF
        else:
            d.shift_imm = (inst >> 7) & 0x1F


def _decode_load_store(d: DecodedInstruction, inst: int) -> None:
    """Decode single data transfer instruction fields.

    Note: for LDR/STR, I=1 means REGISTER offset (opposite of data processing).
    """
    d.immediate = ((inst >> 25) & 1) == 1
    d.pre_index = ((inst >> 24) & 1) == 1
    d.up = ((inst >> 23) & 1) == 1
    d.byte = ((inst >> 22) & 1) == 1
    d.write_back = ((inst >> 21) & 1) == 1
    d.load = ((inst >> 20) & 1) == 1
    d.rn = (inst >> 16) & 0xF
    d.rd = (inst >> 12) & 0xF

    if d.immediate:
        # Register offset
        d.rm = inst & 0xF
        d.shift_type = (inst >> 5) & 0x3
        d.shift_imm = (inst >> 7) & 0x1F
    else:
        d.offset12 = inst & 0xFFF


def _decode_block_transfer(d: DecodedInstruction, inst: int) -> None:
    """Decode block data transfer instruction fields."""
    d.pre_index = ((inst >> 24) & 1) == 1
    d.up = ((inst >> 23) & 1) == 1
    d.force_user = ((inst >> 22) & 1) == 1
    d.write_back = ((inst >> 21) & 1) == 1
    d.load = ((inst >> 20) & 1) == 1
    d.rn = (inst >> 16) & 0xF
    d.register_list = inst & 0xFFFF


def _decode_branch(d: DecodedInstruction, inst: int) -> None:
    """Decode branch instruction fields.

    The 24-bit offset is sign-extended to 32 bits, then shifted left by 2
    (since instructions are word-aligned). This gives a range of +/-32 MiB.
    """
    d.link = ((inst >> 24) & 1) == 1
    offset = inst & 0x00FFFFFF
    # Sign-extend from 24 bits to 32 bits
    if (offset >> 23) != 0:
        offset |= 0xFF000000
    # Shift left by 2 and convert to signed 32-bit
    d.branch_offset = ctypes.c_int32(offset << 2).value


# =========================================================================
# Disassembly
# =========================================================================


def _disasm_reg_list(reg_list: int) -> str:
    """Format a register list bitmap as a comma-separated string."""
    parts: list[str] = []
    for i in range(16):
        if (reg_list >> i) & 1 == 1:
            if i == 15:
                parts.append("PC")
            elif i == 14:
                parts.append("LR")
            elif i == 13:
                parts.append("SP")
            else:
                parts.append(f"R{i}")
    return ", ".join(parts)


def _disasm_operand2(d: DecodedInstruction) -> str:
    """Format the Operand2 field as an assembly string."""
    if d.immediate:
        val, _ = decode_immediate(d.imm8, d.rotate)
        return f"#{val}"
    if not d.shift_by_reg and d.shift_imm == 0 and d.shift_type == SHIFT_LSL:
        return f"R{d.rm}"
    if d.shift_by_reg:
        return f"R{d.rm}, {shift_string(d.shift_type)} R{d.rs}"
    amount = d.shift_imm
    if amount == 0:
        if d.shift_type in (SHIFT_LSR, SHIFT_ASR):
            amount = 32
        elif d.shift_type == SHIFT_ROR:
            return f"R{d.rm}, RRX"
    return f"R{d.rm}, {shift_string(d.shift_type)} #{amount}"


def disassemble(d: DecodedInstruction) -> str:
    """Return a human-readable assembly string for the instruction."""
    cond = cond_string(d.cond)

    if d.inst_type == INST_DATA_PROCESSING:
        op = op_string(d.opcode)
        suf = "S" if (d.s and not is_test_op(d.opcode)) else ""
        op2 = _disasm_operand2(d)
        if d.opcode in (OP_MOV, OP_MVN):
            return f"{op}{cond}{suf} R{d.rd}, {op2}"
        if is_test_op(d.opcode):
            return f"{op}{cond} R{d.rn}, {op2}"
        return f"{op}{cond}{suf} R{d.rd}, R{d.rn}, {op2}"

    if d.inst_type == INST_LOAD_STORE:
        op = "LDR" if d.load else "STR"
        b_suf = "B" if d.byte else ""
        if d.immediate:
            offset = f"R{d.rm}"
            if d.shift_imm != 0:
                offset += f", {shift_string(d.shift_type)} #{d.shift_imm}"
        else:
            offset = f"#{d.offset12}"
        sign = "-" if not d.up else ""
        if d.pre_index:
            wb = "!" if d.write_back else ""
            return f"{op}{cond}{b_suf} R{d.rd}, [R{d.rn}, {sign}{offset}]{wb}"
        return f"{op}{cond}{b_suf} R{d.rd}, [R{d.rn}], {sign}{offset}"

    if d.inst_type == INST_BLOCK_TRANSFER:
        op = "LDM" if d.load else "STM"
        if not d.pre_index and d.up:
            mode = "IA"
        elif d.pre_index and d.up:
            mode = "IB"
        elif not d.pre_index and not d.up:
            mode = "DA"
        else:
            mode = "DB"
        wb = "!" if d.write_back else ""
        regs = _disasm_reg_list(d.register_list)
        return f"{op}{cond}{mode} R{d.rn}{wb}, {{{regs}}}"

    if d.inst_type == INST_BRANCH:
        op = "BL" if d.link else "B"
        return f"{op}{cond} #{d.branch_offset}"

    if d.inst_type == INST_SWI:
        if d.swi_comment == HALT_SWI:
            return f"HLT{cond}"
        return f"SWI{cond} #0x{d.swi_comment:X}"

    if d.inst_type == INST_COPROCESSOR:
        return f"CDP{cond} (undefined)"

    return f"UND{cond} #0x{d.raw:08X}"


# =========================================================================
# Encoding Helpers
# =========================================================================
#
# These functions create ARM instruction words, useful for writing test
# programs without an assembler.


def encode_data_processing(
    cond: int, opcode: int, s: int, rn: int, rd: int, operand2: int
) -> int:
    """Create a data processing instruction word."""
    return (
        (cond << 28)
        | operand2
        | (opcode << 21)
        | (s << 20)
        | (rn << 16)
        | (rd << 12)
    ) & MASK_32


def encode_mov_imm(cond: int, rd: int, imm8: int) -> int:
    """Create a MOV immediate instruction.

    Example: encode_mov_imm(COND_AL, 0, 42) -> MOV R0, #42
    """
    return encode_data_processing(cond, OP_MOV, 0, 0, rd, (1 << 25) | imm8)


def encode_alu_reg(cond: int, opcode: int, s: int, rd: int, rn: int, rm: int) -> int:
    """Create a data processing instruction with a register operand.

    Example: encode_alu_reg(COND_AL, OP_ADD, 1, 0, 1, 2) -> ADDS R0, R1, R2
    """
    return encode_data_processing(cond, opcode, s, rn, rd, rm)


def encode_branch(cond: int, link: bool, offset: int) -> int:
    """Create a Branch or Branch-with-Link instruction.

    offset is in bytes, relative to PC+8.
    """
    inst = (cond << 28) | 0x0A000000
    if link:
        inst |= 0x01000000
    # Offset is in bytes. We encode (offset/4) in 24 bits.
    encoded = (offset >> 2) & 0x00FFFFFF
    inst |= encoded
    return inst & MASK_32


def encode_halt() -> int:
    """Create our pseudo-halt instruction (SWI 0x123456)."""
    return ((COND_AL << 28) | 0x0F000000 | HALT_SWI) & MASK_32


def encode_ldr(cond: int, rd: int, rn: int, offset: int, pre_index: bool) -> int:
    """Create a Load Register instruction with immediate offset."""
    inst = (cond << 28) | 0x04100000  # bits 27:26=01, L=1, I=0 (immediate)
    inst |= rd << 12
    inst |= rn << 16
    if pre_index:
        inst |= 1 << 24  # P bit
    if offset >= 0:
        inst |= 1 << 23  # U bit (add)
        inst |= offset & 0xFFF
    else:
        inst |= (-offset) & 0xFFF
    return inst & MASK_32


def encode_str(cond: int, rd: int, rn: int, offset: int, pre_index: bool) -> int:
    """Create a Store Register instruction with immediate offset."""
    inst = (cond << 28) | 0x04000000  # bits 27:26=01, L=0, I=0 (immediate)
    inst |= rd << 12
    inst |= rn << 16
    if pre_index:
        inst |= 1 << 24
    if offset >= 0:
        inst |= 1 << 23
        inst |= offset & 0xFFF
    else:
        inst |= (-offset) & 0xFFF
    return inst & MASK_32


def encode_ldm(
    cond: int, rn: int, reg_list: int, write_back: bool, mode: str
) -> int:
    """Create a Load Multiple instruction."""
    inst = (cond << 28) | 0x08100000  # bits 27:25=100, L=1
    inst |= rn << 16
    inst |= reg_list & 0xFFFF
    if write_back:
        inst |= 1 << 21
    if mode == "IA":
        inst |= 1 << 23  # P=0, U=1
    elif mode == "IB":
        inst |= 1 << 24  # P=1, U=1
        inst |= 1 << 23
    elif mode == "DA":
        pass  # P=0, U=0
    elif mode == "DB":
        inst |= 1 << 24  # P=1, U=0
    return inst & MASK_32


def encode_stm(
    cond: int, rn: int, reg_list: int, write_back: bool, mode: str
) -> int:
    """Create a Store Multiple instruction."""
    inst = encode_ldm(cond, rn, reg_list, write_back, mode)
    inst &= ~(1 << 20)  # Clear L bit (store, not load)
    return inst & MASK_32


# =========================================================================
# ARM1 CPU Simulator
# =========================================================================


class ARM1:
    """The top-level ARM1 CPU simulator.

    This implements the complete ARMv1 instruction set as designed by Sophie
    Wilson and Steve Furber at Acorn Computers in 1984-1985.

    Architecture:
        - 32-bit RISC processor, 25,000 transistors
        - 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
        - R15 = combined Program Counter + Status Register
        - 3-stage pipeline: Fetch -> Decode -> Execute

    Register File Layout::

        [0..15]  = R0-R15 (User/System mode base registers)
        [16..22] = R8_fiq, R9_fiq, R10_fiq, R11_fiq, R12_fiq, R13_fiq, R14_fiq
        [23..24] = R13_irq, R14_irq
        [25..26] = R13_svc, R14_svc

    Usage::

        cpu = ARM1(memory_size=1024*1024)  # 1 MiB memory
        cpu.load_program(machine_code, 0)
        traces = cpu.run(max_steps=10000)
    """

    def __init__(self, memory_size: int = 1024 * 1024) -> None:
        if memory_size <= 0:
            memory_size = 1024 * 1024  # 1 MiB default
        self._regs: list[int] = [0] * 27
        self._memory: bytearray = bytearray(memory_size)
        self._halted: bool = False
        self.reset()

    def reset(self) -> None:
        """Restore the CPU to its power-on state.

        - Supervisor mode (SVC)
        - IRQs and FIQs disabled
        - PC = 0
        - All flags cleared
        """
        self._regs = [0] * 27
        # Set R15: SVC mode, IRQ/FIQ disabled
        self._regs[15] = (FLAG_I | FLAG_F | MODE_SVC) & MASK_32
        self._halted = False

    # ── Register access ────────────────────────────────────────────────

    def read_register(self, index: int) -> int:
        """Read a register (R0-R15), respecting mode banking."""
        return self._regs[self._physical_reg(index)]

    def write_register(self, index: int, value: int) -> None:
        """Write a register (R0-R15), respecting mode banking."""
        self._regs[self._physical_reg(index)] = value & MASK_32

    def _physical_reg(self, index: int) -> int:
        """Map a logical register index (0-15) to a physical register index (0-26)."""
        mode = self.mode

        if mode == MODE_FIQ and 8 <= index <= 14:
            return 16 + (index - 8)
        if mode == MODE_IRQ and 13 <= index <= 14:
            return 23 + (index - 13)
        if mode == MODE_SVC and 13 <= index <= 14:
            return 25 + (index - 13)
        return index

    @property
    def pc(self) -> int:
        """Return the current program counter (26-bit address)."""
        return self._regs[15] & PC_MASK

    @pc.setter
    def pc(self, addr: int) -> None:
        """Set the program counter portion of R15 without changing flags/mode."""
        self._regs[15] = (self._regs[15] & ~PC_MASK & MASK_32) | (addr & PC_MASK)

    @property
    def flags(self) -> Flags:
        """Return the current condition flags."""
        r15 = self._regs[15]
        return Flags(
            n=(r15 & FLAG_N) != 0,
            z=(r15 & FLAG_Z) != 0,
            c=(r15 & FLAG_C) != 0,
            v=(r15 & FLAG_V) != 0,
        )

    @flags.setter
    def flags(self, f: Flags) -> None:
        """Update the condition flags in R15."""
        r15 = self._regs[15] & ~(FLAG_N | FLAG_Z | FLAG_C | FLAG_V) & MASK_32
        if f.n:
            r15 |= FLAG_N
        if f.z:
            r15 |= FLAG_Z
        if f.c:
            r15 |= FLAG_C
        if f.v:
            r15 |= FLAG_V
        self._regs[15] = r15 & MASK_32

    @property
    def mode(self) -> int:
        """Return the current processor mode (0=USR, 1=FIQ, 2=IRQ, 3=SVC)."""
        return self._regs[15] & MODE_MASK

    @property
    def halted(self) -> bool:
        """Return True if the CPU has been halted."""
        return self._halted

    # ── Memory access ──────────────────────────────────────────────────

    def read_word(self, addr: int) -> int:
        """Read a 32-bit word from memory (little-endian)."""
        addr = addr & PC_MASK
        a = addr & ~3  # Word-align
        if a + 3 >= len(self._memory):
            return 0
        return (
            self._memory[a]
            | (self._memory[a + 1] << 8)
            | (self._memory[a + 2] << 16)
            | (self._memory[a + 3] << 24)
        )

    def write_word(self, addr: int, value: int) -> None:
        """Write a 32-bit word to memory (little-endian)."""
        addr = addr & PC_MASK
        a = addr & ~3
        value = value & MASK_32
        if a + 3 >= len(self._memory):
            return
        self._memory[a] = value & 0xFF
        self._memory[a + 1] = (value >> 8) & 0xFF
        self._memory[a + 2] = (value >> 16) & 0xFF
        self._memory[a + 3] = (value >> 24) & 0xFF

    def read_byte(self, addr: int) -> int:
        """Read a single byte from memory."""
        addr = addr & PC_MASK
        if addr >= len(self._memory):
            return 0
        return self._memory[addr]

    def write_byte(self, addr: int, value: int) -> None:
        """Write a single byte to memory."""
        addr = addr & PC_MASK
        if addr >= len(self._memory):
            return
        self._memory[addr] = value & 0xFF

    @property
    def memory(self) -> bytearray:
        """Return a reference to the raw memory array."""
        return self._memory

    def load_program(self, code: bytes | bytearray, start_addr: int = 0) -> None:
        """Load machine code into memory at the given start address."""
        for i, b in enumerate(code):
            addr = start_addr + i
            if addr < len(self._memory):
                self._memory[addr] = b

    # ── Execution ──────────────────────────────────────────────────────

    def step(self) -> Trace:
        """Execute one instruction and return a trace of what happened.

        The Fetch-Decode-Execute cycle:

        1. FETCH:  Read 32-bit instruction from memory at PC
        2. DECODE: Extract fields (condition, opcode, registers, shift, etc.)
        3. CHECK:  Evaluate condition code against current flags
        4. EXECUTE: If condition met, perform the operation
        5. ADVANCE: PC += 4 (unless a branch or PC write occurred)

        The 3-stage pipeline means the PC is always 8 bytes ahead of the
        currently executing instruction. When you read R15 during execution
        of an instruction at address A, you get A + 8.
        """
        # Capture state before execution
        cur_pc = self.pc
        regs_before = [self.read_register(i) for i in range(16)]
        flags_before = self.flags

        # Fetch
        instruction = self.read_word(cur_pc)

        # Decode
        decoded = decode(instruction)

        # Evaluate condition
        cond_met = evaluate_condition(decoded.cond, flags_before)

        trace = Trace(
            address=cur_pc,
            raw=instruction,
            mnemonic=disassemble(decoded),
            condition=cond_string(decoded.cond),
            condition_met=cond_met,
            regs_before=regs_before,
            flags_before=flags_before,
        )

        # Advance PC (default: next instruction)
        self.pc = (cur_pc + 4) & PC_MASK

        if cond_met:
            if decoded.inst_type == INST_DATA_PROCESSING:
                self._execute_data_processing(decoded, trace)
            elif decoded.inst_type == INST_LOAD_STORE:
                self._execute_load_store(decoded, trace)
            elif decoded.inst_type == INST_BLOCK_TRANSFER:
                self._execute_block_transfer(decoded, trace)
            elif decoded.inst_type == INST_BRANCH:
                self._execute_branch(decoded, trace)
            elif decoded.inst_type == INST_SWI:
                self._execute_swi(decoded, trace)
            elif decoded.inst_type in (INST_COPROCESSOR, INST_UNDEFINED):
                self._trap_undefined(cur_pc)

        # Capture state after execution
        trace.regs_after = [self.read_register(i) for i in range(16)]
        trace.flags_after = self.flags

        return trace

    def run(self, max_steps: int = 10000) -> list[Trace]:
        """Execute instructions until halted or max_steps reached."""
        traces: list[Trace] = []
        for _ in range(max_steps):
            if self._halted:
                break
            traces.append(self.step())
        return traces

    # ── Data Processing ────────────────────────────────────────────────

    def _read_reg_for_exec(self, index: int) -> int:
        """Read a register value as seen during instruction execution.

        For R15, this returns PC + 8 (accounting for the 3-stage pipeline).
        But we've already advanced PC by 4 in step(), so we add 4 more.
        """
        if index == 15:
            return (self._regs[15] + 4) & MASK_32
        return self.read_register(index)

    def _execute_data_processing(
        self, d: DecodedInstruction, trace: Trace
    ) -> None:
        """Execute a data processing instruction."""
        # Get first operand (Rn)
        a = 0
        if d.opcode not in (OP_MOV, OP_MVN):
            a = self._read_reg_for_exec(d.rn)

        # Get second operand through barrel shifter
        f = self.flags

        if d.immediate:
            b, shifter_carry = decode_immediate(d.imm8, d.rotate)
            if d.rotate == 0:
                shifter_carry = f.c  # Carry unchanged when no rotation
        else:
            rm_val = self._read_reg_for_exec(d.rm)
            if d.shift_by_reg:
                shift_amount = self._read_reg_for_exec(d.rs) & 0xFF
            else:
                shift_amount = d.shift_imm
            b, shifter_carry = barrel_shift(
                rm_val, d.shift_type, shift_amount, f.c, d.shift_by_reg
            )

        # Execute ALU operation
        result = alu_execute(d.opcode, a, b, f.c, shifter_carry, f.v)

        # Write result to Rd (unless test-only operation)
        if result.write_result:
            if d.rd == 15:
                if d.s:
                    # MOVS PC, LR — restore PC and flags
                    self._regs[15] = result.result & MASK_32
                else:
                    self.pc = result.result & PC_MASK
            else:
                self.write_register(d.rd, result.result)

        # Update flags if S bit set (and Rd is not R15)
        if d.s and d.rd != 15:
            self.flags = Flags(n=result.n, z=result.z, c=result.c, v=result.v)

        # For test-only ops, always update flags
        if is_test_op(d.opcode):
            self.flags = Flags(n=result.n, z=result.z, c=result.c, v=result.v)

    # ── Load/Store ─────────────────────────────────────────────────────

    def _execute_load_store(self, d: DecodedInstruction, trace: Trace) -> None:
        """Execute a single data transfer instruction (LDR/STR)."""
        # Compute offset
        if d.immediate:
            # Register offset (with optional shift)
            rm_val = self._read_reg_for_exec(d.rm)
            if d.shift_imm != 0:
                rm_val, _ = barrel_shift(
                    rm_val, d.shift_type, d.shift_imm, self.flags.c, False
                )
            offset = rm_val
        else:
            offset = d.offset12

        # Base address
        base = self._read_reg_for_exec(d.rn)

        # Compute effective address
        if d.up:
            addr = (base + offset) & MASK_32
        else:
            addr = (base - offset) & MASK_32

        # Pre/post-indexed
        transfer_addr = addr if d.pre_index else base

        if d.load:
            # LDR / LDRB
            if d.byte:
                value = self.read_byte(transfer_addr)
            else:
                value = self.read_word(transfer_addr)
                # ARM1 quirk: unaligned word loads rotate the data
                rotation = (transfer_addr & 3) * 8
                if rotation != 0:
                    value = (
                        (value >> rotation) | (value << (32 - rotation))
                    ) & MASK_32
            trace.memory_reads.append(
                MemoryAccess(address=transfer_addr, value=value)
            )
            if d.rd == 15:
                self._regs[15] = value & MASK_32
            else:
                self.write_register(d.rd, value)
        else:
            # STR / STRB
            value = self._read_reg_for_exec(d.rd)
            if d.byte:
                self.write_byte(transfer_addr, value & 0xFF)
            else:
                self.write_word(transfer_addr, value)
            trace.memory_writes.append(
                MemoryAccess(address=transfer_addr, value=value)
            )

        # Write-back
        if d.write_back or not d.pre_index:
            if d.rn != 15:
                self.write_register(d.rn, addr)

    # ── Block Transfer ─────────────────────────────────────────────────

    def _execute_block_transfer(
        self, d: DecodedInstruction, trace: Trace
    ) -> None:
        """Execute a block data transfer instruction (LDM/STM)."""
        base = self.read_register(d.rn)
        reg_list = d.register_list

        # Count registers in the list
        count = bin(reg_list).count("1")
        if count == 0:
            return

        # Calculate the start address based on addressing mode
        if not d.pre_index and d.up:      # IA
            start_addr = base
        elif d.pre_index and d.up:        # IB
            start_addr = base + 4
        elif not d.pre_index and not d.up: # DA
            start_addr = base - (count * 4) + 4
        else:                              # DB
            start_addr = base - (count * 4)

        start_addr = start_addr & MASK_32
        addr = start_addr

        for i in range(16):
            if (reg_list >> i) & 1 == 0:
                continue

            if d.load:
                value = self.read_word(addr)
                trace.memory_reads.append(
                    MemoryAccess(address=addr, value=value)
                )
                if i == 15:
                    self._regs[15] = value & MASK_32
                else:
                    self.write_register(i, value)
            else:
                if i == 15:
                    value = (self._regs[15] + 4) & MASK_32  # PC + 8
                else:
                    value = self.read_register(i)
                self.write_word(addr, value)
                trace.memory_writes.append(
                    MemoryAccess(address=addr, value=value)
                )
            addr = (addr + 4) & MASK_32

        # Write-back
        if d.write_back:
            if d.up:
                new_base = (base + count * 4) & MASK_32
            else:
                new_base = (base - count * 4) & MASK_32
            self.write_register(d.rn, new_base)

    # ── Branch ─────────────────────────────────────────────────────────

    def _execute_branch(self, d: DecodedInstruction, trace: Trace) -> None:
        """Execute a branch instruction (B/BL).

        The branch offset is relative to PC + 8 from the original instruction.
        Since we already did PC += 4, we need PC + 4 more = current PC + 4.
        """
        branch_base = (self.pc + 4) & MASK_32

        if d.link:
            # BL: save return address in R14 (LR)
            return_addr = self._regs[15] & MASK_32
            self.write_register(14, return_addr)

        # Compute target address
        target = (branch_base + d.branch_offset) & MASK_32
        self.pc = target & PC_MASK

    # ── SWI ────────────────────────────────────────────────────────────

    def _execute_swi(self, d: DecodedInstruction, trace: Trace) -> None:
        """Execute a software interrupt instruction.

        If the SWI comment field matches HALT_SWI, halt the CPU.
        Otherwise, enter Supervisor mode and jump to the SWI vector.
        """
        if d.swi_comment == HALT_SWI:
            self._halted = True
            return

        # Save R15 to R14_svc
        self._regs[25] = self._regs[15]
        self._regs[26] = self._regs[15]

        # Set mode to SVC, disable IRQs
        r15 = self._regs[15]
        r15 = (r15 & ~MODE_MASK & MASK_32) | MODE_SVC
        r15 |= FLAG_I
        self._regs[15] = r15 & MASK_32

        # Jump to SWI vector (0x08)
        self.pc = 0x08

    # ── Exception handling ─────────────────────────────────────────────

    def _trap_undefined(self, instr_addr: int) -> None:
        """Handle an undefined instruction trap."""
        self._regs[26] = self._regs[15]

        r15 = self._regs[15]
        r15 = (r15 & ~MODE_MASK & MASK_32) | MODE_SVC
        r15 |= FLAG_I
        self._regs[15] = r15 & MASK_32

        self.pc = 0x04

    # ── simulator-protocol conformance ────────────────────────────────────

    def get_state(self) -> ARM1State:
        """Return a frozen snapshot of the current CPU state.

        Satisfies the ``Simulator[ARM1State]`` protocol.  Every field is
        copied out of the mutable simulator so the returned value is a true
        point-in-time snapshot — independent of any future simulation steps.

        Register banking is respected: ``read_register(i)`` returns the
        correct physical register for the current mode, so the 16-element
        ``registers`` tuple always reflects the logical view.

        Memory is snapshotted by converting ``bytearray → bytes``, which
        copies all bytes once and makes the result immutable.

        Returns
        -------
        ARM1State:
            Frozen dataclass with all CPU state at this moment.
        """
        f = self.flags
        return ARM1State(
            registers=tuple(self.read_register(i) for i in range(16)),
            pc=self.pc,
            mode=self.mode,
            flags_n=f.n,
            flags_z=f.z,
            flags_c=f.c,
            flags_v=f.v,
            memory=bytes(self._memory),
            halted=self._halted,
            # Banked FIQ registers: physical indices 16–22 (R8_fiq … R14_fiq)
            banked_fiq=tuple(self._regs[16 + i] for i in range(7)),
            # Banked IRQ registers: physical indices 23–24 (R13_irq, R14_irq)
            banked_irq=tuple(self._regs[23 + i] for i in range(2)),
            # Banked SVC registers: physical indices 25–26 (R13_svc, R14_svc)
            banked_svc=tuple(self._regs[25 + i] for i in range(2)),
        )

    def load(self, program: bytes) -> None:
        """Load a binary program into memory at address 0.

        Satisfies the ``Simulator[ARM1State]`` protocol ``load()`` method.
        Delegates to the existing ``load_program()`` so all existing callers
        are unaffected.

        Parameters
        ----------
        program:
            Raw machine-code bytes to write into memory starting at address 0.
        """
        self.load_program(program, 0)

    def execute(
        self, program: bytes, max_steps: int = 100_000
    ) -> ExecutionResult[ARM1State]:
        """Load *program*, run to HALT or *max_steps*, return a full result.

        Satisfies the ``Simulator[ARM1State]`` protocol.

        This method is the primary entry point for end-to-end testing.  It:

        1. Resets the CPU to power-on state (all registers zeroed, SVC mode).
        2. Loads the program bytes into memory at address 0.
        3. Runs the fetch-decode-execute loop.
        4. Collects a ``StepTrace`` for every instruction executed.
        5. Returns an ``ExecutionResult[ARM1State]`` with the final snapshot,
           trace list, halt status, and error (if any).

        The per-step ``StepTrace`` is built from the existing ``Trace`` objects
        returned by ``step()``:

        - ``pc_before`` ← ``trace.address``
        - ``pc_after``  ← the PC value *after* the instruction ran, derived
          by reading ``self.pc`` immediately after the step.
        - ``mnemonic``  ← ``trace.mnemonic``  (disassembled instruction text)
        - ``description`` ← a formatted string combining address, mnemonic,
          and condition suffix.

        Parameters
        ----------
        program:
            Raw machine-code bytes.
        max_steps:
            Maximum instructions to execute before giving up.  Default 100 000.

        Returns
        -------
        ExecutionResult[ARM1State]:
            Full result including halted status, step count, final state,
            optional error string, and per-instruction trace list.

        Examples
        --------
        >>> import struct
        >>> from arm1_simulator import ARM1, COND_AL, encode_mov_imm, encode_halt
        >>> cpu = ARM1(1024)
        >>> code = b"".join(
        ...     struct.pack("<I", w)
        ...     for w in [encode_mov_imm(COND_AL, 0, 42), encode_halt()]
        ... )
        >>> result = cpu.execute(code)
        >>> result.ok
        True
        >>> result.final_state.registers[0]
        42
        """
        from simulator_protocol import ExecutionResult, StepTrace

        self.reset()
        self.load_program(program, 0)

        step_traces: list[StepTrace] = []
        error: str | None = None

        for _ in range(max_steps):
            if self._halted:
                break
            arm_trace = self.step()
            pc_after = self.pc
            step_traces.append(
                StepTrace(
                    pc_before=arm_trace.address,
                    pc_after=pc_after,
                    mnemonic=arm_trace.mnemonic,
                    description=(
                        f"{arm_trace.mnemonic} @ 0x{arm_trace.address:08X}"
                    ),
                )
            )
        else:
            # Loop completed without a break — max_steps was reached
            if not self._halted:
                error = f"max_steps ({max_steps}) exceeded"

        return ExecutionResult(
            halted=self._halted,
            steps=len(step_traces),
            final_state=self.get_state(),
            error=error,
            traces=step_traces,
        )

    def __str__(self) -> str:
        """Return a formatted representation of the CPU state."""
        m = mode_string(self.mode)
        f = self.flags
        flag_str = (
            ("N" if f.n else "n")
            + ("Z" if f.z else "z")
            + ("C" if f.c else "c")
            + ("V" if f.v else "v")
        )
        s = f"ARM1 [{m}] {flag_str} PC={self.pc:08X}\n"
        for i in range(0, 16, 4):
            s += (
                f"  R{i:<2d}={self.read_register(i):08X}"
                f"  R{i+1:<2d}={self.read_register(i+1):08X}"
                f"  R{i+2:<2d}={self.read_register(i+2):08X}"
                f"  R{i+3:<2d}={self.read_register(i+3):08X}\n"
            )
        return s
