//! Encoding helpers for constructing machine code in tests.

use crate::opcodes::*;

fn encode_i_type(rd: u32, rs1: u32, imm: i32, funct3: u32, opcode: u32) -> u32 {
    (((imm as u32) & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
}

fn encode_r_type(rd: u32, rs1: u32, rs2: u32, funct3: u32, funct7: u32) -> u32 {
    (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_OP
}

pub fn encode_addi(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_ADDI, OPCODE_OP_IMM) }
pub fn encode_slti(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_SLTI, OPCODE_OP_IMM) }
pub fn encode_sltiu(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_SLTIU, OPCODE_OP_IMM) }
pub fn encode_xori(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_XORI, OPCODE_OP_IMM) }
pub fn encode_ori(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_ORI, OPCODE_OP_IMM) }
pub fn encode_andi(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_ANDI, OPCODE_OP_IMM) }

pub fn encode_slli(rd: u32, rs1: u32, shamt: u32) -> u32 {
    (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SLLI << 12) | (rd << 7) | OPCODE_OP_IMM
}
pub fn encode_srli(rd: u32, rs1: u32, shamt: u32) -> u32 {
    (FUNCT7_NORMAL << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM
}
pub fn encode_srai(rd: u32, rs1: u32, shamt: u32) -> u32 {
    (FUNCT7_ALT << 25) | ((shamt & 0x1F) << 20) | (rs1 << 15) | (FUNCT3_SRLI << 12) | (rd << 7) | OPCODE_OP_IMM
}

pub fn encode_add(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_NORMAL) }
pub fn encode_sub(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_ADD, FUNCT7_ALT) }
pub fn encode_sll(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_SLL, FUNCT7_NORMAL) }
pub fn encode_slt(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_SLT, FUNCT7_NORMAL) }
pub fn encode_sltu(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_SLTU, FUNCT7_NORMAL) }
pub fn encode_xor(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_XOR, FUNCT7_NORMAL) }
pub fn encode_srl(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_NORMAL) }
pub fn encode_sra(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_SRL, FUNCT7_ALT) }
pub fn encode_or(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_OR, FUNCT7_NORMAL) }
pub fn encode_and(rd: u32, rs1: u32, rs2: u32) -> u32 { encode_r_type(rd, rs1, rs2, FUNCT3_AND, FUNCT7_NORMAL) }

pub fn encode_lb(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_LB, OPCODE_LOAD) }
pub fn encode_lh(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_LH, OPCODE_LOAD) }
pub fn encode_lw(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_LW, OPCODE_LOAD) }
pub fn encode_lbu(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_LBU, OPCODE_LOAD) }
pub fn encode_lhu(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, FUNCT3_LHU, OPCODE_LOAD) }

fn encode_s_type(rs2: u32, rs1: u32, imm: i32, funct3: u32) -> u32 {
    let imm_val = (imm as u32) & 0xFFF;
    let imm_low = imm_val & 0x1F;
    let imm_high = (imm_val >> 5) & 0x7F;
    (imm_high << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (imm_low << 7) | OPCODE_STORE
}

pub fn encode_sb(rs2: u32, rs1: u32, imm: i32) -> u32 { encode_s_type(rs2, rs1, imm, FUNCT3_SB) }
pub fn encode_sh(rs2: u32, rs1: u32, imm: i32) -> u32 { encode_s_type(rs2, rs1, imm, FUNCT3_SH) }
pub fn encode_sw(rs2: u32, rs1: u32, imm: i32) -> u32 { encode_s_type(rs2, rs1, imm, FUNCT3_SW) }

fn encode_b_type(rs1: u32, rs2: u32, offset: i32, funct3: u32) -> u32 {
    let imm = (offset as u32) & 0x1FFE;
    let bit12 = (imm >> 12) & 0x1;
    let bit11 = (imm >> 11) & 0x1;
    let bits10_5 = (imm >> 5) & 0x3F;
    let bits4_1 = (imm >> 1) & 0xF;
    (bit12 << 31) | (bits10_5 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (bits4_1 << 8) | (bit11 << 7) | OPCODE_BRANCH
}

pub fn encode_beq(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BEQ) }
pub fn encode_bne(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BNE) }
pub fn encode_blt(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BLT) }
pub fn encode_bge(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BGE) }
pub fn encode_bltu(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BLTU) }
pub fn encode_bgeu(rs1: u32, rs2: u32, offset: i32) -> u32 { encode_b_type(rs1, rs2, offset, FUNCT3_BGEU) }

pub fn encode_jal(rd: u32, offset: i32) -> u32 {
    let imm = (offset as u32) & 0x1FFFFE;
    let bit20 = (imm >> 20) & 0x1;
    let bits10_1 = (imm >> 1) & 0x3FF;
    let bit11 = (imm >> 11) & 0x1;
    let bits19_12 = (imm >> 12) & 0xFF;
    (bit20 << 31) | (bits10_1 << 21) | (bit11 << 20) | (bits19_12 << 12) | (rd << 7) | OPCODE_JAL
}

pub fn encode_jalr(rd: u32, rs1: u32, imm: i32) -> u32 { encode_i_type(rd, rs1, imm, 0, OPCODE_JALR) }

pub fn encode_lui(rd: u32, imm: u32) -> u32 { ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_LUI }
pub fn encode_auipc(rd: u32, imm: u32) -> u32 { ((imm & 0xFFFFF) << 12) | (rd << 7) | OPCODE_AUIPC }

pub fn encode_ecall() -> u32 { OPCODE_SYSTEM }
pub fn encode_mret() -> u32 { (FUNCT7_MRET << 25) | (0b00010 << 20) | OPCODE_SYSTEM }

fn encode_csr(rd: u32, csr: u32, rs1: u32, funct3: u32) -> u32 {
    ((csr & 0xFFF) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | OPCODE_SYSTEM
}

pub fn encode_csrrw(rd: u32, csr: u32, rs1: u32) -> u32 { encode_csr(rd, csr, rs1, FUNCT3_CSRRW) }
pub fn encode_csrrs(rd: u32, csr: u32, rs1: u32) -> u32 { encode_csr(rd, csr, rs1, FUNCT3_CSRRS) }
pub fn encode_csrrc(rd: u32, csr: u32, rs1: u32) -> u32 { encode_csr(rd, csr, rs1, FUNCT3_CSRRC) }

/// Convert instruction words to little-endian bytes.
pub fn assemble(instructions: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(instructions.len() * 4);
    for &inst in instructions {
        result.push((inst & 0xFF) as u8);
        result.push(((inst >> 8) & 0xFF) as u8);
        result.push(((inst >> 16) & 0xFF) as u8);
        result.push(((inst >> 24) & 0xFF) as u8);
    }
    result
}
