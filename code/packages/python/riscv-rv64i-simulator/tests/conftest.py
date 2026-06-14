"""
Shared test helpers for the RISC-V RV64I simulator test suite.

Instruction encoding follows the RISC-V Unprivileged ISA specification.
All helpers return 4-byte lists (one 32-bit instruction, little-endian).
"""

from __future__ import annotations

import struct

from riscv_rv64i_simulator import RV64ISimulator, RV64IState

# ── Test runner ────────────────────────────────────────────────────────────────


def run(instructions: list[list[int] | bytes]) -> RV64IState:
    """
    Assemble a list of encoded instructions into a program, append the halt
    sentinel (0x00000000), execute, and return the final state.
    """
    prog = b"".join(bytes(i) for i in instructions) + b"\x00\x00\x00\x00"
    sim = RV64ISimulator()
    return sim.execute(prog)


# ── Low-level packing ──────────────────────────────────────────────────────────


def w32(word: int) -> list[int]:
    """Pack a 32-bit instruction word to a little-endian byte list."""
    return list(struct.pack("<I", word & 0xFFFF_FFFF))


# ── R-type encoding ────────────────────────────────────────────────────────────


def r_type(opcode: int, funct3: int, funct7: int, rd: int, rs1: int, rs2: int) -> list[int]:
    """Encode an R-type instruction."""
    instr = (
        (funct7 & 0x7F) << 25
        | (rs2   & 0x1F) << 20
        | (rs1   & 0x1F) << 15
        | (funct3 & 0x7) << 12
        | (rd    & 0x1F) << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── I-type encoding ────────────────────────────────────────────────────────────


def i_type(opcode: int, funct3: int, rd: int, rs1: int, imm: int) -> list[int]:
    """Encode an I-type instruction (imm is a signed 12-bit value)."""
    imm12 = imm & 0xFFF
    instr = (
        (imm12  & 0xFFF) << 20
        | (rs1  & 0x1F) << 15
        | (funct3 & 0x7) << 12
        | (rd   & 0x1F) << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── S-type encoding ────────────────────────────────────────────────────────────


def s_type(opcode: int, funct3: int, rs1: int, rs2: int, imm: int) -> list[int]:
    """Encode an S-type instruction."""
    imm12 = imm & 0xFFF
    hi5 = (imm12 >> 5) & 0x7F
    lo5 = imm12 & 0x1F
    instr = (
        (hi5   & 0x7F) << 25
        | (rs2  & 0x1F) << 20
        | (rs1  & 0x1F) << 15
        | (funct3 & 0x7) << 12
        | (lo5  & 0x1F) << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── B-type encoding ────────────────────────────────────────────────────────────


def b_type(opcode: int, funct3: int, rs1: int, rs2: int, offset: int) -> list[int]:
    """
    Encode a B-type branch instruction.

    `offset` is the signed byte offset from the instruction's PC.
    It must be even (4-byte aligned in practice).
    """
    imm = offset & 0x1FFF
    b12  = (imm >> 12) & 1
    b11  = (imm >> 11) & 1
    b105 = (imm >> 5) & 0x3F
    b41  = (imm >> 1) & 0xF
    instr = (
        (b12  & 1)    << 31
        | (b105 & 0x3F) << 25
        | (rs2  & 0x1F) << 20
        | (rs1  & 0x1F) << 15
        | (funct3 & 0x7) << 12
        | (b41  & 0xF)  << 8
        | (b11  & 1)    << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── U-type encoding ────────────────────────────────────────────────────────────


def u_type(opcode: int, rd: int, imm20: int) -> list[int]:
    """Encode a U-type instruction (imm20 is the upper 20-bit value)."""
    instr = (
        (imm20 & 0xFFFFF) << 12
        | (rd  & 0x1F) << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── J-type encoding ────────────────────────────────────────────────────────────


def j_type(opcode: int, rd: int, offset: int) -> list[int]:
    """
    Encode a J-type instruction (JAL).

    `offset` is the signed byte offset from the instruction's PC.
    """
    imm = offset & 0x1FFFFF
    b20   = (imm >> 20) & 1
    b1910 = (imm >> 1)  & 0x3FF
    b11   = (imm >> 11) & 1
    b1912 = (imm >> 12) & 0xFF
    instr = (
        (b20   & 1)    << 31
        | (b1912 & 0xFF) << 12
        | (b11   & 1)   << 20
        | (b1910 & 0x3FF) << 21
        | (rd    & 0x1F) << 7
        | (opcode & 0x7F)
    )
    return w32(instr)


# ── Convenient named encoders ──────────────────────────────────────────────────

OP_LUI    = 0x37
OP_AUIPC  = 0x17
OP_JAL    = 0x6F
OP_JALR   = 0x67
OP_BRANCH = 0x63
OP_LOAD   = 0x03
OP_STORE  = 0x23
OP_ALUI   = 0x13
OP_ALUR   = 0x33
OP_ALUIW  = 0x1B
OP_ALURW  = 0x3B


def addi(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b000, rd, rs1, imm)


def addiw(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUIW, 0b000, rd, rs1, imm)


def add(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b000, 0x00, rd, rs1, rs2)


def addw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b000, 0x00, rd, rs1, rs2)


def sub(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b000, 0x20, rd, rs1, rs2)


def subw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b000, 0x20, rd, rs1, rs2)


def sll(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b001, 0x00, rd, rs1, rs2)


def srl(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b101, 0x00, rd, rs1, rs2)


def sra(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b101, 0x20, rd, rs1, rs2)


def slli(rd: int, rs1: int, shamt: int) -> list[int]:
    return i_type(OP_ALUI, 0b001, rd, rs1, shamt & 0x3F)


def srli(rd: int, rs1: int, shamt: int) -> list[int]:
    return i_type(OP_ALUI, 0b101, rd, rs1, shamt & 0x3F)


def srai(rd: int, rs1: int, shamt: int) -> list[int]:
    # RV64I SRAI: funct6 = 0b010000 = 0x10 in bits[31:26]; shamt is 6-bit.
    # imm12 = (funct6 << 6) | shamt = (0x10 << 6) | shamt = 0x400 | shamt.
    # This makes instr[31:25] (funct7) = 0x20, which the simulator checks.
    return i_type(OP_ALUI, 0b101, rd, rs1, (0x10 << 6) | (shamt & 0x3F))


def slliw(rd: int, rs1: int, shamt: int) -> list[int]:
    return i_type(OP_ALUIW, 0b001, rd, rs1, shamt & 0x1F)


def srliw(rd: int, rs1: int, shamt: int) -> list[int]:
    return i_type(OP_ALUIW, 0b101, rd, rs1, shamt & 0x1F)


def sraiw(rd: int, rs1: int, shamt: int) -> list[int]:
    return i_type(OP_ALUIW, 0b101, rd, rs1, (0x20 << 5) | (shamt & 0x1F))


def sllw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b001, 0x00, rd, rs1, rs2)


def srlw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b101, 0x00, rd, rs1, rs2)


def sraw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b101, 0x20, rd, rs1, rs2)


def and_(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b111, 0x00, rd, rs1, rs2)


def or_(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b110, 0x00, rd, rs1, rs2)


def xor(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b100, 0x00, rd, rs1, rs2)


def andi(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b111, rd, rs1, imm)


def ori(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b110, rd, rs1, imm)


def xori(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b100, rd, rs1, imm)


def slt(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b010, 0x00, rd, rs1, rs2)


def sltu(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b011, 0x00, rd, rs1, rs2)


def slti(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b010, rd, rs1, imm)


def sltiu(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_ALUI, 0b011, rd, rs1, imm)


def lui(rd: int, imm20: int) -> list[int]:
    return u_type(OP_LUI, rd, imm20)


def auipc(rd: int, imm20: int) -> list[int]:
    return u_type(OP_AUIPC, rd, imm20)


def jal(rd: int, offset: int) -> list[int]:
    return j_type(OP_JAL, rd, offset)


def jalr(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_JALR, 0b000, rd, rs1, imm)


def beq(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b000, rs1, rs2, offset)


def bne(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b001, rs1, rs2, offset)


def blt(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b100, rs1, rs2, offset)


def bge(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b101, rs1, rs2, offset)


def bltu(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b110, rs1, rs2, offset)


def bgeu(rs1: int, rs2: int, offset: int) -> list[int]:
    return b_type(OP_BRANCH, 0b111, rs1, rs2, offset)


def lw(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b010, rd, rs1, imm)


def lh(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b001, rd, rs1, imm)


def lb(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b000, rd, rs1, imm)


def ld(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b011, rd, rs1, imm)


def lbu(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b100, rd, rs1, imm)


def lhu(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b101, rd, rs1, imm)


def lwu(rd: int, rs1: int, imm: int) -> list[int]:
    return i_type(OP_LOAD, 0b110, rd, rs1, imm)


def sw(rs1: int, rs2: int, imm: int) -> list[int]:
    return s_type(OP_STORE, 0b010, rs1, rs2, imm)


def sh(rs1: int, rs2: int, imm: int) -> list[int]:
    return s_type(OP_STORE, 0b001, rs1, rs2, imm)


def sb(rs1: int, rs2: int, imm: int) -> list[int]:
    return s_type(OP_STORE, 0b000, rs1, rs2, imm)


def sd(rs1: int, rs2: int, imm: int) -> list[int]:
    return s_type(OP_STORE, 0b011, rs1, rs2, imm)


def mul(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b000, 0x01, rd, rs1, rs2)


def mulh(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b001, 0x01, rd, rs1, rs2)


def mulhu(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b011, 0x01, rd, rs1, rs2)


def mulhsu(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b010, 0x01, rd, rs1, rs2)


def div_(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b100, 0x01, rd, rs1, rs2)


def divu(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b101, 0x01, rd, rs1, rs2)


def rem(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b110, 0x01, rd, rs1, rs2)


def remu(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALUR, 0b111, 0x01, rd, rs1, rs2)


def mulw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b000, 0x01, rd, rs1, rs2)


def divw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b100, 0x01, rd, rs1, rs2)


def divuw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b101, 0x01, rd, rs1, rs2)


def remw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b110, 0x01, rd, rs1, rs2)


def remuw(rd: int, rs1: int, rs2: int) -> list[int]:
    return r_type(OP_ALURW, 0b111, 0x01, rd, rs1, rs2)


def nop() -> list[int]:
    """ADDI x0, x0, 0 — the canonical RISC-V NOP."""
    return addi(0, 0, 0)
