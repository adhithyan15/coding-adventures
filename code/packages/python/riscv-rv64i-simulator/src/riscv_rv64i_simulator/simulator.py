"""
RISC-V RV64I + M Extension Behavioral Simulator
================================================

RISC-V is a clean, load-store RISC ISA with:
  - 32 general-purpose 64-bit integer registers (x0–x31)
  - x0 hardwired to zero; writes are silently ignored
  - Fixed 32-bit instruction width, always 4-byte aligned
  - Little-endian memory
  - No condition codes; comparisons produce 0/1 in a destination register

Instruction encoding uses six formats (R/I/S/B/U/J).  The opcode in bits[6:0]
uniquely identifies the format.  Because the two LSBs are always 0b11 for
32-bit instructions, fetching a word of 0x00000000 (which has LSBs=00) is used
as the halt sentinel.

This module implements:
  - Full RV64I base integer instruction set
  - M extension: integer multiply and divide (both 64-bit and 32-bit word forms)
  - ECALL/EBREAK treated as halt

Not implemented (out of scope for behavioral compiler testing):
  - F/D floating-point extensions
  - A atomic extension
  - C compressed (16-bit) extension
  - Privilege modes, CSRs, MMU, interrupts
"""

from __future__ import annotations

from dataclasses import dataclass

from .state import (
    MASK8,
    MASK16,
    MASK32,
    MASK64,
    MEM_SIZE,
    NUM_REGS,
    SP,
    RV64IState,
    sext,
    sext12,
    sext32_to_64,
    to_signed32,
    to_signed64,
)

# ── Step trace ─────────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class StepTrace:
    """
    Record of a single instruction execution, returned by RV64ISimulator.step().

    Attributes
    ----------
    pc_before   Address from which the instruction was fetched.
    pc_after    PC value after the instruction executed (next fetch address).
    halted      True if the simulator halted (fetched the zero sentinel).
    """

    pc_before: int
    pc_after:  int
    halted:    bool


# ── Opcode constants ───────────────────────────────────────────────────────────
# All opcodes have the two LSBs == 0b11 for standard 32-bit instructions.

OP_LUI:    int = 0x37   # 0110111 — Load Upper Immediate
OP_AUIPC:  int = 0x17   # 0010111 — Add Upper Immediate to PC
OP_JAL:    int = 0x6F   # 1101111 — Jump And Link
OP_JALR:   int = 0x67   # 1100111 — Jump And Link Register
OP_BRANCH: int = 0x63   # 1100011 — Conditional branches
OP_LOAD:   int = 0x03   # 0000011 — Load (LB/LH/LW/LD/LBU/LHU/LWU)
OP_STORE:  int = 0x23   # 0100011 — Store (SB/SH/SW/SD)
OP_ALUI:   int = 0x13   # 0010011 — ALU immediate (ADDI/SLTI/…)
OP_ALUR:   int = 0x33   # 0110011 — ALU register (ADD/SUB/…) + M-ext (mul/div)
OP_ALUIW:  int = 0x1B   # 0011011 — RV64I word ALU immediate (ADDIW/SLLIW/…)
OP_ALURW:  int = 0x3B   # 0111011 — RV64I word ALU register (ADDW/SUBW/…)
OP_FENCE:  int = 0x0F   # 0001111 — Memory fence (NOP in this simulator)
OP_SYSTEM: int = 0x73   # 1110011 — ECALL / EBREAK / CSR (halt here)

# funct3 for branch instructions
FUNCT3_BEQ:  int = 0b000
FUNCT3_BNE:  int = 0b001
FUNCT3_BLT:  int = 0b100
FUNCT3_BGE:  int = 0b101
FUNCT3_BLTU: int = 0b110
FUNCT3_BGEU: int = 0b111

# funct3 for load instructions
FUNCT3_LB:  int = 0b000
FUNCT3_LH:  int = 0b001
FUNCT3_LW:  int = 0b010
FUNCT3_LD:  int = 0b011
FUNCT3_LBU: int = 0b100
FUNCT3_LHU: int = 0b101
FUNCT3_LWU: int = 0b110

# funct3 for store instructions
FUNCT3_SB: int = 0b000
FUNCT3_SH: int = 0b001
FUNCT3_SW: int = 0b010
FUNCT3_SD: int = 0b011

# funct3 for ALU immediate
FUNCT3_ADDI:      int = 0b000
FUNCT3_SLTI:      int = 0b010
FUNCT3_SLTIU:     int = 0b011
FUNCT3_XORI:      int = 0b100
FUNCT3_ORI:       int = 0b110
FUNCT3_ANDI:      int = 0b111
FUNCT3_SLLI:      int = 0b001
FUNCT3_SRLI_SRAI: int = 0b101

# funct3 for ALU register (also used for M-extension)
FUNCT3_ADD_SUB:   int = 0b000   # funct7 distinguishes ADD(0) vs SUB(0x20)
FUNCT3_SLL:       int = 0b001
FUNCT3_SLT:       int = 0b010
FUNCT3_SLTU:      int = 0b011
FUNCT3_XOR:       int = 0b100
FUNCT3_SRL_SRA:   int = 0b101   # funct7 distinguishes SRL(0) vs SRA(0x20)
FUNCT3_OR:        int = 0b110
FUNCT3_AND:       int = 0b111
FUNCT3_MUL:       int = 0b000   # M-ext (funct7=0b0000001)
FUNCT3_MULH:      int = 0b001
FUNCT3_MULHSU:    int = 0b010
FUNCT3_MULHU:     int = 0b011
FUNCT3_DIV:       int = 0b100
FUNCT3_DIVU:      int = 0b101
FUNCT3_REM:       int = 0b110
FUNCT3_REMU:      int = 0b111

FUNCT7_NORMAL: int = 0x00   # standard R-type
FUNCT7_ALT:    int = 0x20   # SUB / SRA
FUNCT7_MEXT:   int = 0x01   # M extension


# ── CPU mutable state ──────────────────────────────────────────────────────────


class _CPU:
    """
    Mutable simulator state owned by RV64ISimulator.

    Kept mutable for efficiency during execution; external callers only see the
    frozen RV64IState snapshots returned by get_state().
    """

    __slots__ = ("gpr", "memory", "pc", "halted")

    def __init__(self) -> None:
        self.gpr: list[int] = [0] * NUM_REGS    # x0–x31, all 64-bit
        self.memory: bytearray = bytearray(MEM_SIZE)
        self.pc: int = 0
        self.halted: bool = False

    # ── Register access ───────────────────────────────────────────────────────

    def read_reg(self, r: int) -> int:
        """Read register r.  x0 always returns 0."""
        return 0 if r == 0 else self.gpr[r] & MASK64

    def write_reg(self, r: int, val: int) -> None:
        """Write register r.  Writes to x0 are silently discarded."""
        if r != 0:
            self.gpr[r] = val & MASK64

    # ── Memory helpers ────────────────────────────────────────────────────────

    def read8(self, addr: int) -> int:
        return self.memory[addr & 0xFFFF]

    def write8(self, addr: int, val: int) -> None:
        self.memory[addr & 0xFFFF] = val & 0xFF

    def read16(self, addr: int) -> int:
        a = addr & 0xFFFF
        return self.memory[a] | (self.memory[(a + 1) & 0xFFFF] << 8)

    def write16(self, addr: int, val: int) -> None:
        a = addr & 0xFFFF
        self.memory[a] = val & 0xFF
        self.memory[(a + 1) & 0xFFFF] = (val >> 8) & 0xFF

    def read32(self, addr: int) -> int:
        a = addr & 0xFFFF
        return (self.memory[a]
                | (self.memory[(a + 1) & 0xFFFF] << 8)
                | (self.memory[(a + 2) & 0xFFFF] << 16)
                | (self.memory[(a + 3) & 0xFFFF] << 24))

    def write32(self, addr: int, val: int) -> None:
        a = addr & 0xFFFF
        self.memory[a]                    = val & 0xFF
        self.memory[(a + 1) & 0xFFFF]    = (val >> 8)  & 0xFF
        self.memory[(a + 2) & 0xFFFF]    = (val >> 16) & 0xFF
        self.memory[(a + 3) & 0xFFFF]    = (val >> 24) & 0xFF

    def read64(self, addr: int) -> int:
        a = addr & 0xFFFF
        lo = self.read32(a)
        hi = self.read32((a + 4) & 0xFFFF)
        return (hi << 32) | lo

    def write64(self, addr: int, val: int) -> None:
        a = addr & 0xFFFF
        self.write32(a, val & MASK32)
        self.write32((a + 4) & 0xFFFF, (val >> 32) & MASK32)

    def fetch32(self) -> int:
        """Fetch a 32-bit instruction at PC and advance PC by 4."""
        instr = self.read32(self.pc)
        self.pc = (self.pc + 4) & MASK64
        return instr


# ── Immediate decoding ─────────────────────────────────────────────────────────


def _decode_i_imm(instr: int) -> int:
    """Decode and sign-extend a 12-bit I-type immediate."""
    return sext12(instr >> 20)


def _decode_s_imm(instr: int) -> int:
    """Decode and sign-extend a 12-bit S-type immediate."""
    hi = (instr >> 25) & 0x7F   # bits [11:5]
    lo = (instr >> 7)  & 0x1F   # bits [4:0]
    return sext12((hi << 5) | lo)


def _decode_b_imm(instr: int) -> int:
    """
    Decode and sign-extend a 13-bit B-type immediate.

    The B-type scatters the immediate across the word to keep register fields
    in the same position as the S-type:
      bit[12] = instr[31]
      bit[11] = instr[7]
      bits[10:5] = instr[30:25]
      bits[4:1] = instr[11:8]
      bit[0] = 0 (always, since branches target aligned addresses)
    """
    b12  = (instr >> 31) & 1
    b11  = (instr >> 7)  & 1
    b105 = (instr >> 25) & 0x3F
    b41  = (instr >> 8)  & 0xF
    raw  = (b12 << 12) | (b11 << 11) | (b105 << 5) | (b41 << 1)
    return sext(raw, 13)


def _decode_u_imm(instr: int) -> int:
    """
    Decode a U-type immediate (upper 20 bits, sign-extended to 64 bits).

    LUI / AUIPC use this form.  The result is already shifted left 12 bits.
    """
    raw = instr & 0xFFFFF000   # bits[31:12] << 12, bits[11:0] = 0
    return sext(raw, 32)


def _decode_j_imm(instr: int) -> int:
    """
    Decode and sign-extend a 21-bit J-type immediate (JAL).

    Bit layout:
      bit[20]    = instr[31]
      bits[10:1] = instr[30:21]
      bit[11]    = instr[20]
      bits[19:12]= instr[19:12]
      bit[0]     = 0
    """
    b20   = (instr >> 31) & 1
    b1910 = (instr >> 21) & 0x3FF
    b11   = (instr >> 20) & 1
    b1912 = (instr >> 12) & 0xFF
    raw   = (b20 << 20) | (b1912 << 12) | (b11 << 11) | (b1910 << 1)
    return sext(raw, 21)


# ── Instruction execution ──────────────────────────────────────────────────────


def _exec_lui(cpu: _CPU, instr: int) -> None:
    """LUI rd, imm20 — load upper 20 bits into rd; lower 12 bits = 0."""
    rd  = (instr >> 7) & 0x1F
    imm = _decode_u_imm(instr)
    cpu.write_reg(rd, imm & MASK64)


def _exec_auipc(cpu: _CPU, instr: int, pc_at_fetch: int) -> None:
    """AUIPC rd, imm20 — rd = PC + (imm20 << 12)."""
    rd  = (instr >> 7) & 0x1F
    imm = _decode_u_imm(instr)
    cpu.write_reg(rd, (pc_at_fetch + imm) & MASK64)


def _exec_jal(cpu: _CPU, instr: int, pc_at_fetch: int) -> None:
    """
    JAL rd, offset — unconditional jump with link.

    rd = PC + 4  (return address, one instruction past JAL)
    PC = PC + sign_extend(offset, 21)
    """
    rd     = (instr >> 7) & 0x1F
    offset = _decode_j_imm(instr)
    ret    = (pc_at_fetch + 4) & MASK64
    cpu.write_reg(rd, ret)
    cpu.pc = (pc_at_fetch + offset) & MASK64


def _exec_jalr(cpu: _CPU, instr: int, pc_at_fetch: int) -> None:
    """
    JALR rd, rs1, imm — indirect jump with link.

    rd = PC + 4
    PC = (rs1 + sign_extend(imm,12)) & ~1  (clear LSB)
    """
    rd  = (instr >> 7)  & 0x1F
    rs1 = (instr >> 15) & 0x1F
    imm = _decode_i_imm(instr)
    ret    = (pc_at_fetch + 4) & MASK64
    target = (cpu.read_reg(rs1) + imm) & MASK64 & ~1
    cpu.write_reg(rd, ret)
    cpu.pc = target


def _exec_branch(cpu: _CPU, instr: int, pc_at_fetch: int) -> None:
    """
    Conditional branches (BEQ/BNE/BLT/BGE/BLTU/BGEU).

    Branch target = PC + sign_extend(imm, 13).
    No branch-delay slot (unlike MIPS).
    """
    funct3 = (instr >> 12) & 0x7
    rs1    = (instr >> 15) & 0x1F
    rs2    = (instr >> 20) & 0x1F
    offset = _decode_b_imm(instr)

    a = cpu.read_reg(rs1)
    b = cpu.read_reg(rs2)

    # Unsigned comparisons use the 64-bit unsigned values directly.
    # Signed comparisons need to interpret the values as signed.
    taken = False
    if funct3 == FUNCT3_BEQ:
        taken = a == b
    elif funct3 == FUNCT3_BNE:
        taken = a != b
    elif funct3 == FUNCT3_BLT:
        taken = to_signed64(a) < to_signed64(b)
    elif funct3 == FUNCT3_BGE:
        taken = to_signed64(a) >= to_signed64(b)
    elif funct3 == FUNCT3_BLTU:
        taken = a < b
    elif funct3 == FUNCT3_BGEU:
        taken = a >= b

    if taken:
        cpu.pc = (pc_at_fetch + offset) & MASK64


def _exec_load(cpu: _CPU, instr: int) -> None:
    """
    Load instructions (LB/LH/LW/LD/LBU/LHU/LWU).

    Address = rs1 + sign_extend(imm, 12).
    """
    funct3 = (instr >> 12) & 0x7
    rd     = (instr >> 7)  & 0x1F
    rs1    = (instr >> 15) & 0x1F
    imm    = _decode_i_imm(instr)
    addr   = (cpu.read_reg(rs1) + imm) & MASK64

    if funct3 == FUNCT3_LB:
        val = sext(cpu.read8(addr), 8) & MASK64
    elif funct3 == FUNCT3_LH:
        val = sext(cpu.read16(addr), 16) & MASK64
    elif funct3 == FUNCT3_LW:
        val = sext(cpu.read32(addr), 32) & MASK64
    elif funct3 == FUNCT3_LD:
        val = cpu.read64(addr) & MASK64
    elif funct3 == FUNCT3_LBU:
        val = cpu.read8(addr)
    elif funct3 == FUNCT3_LHU:
        val = cpu.read16(addr)
    elif funct3 == FUNCT3_LWU:
        val = cpu.read32(addr) & MASK32
    else:
        return   # undefined funct3 — treat as NOP

    cpu.write_reg(rd, val)


def _exec_store(cpu: _CPU, instr: int) -> None:
    """
    Store instructions (SB/SH/SW/SD).

    Address = rs1 + sign_extend(imm, 12).
    """
    funct3 = (instr >> 12) & 0x7
    rs1    = (instr >> 15) & 0x1F
    rs2    = (instr >> 20) & 0x1F
    imm    = _decode_s_imm(instr)
    addr   = (cpu.read_reg(rs1) + imm) & MASK64
    val    = cpu.read_reg(rs2)

    if funct3 == FUNCT3_SB:
        cpu.write8(addr, val & MASK8)
    elif funct3 == FUNCT3_SH:
        cpu.write16(addr, val & MASK16)
    elif funct3 == FUNCT3_SW:
        cpu.write32(addr, val & MASK32)
    elif funct3 == FUNCT3_SD:
        cpu.write64(addr, val)


def _exec_alui(cpu: _CPU, instr: int) -> None:
    """
    ALU immediate instructions (ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI,
    SRLI, SRAI).

    Encoding: I-type; imm is sign-extended 12-bit.
    For shifts, the shift amount (shamt) is in bits[25:20] (6 bits for RV64I).
    """
    funct3 = (instr >> 12) & 0x7
    rd     = (instr >> 7)  & 0x1F
    rs1    = (instr >> 15) & 0x1F
    a      = cpu.read_reg(rs1)

    if funct3 == FUNCT3_ADDI:
        imm = _decode_i_imm(instr)
        cpu.write_reg(rd, (a + imm) & MASK64)

    elif funct3 == FUNCT3_SLTI:
        imm = _decode_i_imm(instr)
        cpu.write_reg(rd, 1 if to_signed64(a) < imm else 0)

    elif funct3 == FUNCT3_SLTIU:
        # Compare unsigned; imm is still sign-extended then treated as unsigned
        imm = _decode_i_imm(instr) & MASK64
        cpu.write_reg(rd, 1 if a < imm else 0)

    elif funct3 == FUNCT3_XORI:
        imm = _decode_i_imm(instr) & MASK64
        cpu.write_reg(rd, (a ^ imm) & MASK64)

    elif funct3 == FUNCT3_ORI:
        imm = _decode_i_imm(instr) & MASK64
        cpu.write_reg(rd, (a | imm) & MASK64)

    elif funct3 == FUNCT3_ANDI:
        imm = _decode_i_imm(instr) & MASK64
        cpu.write_reg(rd, (a & imm) & MASK64)

    elif funct3 == FUNCT3_SLLI:
        # For RV64I, shamt is bits[25:20] (6 bits)
        shamt = (instr >> 20) & 0x3F
        cpu.write_reg(rd, (a << shamt) & MASK64)

    elif funct3 == FUNCT3_SRLI_SRAI:
        shamt  = (instr >> 20) & 0x3F
        funct7 = (instr >> 25) & 0x7F
        if funct7 & 0x20:   # SRAI: arithmetic (sign-preserving)
            signed_a = to_signed64(a)
            cpu.write_reg(rd, (signed_a >> shamt) & MASK64)
        else:               # SRLI: logical (zero-fill)
            cpu.write_reg(rd, (a >> shamt) & MASK64)


def _exec_alur(cpu: _CPU, instr: int) -> None:
    """
    ALU register instructions (ADD, SUB, SLL, SLT, SLTU, XOR, SRL, SRA, OR,
    AND) plus M-extension (MUL, MULH, MULHSU, MULHU, DIV, DIVU, REM, REMU).

    funct7 = 0x00: standard ops
    funct7 = 0x20: ALT ops (SUB, SRA)
    funct7 = 0x01: M extension
    """
    funct3 = (instr >> 12) & 0x7
    funct7 = (instr >> 25) & 0x7F
    rd     = (instr >> 7)  & 0x1F
    rs1    = (instr >> 15) & 0x1F
    rs2    = (instr >> 20) & 0x1F
    a      = cpu.read_reg(rs1)
    b      = cpu.read_reg(rs2)

    if funct7 == FUNCT7_MEXT:
        _exec_mext_64(cpu, rd, funct3, a, b)
        return

    # Standard ALU register
    if funct3 == FUNCT3_ADD_SUB:
        if funct7 & 0x20:
            cpu.write_reg(rd, (a - b) & MASK64)   # SUB
        else:
            cpu.write_reg(rd, (a + b) & MASK64)   # ADD

    elif funct3 == FUNCT3_SLL:
        cpu.write_reg(rd, (a << (b & 63)) & MASK64)

    elif funct3 == FUNCT3_SLT:
        cpu.write_reg(rd, 1 if to_signed64(a) < to_signed64(b) else 0)

    elif funct3 == FUNCT3_SLTU:
        cpu.write_reg(rd, 1 if a < b else 0)

    elif funct3 == FUNCT3_XOR:
        cpu.write_reg(rd, (a ^ b) & MASK64)

    elif funct3 == FUNCT3_SRL_SRA:
        shamt = b & 63
        if funct7 & 0x20:
            cpu.write_reg(rd, (to_signed64(a) >> shamt) & MASK64)   # SRA
        else:
            cpu.write_reg(rd, (a >> shamt) & MASK64)                 # SRL

    elif funct3 == FUNCT3_OR:
        cpu.write_reg(rd, (a | b) & MASK64)

    elif funct3 == FUNCT3_AND:
        cpu.write_reg(rd, (a & b) & MASK64)


def _exec_mext_64(cpu: _CPU, rd: int, funct3: int, a: int, b: int) -> None:
    """
    M-extension 64-bit multiply and divide.

    Multiply produces a 128-bit product; MUL takes the lower 64 bits,
    MULH/MULHSU/MULHU take the upper 64 bits.

    Division-by-zero returns -1 (signed) / MAXUINT (unsigned) for the quotient
    and the dividend for the remainder, per the RISC-V specification.
    """
    if funct3 == FUNCT3_MUL:
        # Lower 64 bits of 64-bit × 64-bit product
        cpu.write_reg(rd, (a * b) & MASK64)

    elif funct3 == FUNCT3_MULH:
        # Upper 64 bits of signed × signed 128-bit product
        result = (to_signed64(a) * to_signed64(b)) >> 64
        cpu.write_reg(rd, result & MASK64)

    elif funct3 == FUNCT3_MULHSU:
        # Upper 64 bits of signed × unsigned 128-bit product
        result = (to_signed64(a) * b) >> 64
        cpu.write_reg(rd, result & MASK64)

    elif funct3 == FUNCT3_MULHU:
        # Upper 64 bits of unsigned × unsigned 128-bit product
        result = (a * b) >> 64
        cpu.write_reg(rd, result & MASK64)

    elif funct3 == FUNCT3_DIV:
        # Signed division, truncated toward zero
        sa, sb = to_signed64(a), to_signed64(b)
        if sb == 0:
            cpu.write_reg(rd, MASK64)   # -1 as unsigned 64-bit
        elif sa == -(1 << 63) and sb == -1:
            cpu.write_reg(rd, a)        # overflow: quotient = dividend
        else:
            # Python truncates toward negative infinity; we need toward zero
            result = int(sa / sb)
            cpu.write_reg(rd, result & MASK64)

    elif funct3 == FUNCT3_DIVU:
        if b == 0:
            cpu.write_reg(rd, MASK64)
        else:
            cpu.write_reg(rd, (a // b) & MASK64)

    elif funct3 == FUNCT3_REM:
        sa, sb = to_signed64(a), to_signed64(b)
        if sb == 0:
            cpu.write_reg(rd, a)        # remainder = dividend
        elif sa == -(1 << 63) and sb == -1:
            cpu.write_reg(rd, 0)        # overflow: remainder = 0
        else:
            # RISC-V: sign of remainder matches dividend
            result = int(sa / sb)
            cpu.write_reg(rd, (sa - result * sb) & MASK64)

    elif funct3 == FUNCT3_REMU:
        if b == 0:
            cpu.write_reg(rd, a)
        else:
            cpu.write_reg(rd, (a % b) & MASK64)


def _exec_aluiw(cpu: _CPU, instr: int) -> None:
    """
    RV64I word ALU immediate (opcode 0x1B).

    Operate on lower 32 bits of rs1 with a sign-extended 12-bit immediate,
    then sign-extend the 32-bit result to 64 bits.

    Instructions: ADDIW, SLLIW, SRLIW, SRAIW.
    """
    funct3 = (instr >> 12) & 0x7
    rd     = (instr >> 7)  & 0x1F
    rs1    = (instr >> 15) & 0x1F
    a32    = cpu.read_reg(rs1) & MASK32

    if funct3 == FUNCT3_ADDI:   # ADDIW
        imm = _decode_i_imm(instr)
        cpu.write_reg(rd, sext32_to_64((a32 + imm) & MASK32) & MASK64)

    elif funct3 == FUNCT3_SLLI:   # SLLIW — shamt is bits[24:20] (5-bit)
        shamt = (instr >> 20) & 0x1F
        cpu.write_reg(rd, sext32_to_64((a32 << shamt) & MASK32) & MASK64)

    elif funct3 == FUNCT3_SRLI_SRAI:   # SRLIW / SRAIW
        shamt  = (instr >> 20) & 0x1F
        funct7 = (instr >> 25) & 0x7F
        if funct7 & 0x20:   # SRAIW: arithmetic
            signed_a32 = to_signed32(a32)
            cpu.write_reg(rd, sext32_to_64((signed_a32 >> shamt) & MASK32) & MASK64)
        else:               # SRLIW: logical
            cpu.write_reg(rd, sext32_to_64((a32 >> shamt) & MASK32) & MASK64)


def _exec_alurw(cpu: _CPU, instr: int) -> None:
    """
    RV64I word ALU register (opcode 0x3B).

    Operate on lower 32 bits of rs1 and rs2, then sign-extend result to 64
    bits.  Includes M-extension word multiply/divide.

    Instructions: ADDW, SUBW, SLLW, SRLW, SRAW, MULW, DIVW, DIVUW, REMW,
    REMUW.
    """
    funct3 = (instr >> 12) & 0x7
    funct7 = (instr >> 25) & 0x7F
    rd     = (instr >> 7)  & 0x1F
    rs1    = (instr >> 15) & 0x1F
    rs2    = (instr >> 20) & 0x1F
    a32    = cpu.read_reg(rs1) & MASK32
    b32    = cpu.read_reg(rs2) & MASK32

    if funct7 == FUNCT7_MEXT:
        _exec_mext_word(cpu, rd, funct3, a32, b32)
        return

    if funct3 == FUNCT3_ADD_SUB:
        if funct7 & 0x20:   # SUBW
            cpu.write_reg(rd, sext32_to_64((a32 - b32) & MASK32) & MASK64)
        else:                # ADDW
            cpu.write_reg(rd, sext32_to_64((a32 + b32) & MASK32) & MASK64)

    elif funct3 == FUNCT3_SLL:   # SLLW — shift by rs2[4:0]
        shamt = b32 & 31
        cpu.write_reg(rd, sext32_to_64((a32 << shamt) & MASK32) & MASK64)

    elif funct3 == FUNCT3_SRL_SRA:
        shamt = b32 & 31
        if funct7 & 0x20:   # SRAW
            cpu.write_reg(rd, sext32_to_64((to_signed32(a32) >> shamt) & MASK32) & MASK64)
        else:                # SRLW
            cpu.write_reg(rd, sext32_to_64((a32 >> shamt) & MASK32) & MASK64)


def _exec_mext_word(cpu: _CPU, rd: int, funct3: int, a32: int, b32: int) -> None:
    """M-extension 32-bit word multiply and divide, results sign-extended to 64."""
    if funct3 == FUNCT3_MUL:   # MULW
        cpu.write_reg(rd, sext32_to_64((a32 * b32) & MASK32) & MASK64)

    elif funct3 == FUNCT3_DIV:   # DIVW
        sa, sb = to_signed32(a32), to_signed32(b32)
        if sb == 0:
            cpu.write_reg(rd, MASK64)
        elif sa == -(1 << 31) and sb == -1:
            cpu.write_reg(rd, sext32_to_64(a32) & MASK64)
        else:
            cpu.write_reg(rd, sext32_to_64(int(sa / sb) & MASK32) & MASK64)

    elif funct3 == FUNCT3_DIVU:   # DIVUW
        if b32 == 0:
            cpu.write_reg(rd, MASK64)
        else:
            cpu.write_reg(rd, sext32_to_64((a32 // b32) & MASK32) & MASK64)

    elif funct3 == FUNCT3_REM:   # REMW
        sa, sb = to_signed32(a32), to_signed32(b32)
        if sb == 0:
            cpu.write_reg(rd, sext32_to_64(a32) & MASK64)
        elif sa == -(1 << 31) and sb == -1:
            cpu.write_reg(rd, 0)
        else:
            result = int(sa / sb)
            cpu.write_reg(rd, sext32_to_64((sa - result * sb) & MASK32) & MASK64)

    elif funct3 == FUNCT3_REMU:   # REMUW
        if b32 == 0:
            cpu.write_reg(rd, sext32_to_64(a32) & MASK64)
        else:
            cpu.write_reg(rd, sext32_to_64((a32 % b32) & MASK32) & MASK64)


# ── Main step logic ────────────────────────────────────────────────────────────


def _step(cpu: _CPU) -> None:
    """
    Fetch and execute one instruction.

    RISC-V has a fixed 32-bit instruction width.  The all-zero word 0x00000000
    is the halt sentinel (it decodes to ADDI x0, x0, 0 which would be a NOP,
    but we treat it as halt to simplify the protocol).
    """
    pc_before = cpu.pc
    instr = cpu.fetch32()   # fetches 4 bytes; advances PC by 4

    # Halt sentinel: any all-zero word
    if instr == 0x0000_0000:
        cpu.halted = True
        return

    opcode = instr & 0x7F

    if opcode == OP_LUI:
        _exec_lui(cpu, instr)

    elif opcode == OP_AUIPC:
        _exec_auipc(cpu, instr, pc_before)

    elif opcode == OP_JAL:
        _exec_jal(cpu, instr, pc_before)

    elif opcode == OP_JALR:
        _exec_jalr(cpu, instr, pc_before)

    elif opcode == OP_BRANCH:
        _exec_branch(cpu, instr, pc_before)

    elif opcode == OP_LOAD:
        _exec_load(cpu, instr)

    elif opcode == OP_STORE:
        _exec_store(cpu, instr)

    elif opcode == OP_ALUI:
        _exec_alui(cpu, instr)

    elif opcode == OP_ALUR:
        _exec_alur(cpu, instr)

    elif opcode == OP_ALUIW:
        _exec_aluiw(cpu, instr)

    elif opcode == OP_ALURW:
        _exec_alurw(cpu, instr)

    elif opcode == OP_FENCE:
        pass   # NOP — no memory ordering model in behavioral simulator

    elif opcode == OP_SYSTEM:
        # ECALL (imm=0) and EBREAK (imm=1) both halt
        cpu.halted = True


# ── Public Simulator class ─────────────────────────────────────────────────────


class RV64ISimulator:
    """
    RISC-V RV64I + M extension behavioral simulator implementing SIM00 protocol.

    Usage
    -----
        sim = RV64ISimulator()
        state = sim.execute(program_bytes)
        print(state.a0)  # return value in a0 (x10)

    The simulator starts with SP=0xFFF8 and all other registers and memory
    zeroed.  PC starts at 0.
    """

    def __init__(self) -> None:
        self._cpu = _CPU()
        self.reset()

    # ── SIM00 protocol ────────────────────────────────────────────────────────

    def reset(self) -> None:
        """Zero all registers and memory; set SP=0xFFF8, PC=0."""
        cpu = self._cpu
        for i in range(NUM_REGS):
            cpu.gpr[i] = 0
        cpu.pc = 0
        cpu.halted = False
        cpu.gpr[SP] = 0xFFF8
        for i in range(MEM_SIZE):
            cpu.memory[i] = 0

    def load(self, program: bytes) -> None:
        """Reset and copy program bytes into memory starting at address 0."""
        self.reset()
        cpu = self._cpu
        for i, b in enumerate(program):
            if i >= MEM_SIZE:
                break
            cpu.memory[i] = b

    def get_state(self) -> RV64IState:
        """Return a frozen snapshot of the current simulator state."""
        cpu = self._cpu
        return RV64IState(
            pc=cpu.pc,
            gpr=tuple(cpu.gpr),
            memory=tuple(cpu.memory),
            halted=cpu.halted,
        )

    def step(self) -> StepTrace:
        """Execute one instruction and return a StepTrace."""
        cpu = self._cpu
        if cpu.halted:
            return StepTrace(pc_before=cpu.pc, pc_after=cpu.pc, halted=True)
        pc_before = cpu.pc
        _step(cpu)
        return StepTrace(pc_before=pc_before, pc_after=cpu.pc, halted=cpu.halted)

    def execute(self, program: bytes, max_steps: int = 100_000) -> RV64IState:
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
        """Stub — RISC-V has no I/O in this simulation."""

    def get_output_port(self, port: int) -> int:
        """Stub — returns 0."""
        return 0

    def interrupt(self, vector: int) -> None:
        """Stub — interrupts not modeled."""

    def nmi(self) -> None:
        """Stub — NMI not modeled."""
