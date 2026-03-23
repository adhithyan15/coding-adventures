"""Encoding helpers for constructing machine code in tests.

These helpers let us write human-readable instruction encodings:

    encode_addi(1, 0, 42)   # "addi x1, x0, 42" -- much clearer than 0x02A00093

Each helper constructs the 32-bit instruction word by placing fields in
their correct bit positions according to the RISC-V encoding format.
"""

from riscv_simulator.opcodes import (
    FUNCT3_ADDI,
    FUNCT3_ANDI,
    FUNCT3_BEQ,
    FUNCT3_BGE,
    FUNCT3_BGEU,
    FUNCT3_BLT,
    FUNCT3_BLTU,
    FUNCT3_BNE,
    FUNCT3_CSRRC,
    FUNCT3_CSRRS,
    FUNCT3_CSRRW,
    FUNCT3_LB,
    FUNCT3_LBU,
    FUNCT3_LH,
    FUNCT3_LHU,
    FUNCT3_LW,
    FUNCT3_ORI,
    FUNCT3_SB,
    FUNCT3_SH,
    FUNCT3_SLLI,
    FUNCT3_SLTI,
    FUNCT3_SLTIU,
    FUNCT3_SRLI,
    FUNCT3_SW,
    FUNCT3_XORI,
    FUNCT3_ADD,
    FUNCT3_SLL,
    FUNCT3_SLT,
    FUNCT3_SLTU,
    FUNCT3_XOR,
    FUNCT3_SRL,
    FUNCT3_OR,
    FUNCT3_AND,
    FUNCT7_ALT,
    FUNCT7_MRET,
    FUNCT7_NORMAL,
    OPCODE_AUIPC,
    OPCODE_BRANCH,
    OPCODE_JAL,
    OPCODE_JALR,
    OPCODE_LOAD,
    OPCODE_LUI,
    OPCODE_OP,
    OPCODE_OP_IMM,
    OPCODE_STORE,
    OPCODE_SYSTEM,
)


# === I-type encoding helper ===

def _encode_i_type(rd: int, rs1: int, imm: int, funct3: int, opcode: int) -> int:
    return ((imm & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode


# === I-type arithmetic encoders ===

def encode_addi(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_ADDI, OPCODE_OP_IMM)


def encode_slti(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_SLTI, OPCODE_OP_IMM)


def encode_sltiu(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_SLTIU, OPCODE_OP_IMM)


def encode_xori(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_XORI, OPCODE_OP_IMM)


def encode_ori(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_ORI, OPCODE_OP_IMM)


def encode_andi(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_ANDI, OPCODE_OP_IMM)


def encode_slli(rd: int, rs1: int, shamt: int) -> int:
    return (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SLLI << 12) | (rd << 7) | OPCODE_OP_IMM


def encode_srli(rd: int, rs1: int, shamt: int) -> int:
    return (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM


def encode_srai(rd: int, rs1: int, shamt: int) -> int:
    return (FUNCT7_ALT << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM


# === R-type encoding helper ===

def _encode_r_type(rd: int, rs1: int, rs2: int, funct3: int, funct7: int) -> int:
    return (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_OP


def encode_add(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_NORMAL)


def encode_sub(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_ALT)


def encode_sll(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_SLL, FUNCT7_NORMAL)


def encode_slt(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_SLT, FUNCT7_NORMAL)


def encode_sltu(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_SLTU, FUNCT7_NORMAL)


def encode_xor(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_XOR, FUNCT7_NORMAL)


def encode_srl(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_NORMAL)


def encode_sra(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_ALT)


def encode_or(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_OR, FUNCT7_NORMAL)


def encode_and(rd: int, rs1: int, rs2: int) -> int:
    return _encode_r_type(rd, rs1, rs2, FUNCT3_AND, FUNCT7_NORMAL)


# === Load encoders (I-type) ===

def encode_lb(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_LB, OPCODE_LOAD)


def encode_lh(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_LH, OPCODE_LOAD)


def encode_lw(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_LW, OPCODE_LOAD)


def encode_lbu(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_LBU, OPCODE_LOAD)


def encode_lhu(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, FUNCT3_LHU, OPCODE_LOAD)


# === Store encoders (S-type) ===

def _encode_s_type(rs2: int, rs1: int, imm: int, funct3: int) -> int:
    imm_val = imm & 0xFFF
    imm_low = imm_val & 0x1F
    imm_high = (imm_val >> 5) & 0x7F
    return (imm_high << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm_low << 7) | OPCODE_STORE


def encode_sb(rs2: int, rs1: int, imm: int) -> int:
    return _encode_s_type(rs2, rs1, imm, FUNCT3_SB)


def encode_sh(rs2: int, rs1: int, imm: int) -> int:
    return _encode_s_type(rs2, rs1, imm, FUNCT3_SH)


def encode_sw(rs2: int, rs1: int, imm: int) -> int:
    return _encode_s_type(rs2, rs1, imm, FUNCT3_SW)


# === Branch encoders (B-type) ===

def _encode_b_type(rs1: int, rs2: int, offset: int, funct3: int) -> int:
    imm = offset & 0x1FFE  # mask to 13 bits, bit 0 forced to 0
    bit12 = (imm >> 12) & 0x1
    bit11 = (imm >> 11) & 0x1
    bits10_5 = (imm >> 5) & 0x3F
    bits4_1 = (imm >> 1) & 0xF
    return (bit12 << 31) | (bits10_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (bits4_1 << 8) | (bit11 << 7) | OPCODE_BRANCH


def encode_beq(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BEQ)


def encode_bne(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BNE)


def encode_blt(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BLT)


def encode_bge(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BGE)


def encode_bltu(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BLTU)


def encode_bgeu(rs1: int, rs2: int, offset: int) -> int:
    return _encode_b_type(rs1, rs2, offset, FUNCT3_BGEU)


# === JAL encoder (J-type) ===

def encode_jal(rd: int, offset: int) -> int:
    imm = offset & 0x1FFFFE
    bit20 = (imm >> 20) & 0x1
    bits10_1 = (imm >> 1) & 0x3FF
    bit11 = (imm >> 11) & 0x1
    bits19_12 = (imm >> 12) & 0xFF
    return (bit20 << 31) | (bits10_1 << 21) | (bit11 << 20) | (bits19_12 << 12) | (rd << 7) | OPCODE_JAL


# === JALR encoder (I-type) ===

def encode_jalr(rd: int, rs1: int, imm: int) -> int:
    return _encode_i_type(rd, rs1, imm, 0, OPCODE_JALR)


# === U-type encoders ===

def encode_lui(rd: int, imm: int) -> int:
    return ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_LUI


def encode_auipc(rd: int, imm: int) -> int:
    return ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_AUIPC


# === System instruction encoders ===

def encode_ecall() -> int:
    return OPCODE_SYSTEM


def encode_mret() -> int:
    return (FUNCT7_MRET << 25) | (0b00010 << 20) | OPCODE_SYSTEM


def _encode_csr(rd: int, csr: int, rs1: int, funct3: int) -> int:
    return ((csr & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_SYSTEM


def encode_csrrw(rd: int, csr: int, rs1: int) -> int:
    return _encode_csr(rd, csr, rs1, FUNCT3_CSRRW)


def encode_csrrs(rd: int, csr: int, rs1: int) -> int:
    return _encode_csr(rd, csr, rs1, FUNCT3_CSRRS)


def encode_csrrc(rd: int, csr: int, rs1: int) -> int:
    return _encode_csr(rd, csr, rs1, FUNCT3_CSRRC)


# === Assemble ===

def assemble(instructions: list[int]) -> bytes:
    """Convert a list of 32-bit instruction words to bytes (little-endian)."""
    result = b""
    for instr in instructions:
        result += (instr & 0xFFFFFFFF).to_bytes(4, byteorder="little")
    return result
