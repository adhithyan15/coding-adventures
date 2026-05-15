"""
ARMv7-A / Thumb-2 Behavioral Simulator
========================================

This module implements a behavioral simulator for the ARMv7-A architecture
running in Thumb-2 mode.  Thumb-2 is a variable-width encoding: instructions
are either 16 bits or 32 bits wide, detected by inspecting bits [15:11] of the
first halfword.

  bits [15:11] in {11101, 11110, 11111}  →  32-bit instruction
  all other values                        →  16-bit instruction

The simulator executes one instruction per step, updating registers, CPSR flags,
memory, and the program counter.  The all-zero halfword (0x0000) is the halt
sentinel — when fetched, it sets halted=True and stops.

Design philosophy
-----------------
  - Pure Python, no C extensions — easy to read and modify.
  - Every instruction is documented inline with its encoding layout.
  - Flag computation functions are small, testable units.
  - The CPUState class is mutable during execution; the external ARMv7AState
    dataclass is frozen (immutable snapshots).

Barrel shifter
--------------
ARM's distinctive feature: the second operand to almost every data-processing
instruction passes through a barrel shifter before reaching the ALU.  The
shifter produces a 32-bit result and a carry-out bit that feeds the C flag for
non-arithmetic operations (MOV, AND, ORR, etc.).

  LSL #n: shift left n bits, fill with 0, carry = last bit shifted out
  LSR #n: shift right n bits, fill with 0, carry = last bit shifted out
  ASR #n: shift right n bits, fill with MSB, carry = last bit shifted out
  ROR #n: rotate right n bits, carry = last bit rotated out
  RRX   : rotate right 1 through carry — carry = bit[0], new MSB = old carry
"""

from __future__ import annotations

from dataclasses import dataclass

from .state import (
    COND_CC,
    COND_CS,
    COND_EQ,
    COND_GE,
    COND_GT,
    COND_HI,
    COND_LE,
    COND_LS,
    COND_LT,
    COND_MI,
    COND_NE,
    COND_PL,
    COND_VC,
    COND_VS,
    CPSR_C,
    CPSR_N,
    CPSR_T,
    CPSR_V,
    CPSR_Z,
    LR,
    MASK8,
    MASK16,
    MASK32,
    MEM_SIZE,
    NUM_REGS,
    PC,
    SP,
    ARMv7AState,
    sext,
    sext8,
)

# ── Step trace ─────────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class StepTrace:
    """
    Record of a single instruction execution returned by ARMv7ASimulator.step().

    Attributes
    ----------
    pc_before   Address from which the instruction was fetched.
    pc_after    PC value after the instruction executed (next fetch address).
    halted      True if the simulator is now halted (fetched the halt sentinel).
    """

    pc_before: int
    pc_after: int
    halted: bool


# ── CPU mutable state ──────────────────────────────────────────────────────────


class _CPU:
    """
    Mutable simulator state — owned exclusively by ARMv7ASimulator.

    We keep a single mutable object during execution for performance; the
    public interface exposes only frozen ARMv7AState snapshots.
    """

    __slots__ = ("gpr", "cpsr", "memory", "pc", "halted")

    def __init__(self) -> None:
        self.gpr: list[int] = [0] * NUM_REGS    # R0–R15
        self.cpsr: int = 0
        self.memory: bytearray = bytearray(MEM_SIZE)
        self.pc: int = 0
        self.halted: bool = False

    # ── Flag helpers ──────────────────────────────────────────────────────────

    def get_flag(self, bit: int) -> int:
        """Return the value (0 or 1) of a CPSR flag bit."""
        return (self.cpsr >> bit) & 1

    def set_flag(self, bit: int, val: int) -> None:
        """Set or clear a single CPSR flag bit."""
        if val:
            self.cpsr |= (1 << bit)
        else:
            self.cpsr &= ~(1 << bit)

    def nf(self) -> int: return (self.cpsr >> CPSR_N) & 1
    def zf(self) -> int: return (self.cpsr >> CPSR_Z) & 1
    def cf(self) -> int: return (self.cpsr >> CPSR_C) & 1
    def vf(self) -> int: return (self.cpsr >> CPSR_V) & 1

    def set_nz(self, result: int) -> None:
        """Update N and Z flags from a 32-bit result."""
        self.set_flag(CPSR_N, (result >> 31) & 1)
        self.set_flag(CPSR_Z, 1 if (result & MASK32) == 0 else 0)

    def set_nzc(self, result: int, carry: int) -> None:
        """Update N, Z, and C flags."""
        self.set_nz(result)
        self.set_flag(CPSR_C, carry)

    # ── Memory helpers ────────────────────────────────────────────────────────

    def read8(self, addr: int) -> int:
        """Read an 8-bit byte from memory (little-endian)."""
        return self.memory[addr & 0xFFFF]

    def write8(self, addr: int, val: int) -> None:
        self.memory[addr & 0xFFFF] = val & 0xFF

    def read16(self, addr: int) -> int:
        """Read a 16-bit halfword from memory (little-endian)."""
        a = addr & 0xFFFF
        return self.memory[a] | (self.memory[(a + 1) & 0xFFFF] << 8)

    def write16(self, addr: int, val: int) -> None:
        a = addr & 0xFFFF
        self.memory[a] = val & 0xFF
        self.memory[(a + 1) & 0xFFFF] = (val >> 8) & 0xFF

    def read32(self, addr: int) -> int:
        """Read a 32-bit word from memory (little-endian)."""
        a = addr & 0xFFFF
        return (self.memory[a]
                | (self.memory[(a + 1) & 0xFFFF] << 8)
                | (self.memory[(a + 2) & 0xFFFF] << 16)
                | (self.memory[(a + 3) & 0xFFFF] << 24))

    def write32(self, addr: int, val: int) -> None:
        a = addr & 0xFFFF
        self.memory[a] = val & 0xFF
        self.memory[(a + 1) & 0xFFFF] = (val >> 8) & 0xFF
        self.memory[(a + 2) & 0xFFFF] = (val >> 16) & 0xFF
        self.memory[(a + 3) & 0xFFFF] = (val >> 24) & 0xFF

    def fetch16(self) -> int:
        """Fetch a 16-bit halfword at PC and advance PC by 2."""
        hw = self.read16(self.pc)
        self.pc = (self.pc + 2) & MASK32
        return hw

    # ── Register access ───────────────────────────────────────────────────────

    def read_reg(self, r: int) -> int:
        """Read a register.  Reading R15 returns PC+4 (Thumb convention)."""
        if r == PC:
            return (self.pc + 2) & MASK32   # pc was already advanced past current hw
        return self.gpr[r] & MASK32

    def write_reg(self, r: int, val: int) -> None:
        """Write a register.  Writing R15 sets the PC (branch)."""
        val = val & MASK32
        if r == PC:
            self.pc = val & ~1   # clear T-bit from target address
        else:
            self.gpr[r] = val


# ── Barrel shifter ─────────────────────────────────────────────────────────────


def _lsl(val: int, n: int, carry_in: int) -> tuple[int, int]:
    """Logical shift left.  Returns (result, carry_out)."""
    if n == 0:
        return val & MASK32, carry_in
    if n >= 32:
        carry = (val >> (32 - n)) & 1 if n == 32 else 0
        return 0, carry
    carry = (val >> (32 - n)) & 1
    return (val << n) & MASK32, carry


def _lsr(val: int, n: int, carry_in: int) -> tuple[int, int]:
    """Logical shift right.  Returns (result, carry_out)."""
    if n == 0:
        return val & MASK32, carry_in
    if n >= 32:
        carry = (val >> 31) & 1 if n == 32 else 0
        return 0, carry
    carry = (val >> (n - 1)) & 1
    return (val >> n) & MASK32, carry


def _asr(val: int, n: int, carry_in: int) -> tuple[int, int]:
    """Arithmetic shift right.  Returns (result, carry_out)."""
    if n == 0:
        return val & MASK32, carry_in
    sign = (val >> 31) & 1
    if n >= 32:
        result = MASK32 if sign else 0
        return result, sign
    carry = (val >> (n - 1)) & 1
    # Extend sign bit
    result = val >> n
    if sign:
        result |= MASK32 << (32 - n)
    return result & MASK32, carry


def _ror(val: int, n: int, carry_in: int) -> tuple[int, int]:
    """Rotate right.  Returns (result, carry_out)."""
    if n == 0:
        return val & MASK32, carry_in
    n = n & 31
    if n == 0:
        carry = (val >> 31) & 1
        return val & MASK32, carry
    result = ((val >> n) | (val << (32 - n))) & MASK32
    carry = (val >> (n - 1)) & 1
    return result, carry


def _rrx(val: int, carry_in: int) -> tuple[int, int]:
    """Rotate right one bit through carry.  Returns (result, carry_out)."""
    carry_out = val & 1
    result = ((carry_in << 31) | (val >> 1)) & MASK32
    return result, carry_out


def _apply_shift_imm(val: int, shift_type: int, imm5: int, carry_in: int) -> tuple[int, int]:
    """
    Apply a shift-immediate operation to val.

    shift_type:
      0 = LSL, 1 = LSR, 2 = ASR, 3 = ROR/RRX
    imm5: shift amount (0 means special for LSR/ASR: treat as 32)

    Returns (result, carry_out).
    """
    if shift_type == 0:   # LSL
        return _lsl(val, imm5, carry_in)
    if shift_type == 1:   # LSR
        n = 32 if imm5 == 0 else imm5
        return _lsr(val, n, carry_in)
    if shift_type == 2:   # ASR
        n = 32 if imm5 == 0 else imm5
        return _asr(val, n, carry_in)
    # shift_type == 3: ROR or RRX
    if imm5 == 0:
        return _rrx(val, carry_in)
    return _ror(val, imm5, carry_in)


# ── ALU flag helpers ───────────────────────────────────────────────────────────


def _add_flags(cpu: _CPU, a: int, b: int, result_full: int) -> None:
    """Set N, Z, C, V for an addition: a + b."""
    result32 = result_full & MASK32
    cpu.set_flag(CPSR_N, (result32 >> 31) & 1)
    cpu.set_flag(CPSR_Z, 1 if result32 == 0 else 0)
    cpu.set_flag(CPSR_C, 1 if result_full > MASK32 else 0)
    # Overflow: both operands same sign, result different sign
    sa = (a >> 31) & 1
    sb = (b >> 31) & 1
    sr = (result32 >> 31) & 1
    cpu.set_flag(CPSR_V, 1 if (sa == sb) and (sa != sr) else 0)


def _sub_flags(cpu: _CPU, a: int, b: int, result_full: int) -> None:
    """Set N, Z, C, V for a subtraction: a - b.
    Carry convention: C=1 means NO borrow (result ≥ 0 unsigned)."""
    result32 = result_full & MASK32
    cpu.set_flag(CPSR_N, (result32 >> 31) & 1)
    cpu.set_flag(CPSR_Z, 1 if result32 == 0 else 0)
    cpu.set_flag(CPSR_C, 0 if a < b else 1)   # C=1 means no borrow
    # Overflow: operands have different signs, result sign matches b
    sa = (a >> 31) & 1
    sb = (b >> 31) & 1
    sr = (result32 >> 31) & 1
    cpu.set_flag(CPSR_V, 1 if (sa != sb) and (sr == sb) else 0)


# ── Condition evaluation ───────────────────────────────────────────────────────


def _check_cond(cpu: _CPU, cond: int) -> bool:
    """Evaluate a 4-bit condition code against the current CPSR flags."""
    n, z, c, v = cpu.nf(), cpu.zf(), cpu.cf(), cpu.vf()
    if cond == COND_EQ: return z == 1
    if cond == COND_NE: return z == 0
    if cond == COND_CS: return c == 1
    if cond == COND_CC: return c == 0
    if cond == COND_MI: return n == 1
    if cond == COND_PL: return n == 0
    if cond == COND_VS: return v == 1
    if cond == COND_VC: return v == 0
    if cond == COND_HI: return c == 1 and z == 0
    if cond == COND_LS: return c == 0 or z == 1
    if cond == COND_GE: return n == v
    if cond == COND_LT: return n != v
    if cond == COND_GT: return z == 0 and n == v
    if cond == COND_LE: return z == 1 or n != v
    return True   # COND_AL (0b1110) and undefined 0b1111


# ── 16-bit instruction execution ──────────────────────────────────────────────


def _exec_16(cpu: _CPU, hw: int) -> None:
    """
    Decode and execute a 16-bit Thumb instruction.

    We dispatch on the high bits of the halfword following the Thumb encoding
    hierarchy documented in the ARMv7-A Architecture Reference Manual,
    section A6.2.
    """

    # ── Shift (immediate), add, subtract, move, compare ─────────────────────
    # bits [15:13] == 0b000 → shift immediate, add, subtract, move, compare
    if (hw >> 13) == 0b000:
        _exec_16_shift_addsub(cpu, hw)
        return

    # bits [15:13] == 0b001 → move/compare/add/subtract immediate
    if (hw >> 13) == 0b001:
        _exec_16_mcas_imm(cpu, hw)
        return

    # bits [15:10] == 0b010000 → data processing
    if (hw >> 10) == 0b010000:
        _exec_16_data_proc(cpu, hw)
        return

    # bits [15:10] == 0b010001 → special data instructions and BX/BLX
    if (hw >> 10) == 0b010001:
        _exec_16_special(cpu, hw)
        return

    # bits [15:12] == 0b0101 → load/store (register offset)
    if (hw >> 12) == 0b0101:
        _exec_16_ldst_reg(cpu, hw)
        return

    # bits [15:12] in {0110, 0111, 1000} → immediate-offset load/store
    # (0b1001 = SP-relative, handled separately below)
    if (hw >> 12) & 0xF in (0b0110, 0b0111, 0b1000):
        _exec_16_ldst_imm(cpu, hw)
        return

    # bits [15:12] == 0b1001 → load/store (SP-relative)
    if (hw >> 12) == 0b1001:
        _exec_16_ldst_sp(cpu, hw)
        return

    # bits [15:12] == 0b1010 → add to SP or PC (ADR)
    if (hw >> 12) == 0b1010:
        _exec_16_adr(cpu, hw)
        return

    # bits [15:12] == 0b1011 → miscellaneous 16-bit
    if (hw >> 12) == 0b1011:
        _exec_16_misc(cpu, hw)
        return

    # bits [15:12] == 0b1100 → load/store multiple
    if (hw >> 12) == 0b1100:
        _exec_16_ldstm(cpu, hw)
        return

    # bits [15:12] == 0b1101 → conditional branch (or SVC)
    if (hw >> 12) == 0b1101:
        _exec_16_cond_branch(cpu, hw)
        return

    # bits [15:11] == 0b11100 → unconditional branch
    if (hw >> 11) == 0b11100:
        imm11 = hw & 0x7FF
        offset = sext(imm11, 11) * 2
        cpu.pc = (cpu.pc + offset) & MASK32
        return

    # Unknown / unimplemented — treat as NOP rather than crash
    # (helps tests that only care about specific instructions)


def _exec_16_shift_addsub(cpu: _CPU, hw: int) -> None:
    """
    Shift immediate, add, and subtract — bits [15:13] == 000.

    Encoding layout (Thumb):
      [15:11] opcode
      [10:6]  imm5 / Rm
      [5:3]   Rn
      [2:0]   Rd
    """
    op = (hw >> 11) & 0b11
    if op in (0b00, 0b01, 0b10):
        # LSL, LSR, ASR immediate
        shift_type = op        # 0=LSL, 1=LSR, 2=ASR
        imm5 = (hw >> 6) & 0x1F
        rm = (hw >> 3) & 0x7
        rd = hw & 0x7
        val = cpu.read_reg(rm)
        result, carry = _apply_shift_imm(val, shift_type, imm5, cpu.cf())
        cpu.write_reg(rd, result)
        cpu.set_nzc(result, carry)
        return

    # bits [15:9] → add/subtract
    op2 = (hw >> 9) & 0b11
    imm3_rm = (hw >> 6) & 0x7
    rn = (hw >> 3) & 0x7
    rd = hw & 0x7

    if op2 == 0b00:   # ADD Rd, Rn, Rm
        a = cpu.read_reg(rn)
        b = cpu.read_reg(imm3_rm)
        full = a + b
        cpu.write_reg(rd, full & MASK32)
        _add_flags(cpu, a, b, full)
    elif op2 == 0b01:   # SUB Rd, Rn, Rm
        a = cpu.read_reg(rn)
        b = cpu.read_reg(imm3_rm)
        full = a - b
        cpu.write_reg(rd, full & MASK32)
        _sub_flags(cpu, a, b, full)
    elif op2 == 0b10:   # ADD Rd, Rn, #imm3
        a = cpu.read_reg(rn)
        b = imm3_rm
        full = a + b
        cpu.write_reg(rd, full & MASK32)
        _add_flags(cpu, a, b, full)
    else:               # SUB Rd, Rn, #imm3
        a = cpu.read_reg(rn)
        b = imm3_rm
        full = a - b
        cpu.write_reg(rd, full & MASK32)
        _sub_flags(cpu, a, b, full)


def _exec_16_mcas_imm(cpu: _CPU, hw: int) -> None:
    """
    Move/Compare/Add/Subtract immediate — bits [15:13] == 001.

    Encoding: [15:13]=001 [12:11]=op [10:8]=Rd/Rn [7:0]=imm8
    op: 00=MOV, 01=CMP, 10=ADD, 11=SUB
    """
    op = (hw >> 11) & 0x3
    rdn = (hw >> 8) & 0x7
    imm8 = hw & 0xFF

    if op == 0b00:   # MOV Rd, #imm8
        cpu.write_reg(rdn, imm8)
        cpu.set_nz(imm8)
    elif op == 0b01:   # CMP Rn, #imm8
        a = cpu.read_reg(rdn)
        full = a - imm8
        _sub_flags(cpu, a, imm8, full)
    elif op == 0b10:   # ADD Rd, #imm8
        a = cpu.read_reg(rdn)
        full = a + imm8
        cpu.write_reg(rdn, full & MASK32)
        _add_flags(cpu, a, imm8, full)
    else:              # SUB Rd, #imm8
        a = cpu.read_reg(rdn)
        full = a - imm8
        cpu.write_reg(rdn, full & MASK32)
        _sub_flags(cpu, a, imm8, full)


def _exec_16_data_proc(cpu: _CPU, hw: int) -> None:
    """
    Data processing (register) — bits [15:10] == 010000.

    Encoding: [15:10]=010000 [9:6]=op [5:3]=Rm [2:0]=Rdn
    """
    op = (hw >> 6) & 0xF
    rm = (hw >> 3) & 0x7
    rdn = hw & 0x7

    a = cpu.read_reg(rdn)
    b = cpu.read_reg(rm)

    if op == 0b0000:   # AND
        r = a & b
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)
    elif op == 0b0001:   # EOR
        r = a ^ b
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)
    elif op == 0b0010:   # LSL (register)
        n = b & 0xFF
        result, carry = _lsl(a, n, cpu.cf())
        cpu.write_reg(rdn, result)
        cpu.set_nzc(result, carry)
    elif op == 0b0011:   # LSR (register)
        n = b & 0xFF
        result, carry = _lsr(a, n, cpu.cf())
        cpu.write_reg(rdn, result)
        cpu.set_nzc(result, carry)
    elif op == 0b0100:   # ASR (register)
        n = b & 0xFF
        result, carry = _asr(a, n, cpu.cf())
        cpu.write_reg(rdn, result)
        cpu.set_nzc(result, carry)
    elif op == 0b0101:   # ADC
        c = cpu.cf()
        full = a + b + c
        cpu.write_reg(rdn, full & MASK32)
        _add_flags(cpu, a, b + c, full)
    elif op == 0b0110:   # SBC: Rd = Rd - Rm - NOT(C) = Rd - Rm + C - 1
        c = cpu.cf()
        # SBC: result = a - b - (1 - c) = a - b + c - 1
        b_eff = b + (1 - c)
        full = a - b_eff
        cpu.write_reg(rdn, full & MASK32)
        _sub_flags(cpu, a, b_eff, full)
    elif op == 0b0111:   # ROR (register)
        n = b & 0xFF
        result, carry = _ror(a, n, cpu.cf())
        cpu.write_reg(rdn, result)
        cpu.set_nzc(result, carry)
    elif op == 0b1000:   # TST
        r = a & b
        cpu.set_nz(r)
    elif op == 0b1001:   # RSB / NEG: Rd = 0 - Rn
        full = 0 - a
        cpu.write_reg(rdn, full & MASK32)
        _sub_flags(cpu, 0, a, full)
    elif op == 0b1010:   # CMP
        full = a - b
        _sub_flags(cpu, a, b, full)
    elif op == 0b1011:   # CMN
        full = a + b
        _add_flags(cpu, a, b, full)
    elif op == 0b1100:   # ORR
        r = a | b
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)
    elif op == 0b1101:   # MUL
        r = (a * b) & MASK32
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)
    elif op == 0b1110:   # BIC
        r = a & (~b & MASK32)
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)
    else:                # MVN (0b1111)
        r = (~b) & MASK32
        cpu.write_reg(rdn, r)
        cpu.set_nz(r)


def _exec_16_special(cpu: _CPU, hw: int) -> None:
    """
    Special data instructions and BX/BLX — bits [15:10] == 010001.

    Encoding: [15:10]=010001 [9:8]=op
      op=00: ADD (high)
      op=01: CMP (high)
      op=10: MOV (high)
      op=11: BX / BLX
    """
    op = (hw >> 8) & 0x3
    dn = (hw >> 7) & 0x1   # high bit of Rd
    rm = (hw >> 3) & 0xF
    rd_low = hw & 0x7
    rd = (dn << 3) | rd_low

    if op == 0b11:
        # BX Rm or BLX Rm
        blx = (hw >> 7) & 1
        target = cpu.read_reg(rm)
        if blx:
            cpu.gpr[LR] = (cpu.pc) | 1   # pc already advanced past current instr
        cpu.pc = target & ~1   # clear LSB (T-bit handled by CPSR but we stay Thumb)
        return

    a = cpu.read_reg(rd)
    b = cpu.read_reg(rm)

    if op == 0b00:   # ADD (high regs)
        r = (a + b) & MASK32
        if rd == PC:
            cpu.pc = r & ~1
        else:
            cpu.gpr[rd] = r
    elif op == 0b01:   # CMP (high regs)
        full = a - b
        _sub_flags(cpu, a, b, full)
    else:            # MOV (high regs, op=10)
        if rd == PC:
            cpu.pc = b & ~1
        else:
            cpu.gpr[rd] = b


def _exec_16_ldst_reg(cpu: _CPU, hw: int) -> None:
    """
    Load/store (register offset) — bits [15:12] == 0101.

    Encoding: [15:12]=0101 [11:9]=op [8:6]=Rm [5:3]=Rn [2:0]=Rt
      000: STR   100: LDR
      001: STRH  101: LDRH
      010: STRB  110: LDRB
      011: LDRSB 111: LDRSH
    """
    op = (hw >> 9) & 0x7
    rm = (hw >> 6) & 0x7
    rn = (hw >> 3) & 0x7
    rt = hw & 0x7

    addr = (cpu.read_reg(rn) + cpu.read_reg(rm)) & MASK32

    if op == 0b000:   # STR
        cpu.write32(addr, cpu.read_reg(rt))
    elif op == 0b001:   # STRH
        cpu.write16(addr, cpu.read_reg(rt) & MASK16)
    elif op == 0b010:   # STRB
        cpu.write8(addr, cpu.read_reg(rt) & MASK8)
    elif op == 0b011:   # LDRSB
        v = cpu.read8(addr)
        cpu.write_reg(rt, sext(v, 8) & MASK32)
    elif op == 0b100:   # LDR
        cpu.write_reg(rt, cpu.read32(addr))
    elif op == 0b101:   # LDRH
        cpu.write_reg(rt, cpu.read16(addr))
    elif op == 0b110:   # LDRB
        cpu.write_reg(rt, cpu.read8(addr))
    else:               # LDRSH
        v = cpu.read16(addr)
        cpu.write_reg(rt, sext(v, 16) & MASK32)


def _exec_16_ldst_imm(cpu: _CPU, hw: int) -> None:
    """
    Load/Store (immediate offset) — bits [15:12] in {0110, 0111, 1000}.

    The direction bit is bit[11]: 0 = store, 1 = load.
    Use 5-bit op (bits[15:11]) to distinguish all six variants:

      0b01100 = STR  Rt, [Rn, #imm5*4]
      0b01101 = LDR  Rt, [Rn, #imm5*4]
      0b01110 = STRB Rt, [Rn, #imm5]
      0b01111 = LDRB Rt, [Rn, #imm5]
      0b10000 = STRH Rt, [Rn, #imm5*2]
      0b10001 = LDRH Rt, [Rn, #imm5*2]

    Encoding: [15:11]=op5 [10:6]=imm5 [5:3]=Rn [2:0]=Rt
    """
    op5 = (hw >> 11) & 0x1F   # bits [15:11] — encodes type AND direction
    imm5 = (hw >> 6) & 0x1F
    rn = (hw >> 3) & 0x7
    rt = hw & 0x7
    base = cpu.read_reg(rn)

    if op5 == 0b01100:   # STR word: Rt → [Rn + imm5*4]
        addr = (base + imm5 * 4) & MASK32
        cpu.write32(addr, cpu.read_reg(rt))
    elif op5 == 0b01101:   # LDR word: Rt ← [Rn + imm5*4]
        addr = (base + imm5 * 4) & MASK32
        cpu.write_reg(rt, cpu.read32(addr))
    elif op5 == 0b01110:   # STRB: Rt[7:0] → [Rn + imm5]
        addr = (base + imm5) & MASK32
        cpu.write8(addr, cpu.read_reg(rt) & MASK8)
    elif op5 == 0b01111:   # LDRB: Rt ← zero-ext([Rn + imm5])
        addr = (base + imm5) & MASK32
        cpu.write_reg(rt, cpu.read8(addr))
    elif op5 == 0b10000:   # STRH: Rt[15:0] → [Rn + imm5*2]
        addr = (base + imm5 * 2) & MASK32
        cpu.write16(addr, cpu.read_reg(rt) & MASK16)
    elif op5 == 0b10001:   # LDRH: Rt ← zero-ext([Rn + imm5*2])
        addr = (base + imm5 * 2) & MASK32
        cpu.write_reg(rt, cpu.read16(addr))


def _exec_16_ldst_sp(cpu: _CPU, hw: int) -> None:
    """
    Load/Store (SP-relative) — bits [15:12] == 1001.

    Encoding: [15:12]=1001 [11]=L [10:8]=Rt [7:0]=imm8
      L=0: STR Rt, [SP, #imm8*4]
      L=1: LDR Rt, [SP, #imm8*4]
    """
    load = (hw >> 11) & 1
    rt = (hw >> 8) & 0x7
    imm8 = hw & 0xFF
    addr = (cpu.gpr[SP] + imm8 * 4) & MASK32

    if load:
        cpu.write_reg(rt, cpu.read32(addr))
    else:
        cpu.write32(addr, cpu.read_reg(rt))


def _exec_16_adr(cpu: _CPU, hw: int) -> None:
    """
    Add to PC (ADR) — bits [15:12] == 1010.

    Encoding: [15:12]=1010 [11]=0 [10:8]=Rd [7:0]=imm8
    ADR Rd, PC + #imm8*4  (PC aligned to 4 bytes)
    """
    rd = (hw >> 8) & 0x7
    imm8 = hw & 0xFF
    # PC for ADR = (current_pc + 4) & ~3  (aligned, Thumb reads PC+4)
    pc_base = (cpu.pc + 2) & ~3
    cpu.write_reg(rd, (pc_base + imm8 * 4) & MASK32)


def _exec_16_misc(cpu: _CPU, hw: int) -> None:
    """
    Miscellaneous 16-bit instructions — bits [15:12] == 1011.

    Includes: PUSH, POP, ADD/SUB SP, NOP, BKPT, etc.
    """
    op = (hw >> 8) & 0xF

    if op == 0b0000:
        # ADD SP, SP, #imm7*4  OR  SUB SP, SP, #imm7*4
        sub = (hw >> 7) & 1
        imm7 = hw & 0x7F
        offset = imm7 * 4
        if sub:
            cpu.gpr[SP] = (cpu.gpr[SP] - offset) & MASK32
        else:
            cpu.gpr[SP] = (cpu.gpr[SP] + offset) & MASK32
        return

    # PUSH: bits [15:9] = 1011 010x
    if (hw >> 9) & 0b111 == 0b010:
        # bit 8 = push LR flag; bits [7:0] = register list
        push_lr = (hw >> 8) & 1
        reglist = hw & 0xFF
        regs = []
        for i in range(8):
            if (reglist >> i) & 1:
                regs.append(i)
        if push_lr:
            regs.append(LR)
        # Push highest-numbered registers first (decrement SP, store)
        for r in reversed(regs):
            cpu.gpr[SP] = (cpu.gpr[SP] - 4) & MASK32
            cpu.write32(cpu.gpr[SP], cpu.gpr[r])
        return

    # POP: bits [15:9] = 1011 110x
    if (hw >> 9) & 0b111 == 0b110:
        # bit 8 = pop PC flag; bits [7:0] = register list
        pop_pc = (hw >> 8) & 1
        reglist = hw & 0xFF
        regs = []
        for i in range(8):
            if (reglist >> i) & 1:
                regs.append(i)
        # Pop lowest-numbered registers first
        for r in regs:
            cpu.gpr[r] = cpu.read32(cpu.gpr[SP])
            cpu.gpr[SP] = (cpu.gpr[SP] + 4) & MASK32
        if pop_pc:
            val = cpu.read32(cpu.gpr[SP])
            cpu.gpr[SP] = (cpu.gpr[SP] + 4) & MASK32
            cpu.pc = val & ~1
        return

    # NOP (bits [15:8] == 1011 1111)
    if (hw >> 8) & 0xFF == 0b10111111:
        return


def _exec_16_ldstm(cpu: _CPU, hw: int) -> None:
    """
    Load/Store Multiple — bits [15:12] == 1100.

    Encoding: [15:12]=1100 [11]=L [10:8]=Rn [7:0]=reglist
      L=0: STM  Rn!, {reglist}  (store, writeback)
      L=1: LDM  Rn!, {reglist}  (load, writeback unless Rn in list)
    """
    load = (hw >> 11) & 1
    rn = (hw >> 8) & 0x7
    reglist = hw & 0xFF
    addr = cpu.gpr[rn]

    regs = [i for i in range(8) if (reglist >> i) & 1]

    if load:
        for r in regs:
            cpu.gpr[r] = cpu.read32(addr)
            addr = (addr + 4) & MASK32
        # Writeback only if Rn not in the loaded list
        if rn not in regs:
            cpu.gpr[rn] = addr
    else:
        for r in regs:
            cpu.write32(addr, cpu.gpr[r])
            addr = (addr + 4) & MASK32
        cpu.gpr[rn] = addr   # always writeback for STM


def _exec_16_cond_branch(cpu: _CPU, hw: int) -> None:
    """
    Conditional branch — bits [15:12] == 1101.

    Encoding: [15:12]=1101 [11:8]=cond [7:0]=imm8
    Branch target = PC + SignExtend(imm8, 8) * 2
    (PC here = address of instruction + 4)
    """
    cond = (hw >> 8) & 0xF
    imm8 = hw & 0xFF
    if cond == 0b1110:   # SVC / undefined for simulation; treat as NOP
        return
    if cond == 0b1111:   # SVC
        return

    if _check_cond(cpu, cond):
        offset = sext8(imm8) * 2
        cpu.pc = (cpu.pc + offset) & MASK32


# ── 32-bit Thumb-2 instruction execution ──────────────────────────────────────


def _exec_32(cpu: _CPU, hw1: int, hw2: int) -> None:
    """
    Decode and execute a 32-bit Thumb-2 instruction.

    The full 32-bit word is formed as (hw1 << 16) | hw2, but we keep them
    separate for clarity.  We dispatch on bits [28:27] of hw1 (op1) and the
    full encoding patterns.
    """
    op1 = (hw1 >> 11) & 0x3   # bits [12:11] of first halfword

    if op1 == 0b01:
        # Load/Store multiple, push/pop, load/store dual/exclusive, table branch
        # Simplified: check for PUSH.W / POP.W (32-bit)
        _exec_32_ldsm_or_branch(cpu, hw1, hw2)
        return

    if op1 == 0b10:
        # Data processing and branches
        _exec_32_dp_or_branch(cpu, hw1, hw2)
        return

    if op1 == 0b11:
        # Load/store (single register)
        _exec_32_ldst(cpu, hw1, hw2)
        return


def _exec_32_ldsm_or_branch(cpu: _CPU, hw1: int, hw2: int) -> None:
    """Handle 32-bit load/store multiple and related encodings."""
    # Detect BL: hw1[15:11]=11110, hw1[14:13] != 11; hw2[15]=1, hw2[14]=1
    # Actually BL is in op1==10, handled in _exec_32_dp_or_branch.
    # Here we have load/store dual, exclusive, table branch.
    # For this simulator, just skip (NOP) unknown 32-bit instructions.
    pass


def _exec_32_dp_or_branch(cpu: _CPU, hw1: int, hw2: int) -> None:
    """
    Data processing and branch (32-bit) — op1 == 10.

    Includes: BL, MOVW, MOVT, ADD.W, SUB.W, AND.W, ORR.W, EOR.W, etc.

    BL encoding (T1):
      hw1: 1111 0 S imm10
      hw2: 11 J1 1 J2 imm11
    """
    # Check if this is a BL instruction
    # hw1[15:11] = 11110 (bits 15..11 of first halfword)
    # hw2[15:14] = 11
    if (hw1 >> 11) == 0b11110 and ((hw2 >> 14) & 0x3) == 0b11 and ((hw2 >> 12) & 1) == 1:
        # BL T1
        s = (hw1 >> 10) & 1
        imm10 = hw1 & 0x3FF
        j1 = (hw2 >> 13) & 1
        j2 = (hw2 >> 11) & 1
        imm11 = hw2 & 0x7FF
        i1 = (~(j1 ^ s)) & 1
        i2 = (~(j2 ^ s)) & 1
        # offset = SignExtend(S:I1:I2:imm10:imm11:0, 25)
        offset_raw = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
        offset = sext(offset_raw, 25)
        # LR = PC | 1 (return address with T-bit set)
        cpu.gpr[LR] = cpu.pc | 1
        cpu.pc = (cpu.pc + offset) & MASK32
        return

    # Check for 32-bit data processing (wide immediate): hw1[15:11]=11110 or 11111
    # MOVW T3: hw1=11110.i.10.0.100.0 hw2=0.imm4.0.imm3.Rd.imm8
    # Simplified dispatching on hw1 pattern

    # MOV immediate (MOVW): hw1[15:11]=11110, hw1[8:5]=0100, hw1[4]=0 → op field
    # Encoding T3: 1111 0 i 1 0 0 1 0 0 / 1111 imm4  0  imm3 Rd imm8
    if (hw1 >> 11) == 0b11110:
        _exec_32_dp_imm(cpu, hw1, hw2)
        return


def _exec_32_dp_imm(cpu: _CPU, hw1: int, hw2: int) -> None:
    """
    32-bit data processing (immediate) instructions.

    Encoding pattern: hw1=1111 0 i op1 S Rn   hw2=0 imm3 Rd imm8
    We handle: AND, ORR, EOR, ADD (modified imm), SUB, MOV, MOVT
    """
    i_bit = (hw1 >> 10) & 1
    op1 = (hw1 >> 5) & 0xF
    s_bit = (hw1 >> 4) & 1
    rn = hw1 & 0xF

    imm3 = (hw2 >> 12) & 0x7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF

    # Construct modified immediate (Thumb-2 Modified Immediate Constant)
    # For simplicity, handle the most common form (no rotation) and MOVW/MOVT
    imm12 = (i_bit << 11) | (imm3 << 8) | imm8

    # MOVW (T3): op1=0b0010 (actually it's special: hw1=1111 0 i 1 0 0 1 0 0 / 1111 imm4 ...)
    # Let's check for MOVW specifically by looking at hw1 bits more carefully
    # MOVW T3: hw1 = 1111 0 i 1 0 0 1 0 0 imm4
    #          hw2 = 0 imm3 Rd imm8
    # hw1[15:11]=11110, hw1[10]=i, hw1[9:8]=10, hw1[7]=0, hw1[6:5]=10 (op=0b0010=MOV),
    # hw1[4]=0 (S=0), hw1[3:0]=imm4(high)
    # MOVT T1: same but op bits differ

    # Check MOVW: hw1 bits [8:4] = 1 0 0 1 0 → we look at hw1[9:4]
    movw_test = (hw1 >> 4) & 0b111111   # bits [9:4]
    if movw_test == 0b100100:   # MOVW (op=0100, S=0)
        imm4 = hw1 & 0xF
        imm16 = (imm4 << 12) | (i_bit << 11) | (imm3 << 8) | imm8
        cpu.write_reg(rd, imm16 & MASK32)
        return

    # MOVT T1: hw1[9:4] = 101100
    movt_test = (hw1 >> 4) & 0b111111
    if movt_test == 0b101100:   # MOVT
        imm4 = hw1 & 0xF
        imm16 = (imm4 << 12) | (i_bit << 11) | (imm3 << 8) | imm8
        # MOVT places imm16 into top 16 bits, keeps low 16 unchanged
        old = cpu.read_reg(rd)
        cpu.write_reg(rd, ((imm16 << 16) | (old & 0xFFFF)) & MASK32)
        return

    # Generic modified immediate — compute the constant
    imm32 = _thumb_expand_imm(imm12)

    # op1 dispatch for 32-bit DP:
    # 0b0000=AND, 0b0001=BIC, 0b0010=ORR/MOV, 0b0011=ORN/MVN,
    # 0b0100=EOR/TEQ, 0b1000=ADD/CMN, 0b1010=ADC, 0b1011=SBC,
    # 0b1101=SUB/CMP, 0b1110=RSB
    a = cpu.gpr[rn] & MASK32

    if op1 == 0b0000:   # AND
        r = a & imm32
        if rd != 15:
            cpu.write_reg(rd, r)
        if s_bit:
            cpu.set_nz(r)
    elif op1 == 0b0010:   # ORR / MOV (rn=15)
        if rn == 15:   # MOV
            cpu.write_reg(rd, imm32)
            if s_bit:
                cpu.set_nz(imm32)
        else:
            r = a | imm32
            cpu.write_reg(rd, r)
            if s_bit:
                cpu.set_nz(r)
    elif op1 == 0b0100:   # EOR
        r = a ^ imm32
        if rd != 15:
            cpu.write_reg(rd, r)
        if s_bit:
            cpu.set_nz(r)
    elif op1 == 0b1000:   # ADD
        full = a + imm32
        if rd != 15:
            cpu.write_reg(rd, full & MASK32)
        if s_bit:
            _add_flags(cpu, a, imm32, full)
    elif op1 == 0b1010:   # ADC
        c = cpu.cf()
        full = a + imm32 + c
        cpu.write_reg(rd, full & MASK32)
        if s_bit:
            _add_flags(cpu, a, imm32 + c, full)
    elif op1 == 0b1101:   # SUB
        full = a - imm32
        if rd != 15:
            cpu.write_reg(rd, full & MASK32)
        if s_bit:
            _sub_flags(cpu, a, imm32, full)
    elif op1 == 0b1110:   # RSB
        full = imm32 - a
        cpu.write_reg(rd, full & MASK32)
        if s_bit:
            _sub_flags(cpu, imm32, a, full)


def _thumb_expand_imm(imm12: int) -> int:
    """
    Thumb-2 Modified Immediate Constant expansion (ARMv7-A §A5.3.2).

    The 12-bit immediate encodes a 32-bit constant:
      bits [11:10] = 00 → plain 8-bit immediate (in various positions)
      otherwise       → rotate-right of an 8-bit constant
    """
    if (imm12 >> 10) == 0b00:
        # No rotation; direct forms based on bits [9:8]
        form = (imm12 >> 8) & 0x3
        imm8 = imm12 & 0xFF
        if form == 0b00:
            return imm8
        if form == 0b01:
            return (imm8 << 16) | imm8
        if form == 0b10:
            return (imm8 << 24) | (imm8 << 8)
        # form == 0b11
        return (imm8 << 24) | (imm8 << 16) | (imm8 << 8) | imm8
    else:
        # Rotate-right of (1 | imm7) by rotation amount
        imm7 = imm12 & 0x7F
        const = (1 << 7) | imm7
        rot = (imm12 >> 7) & 0x1F
        result, _ = _ror(const, rot, 0)
        return result


def _exec_32_ldst(cpu: _CPU, hw1: int, hw2: int) -> None:
    """
    32-bit load/store single (op1 == 11).

    LDR.W Rt, [Rn, #imm12]  and  STR.W Rt, [Rn, #imm12]
    Encoding: hw1=1111 100 size L Rn   hw2=Rt imm12
    """
    size = (hw1 >> 5) & 0x3
    load = (hw1 >> 4) & 1
    rn = hw1 & 0xF

    rt = (hw2 >> 12) & 0xF
    imm12 = hw2 & 0xFFF

    addr = (cpu.gpr[rn] + imm12) & MASK32

    if load:
        if size == 0b10:   # LDR word
            cpu.write_reg(rt, cpu.read32(addr))
        elif size == 0b01:   # LDRH
            cpu.write_reg(rt, cpu.read16(addr))
        elif size == 0b00:   # LDRB
            cpu.write_reg(rt, cpu.read8(addr))
    else:
        if size == 0b10:   # STR word
            cpu.write32(addr, cpu.read_reg(rt))
        elif size == 0b01:   # STRH
            cpu.write16(addr, cpu.read_reg(rt) & MASK16)
        elif size == 0b00:   # STRB
            cpu.write8(addr, cpu.read_reg(rt) & MASK8)


# ── Main step logic ────────────────────────────────────────────────────────────


def _step(cpu: _CPU) -> None:
    """
    Execute one instruction from the current PC.

    Thumb-2 instruction width detection (ARM DDI 0406C §A6.1):
      If bits [15:11] of the first halfword are in {11101, 11110, 11111}
      (i.e., bits[15:13]==111 AND bits[12:11]!=00), it is a 32-bit instruction.
      Otherwise it is a 16-bit instruction.
    """
    hw1 = cpu.fetch16()   # Always fetch the first halfword; PC advances by 2

    # Check for halt sentinel
    if hw1 == 0x0000:
        cpu.halted = True
        return

    # Determine instruction width
    top5 = (hw1 >> 11) & 0x1F
    is_32bit = top5 in (0b11101, 0b11110, 0b11111)

    if is_32bit:
        hw2 = cpu.fetch16()   # Fetch second halfword; PC advances by 2 more
        _exec_32(cpu, hw1, hw2)
    else:
        _exec_16(cpu, hw1)


# ── Public Simulator class ─────────────────────────────────────────────────────


class ARMv7ASimulator:
    """
    ARMv7-A / Thumb-2 behavioral simulator implementing the SIM00 protocol.

    Usage
    -----
        sim = ARMv7ASimulator()
        state = sim.execute(program_bytes)
        print(state.r0)

    The simulator starts in Thumb mode (CPSR.T=1) with SP=0xFFF8 and all other
    registers and memory zeroed.
    """

    def __init__(self) -> None:
        self._cpu = _CPU()
        self.reset()

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers and memory; set SP=0xFFF8, CPSR.T=1, PC=0."""
        cpu = self._cpu
        for i in range(NUM_REGS):
            cpu.gpr[i] = 0
        cpu.pc = 0
        cpu.halted = False
        # CPSR: set T=1 (Thumb mode), all flags cleared
        cpu.cpsr = 1 << CPSR_T
        # SP = top-of-memory - 8 (same convention as other simulators)
        cpu.gpr[SP] = 0xFFF8
        # Zero memory
        for i in range(MEM_SIZE):
            cpu.memory[i] = 0

    def load(self, program: bytes) -> None:
        """Reset and copy program bytes to memory[0..]."""
        self.reset()
        cpu = self._cpu
        for i, b in enumerate(program):
            if i >= MEM_SIZE:
                break
            cpu.memory[i] = b

    def get_state(self) -> ARMv7AState:
        """Return a frozen snapshot of the current simulator state."""
        cpu = self._cpu
        return ARMv7AState(
            pc=cpu.pc,
            gpr=tuple(cpu.gpr),
            cpsr=cpu.cpsr,
            memory=tuple(cpu.memory),
            halted=cpu.halted,
        )

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace."""
        cpu = self._cpu
        if cpu.halted:
            return StepTrace(
                pc_before=cpu.pc,
                pc_after=cpu.pc,
                halted=True,
            )
        pc_before = cpu.pc
        _step(cpu)
        return StepTrace(
            pc_before=pc_before,
            pc_after=cpu.pc,
            halted=cpu.halted,
        )

    def execute(self, program: bytes, max_steps: int = 100_000) -> ARMv7AState:
        """Load program and run until halted or max_steps reached."""
        self.load(program)
        cpu = self._cpu
        for _ in range(max_steps):
            if cpu.halted:
                break
            _step(cpu)
        return self.get_state()

    # ── I/O stubs (SIM00 completeness) ────────────────────────────────────────

    def set_input_port(self, port: int, value: int) -> None:
        """Stub — ARMv7-A has no memory-mapped I/O in this simulation."""

    def get_output_port(self, port: int) -> int:
        """Stub — returns 0."""
        return 0

    def interrupt(self, vector: int) -> None:
        """Stub — interrupts not modeled."""

    def nmi(self) -> None:
        """Stub — NMI not modeled."""
