//! Encoding helpers for constructing MIPS R2000 machine code (used by tests
//! and by `mips-r2000-encoder`, which re-exports the `encode_*` functions).

use crate::opcodes::*;

// ===========================================================================
// Format builders
// ===========================================================================

fn encode_r_type(rs: u32, rt: u32, rd: u32, shamt: u32, funct: u32) -> u32 {
    ((OP_RTYPE & 0x3F) << 26)
        | ((rs & 0x1F) << 21)
        | ((rt & 0x1F) << 16)
        | ((rd & 0x1F) << 11)
        | ((shamt & 0x1F) << 6)
        | (funct & 0x3F)
}

fn encode_i_type(op: u32, rs: u32, rt: u32, imm: i32) -> u32 {
    ((op & 0x3F) << 26) | ((rs & 0x1F) << 21) | ((rt & 0x1F) << 16) | ((imm as u32) & 0xFFFF)
}

fn encode_j_type(op: u32, target: u32) -> u32 {
    ((op & 0x3F) << 26) | (target & 0x03FF_FFFF)
}

// ===========================================================================
// R-type shifts
// ===========================================================================

pub fn encode_sll(rd: u32, rt: u32, shamt: u32) -> u32 {
    encode_r_type(0, rt, rd, shamt, FUNCT_SLL)
}
pub fn encode_srl(rd: u32, rt: u32, shamt: u32) -> u32 {
    encode_r_type(0, rt, rd, shamt, FUNCT_SRL)
}
pub fn encode_sra(rd: u32, rt: u32, shamt: u32) -> u32 {
    encode_r_type(0, rt, rd, shamt, FUNCT_SRA)
}
pub fn encode_sllv(rd: u32, rt: u32, rs: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SLLV)
}
pub fn encode_srlv(rd: u32, rt: u32, rs: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SRLV)
}
pub fn encode_srav(rd: u32, rt: u32, rs: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SRAV)
}

// ===========================================================================
// R-type jumps / syscall / hi-lo moves
// ===========================================================================

/// `JR rs` — jump to register.
pub fn encode_jr(rs: u32) -> u32 {
    encode_r_type(rs, 0, 0, 0, FUNCT_JR)
}

/// `JALR rd, rs` — jump and link register.  `rd` is usually `$ra` (31).
pub fn encode_jalr(rd: u32, rs: u32) -> u32 {
    encode_r_type(rs, 0, rd, 0, FUNCT_JALR)
}

/// `SYSCALL` — the HALT sentinel (see [`crate::opcodes::HALT_OPCODE_WORD`]).
pub fn encode_syscall() -> u32 {
    encode_r_type(0, 0, 0, 0, FUNCT_SYSCALL)
}

/// `BREAK` — software breakpoint (treated as a fault by the simulator).
pub fn encode_break() -> u32 {
    encode_r_type(0, 0, 0, 0, FUNCT_BREAK)
}

pub fn encode_mfhi(rd: u32) -> u32 {
    encode_r_type(0, 0, rd, 0, FUNCT_MFHI)
}
pub fn encode_mthi(rs: u32) -> u32 {
    encode_r_type(rs, 0, 0, 0, FUNCT_MTHI)
}
pub fn encode_mflo(rd: u32) -> u32 {
    encode_r_type(0, 0, rd, 0, FUNCT_MFLO)
}
pub fn encode_mtlo(rs: u32) -> u32 {
    encode_r_type(rs, 0, 0, 0, FUNCT_MTLO)
}

pub fn encode_mult(rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, 0, 0, FUNCT_MULT)
}
pub fn encode_multu(rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, 0, 0, FUNCT_MULTU)
}
pub fn encode_div(rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, 0, 0, FUNCT_DIV)
}
pub fn encode_divu(rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, 0, 0, FUNCT_DIVU)
}

// ===========================================================================
// R-type ALU
// ===========================================================================

pub fn encode_add(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_ADD)
}
pub fn encode_addu(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_ADDU)
}
pub fn encode_sub(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SUB)
}
pub fn encode_subu(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SUBU)
}
pub fn encode_and(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_AND)
}
pub fn encode_or(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_OR)
}
pub fn encode_xor(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_XOR)
}
pub fn encode_nor(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_NOR)
}
pub fn encode_slt(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SLT)
}
pub fn encode_sltu(rd: u32, rs: u32, rt: u32) -> u32 {
    encode_r_type(rs, rt, rd, 0, FUNCT_SLTU)
}

// ===========================================================================
// I-type branches
// ===========================================================================

/// `offset` is the raw 16-bit branch field — a signed *instruction* count
/// (not bytes).  At execution time it is sign-extended and multiplied by 4.
pub fn encode_beq(rs: u32, rt: u32, offset: i32) -> u32 {
    encode_i_type(OP_BEQ, rs, rt, offset)
}
pub fn encode_bne(rs: u32, rt: u32, offset: i32) -> u32 {
    encode_i_type(OP_BNE, rs, rt, offset)
}
pub fn encode_blez(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_BLEZ, rs, 0, offset)
}
pub fn encode_bgtz(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_BGTZ, rs, 0, offset)
}
pub fn encode_bltz(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_REGIMM, rs, REGIMM_BLTZ, offset)
}
pub fn encode_bgez(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_REGIMM, rs, REGIMM_BGEZ, offset)
}
pub fn encode_bltzal(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_REGIMM, rs, REGIMM_BLTZAL, offset)
}
pub fn encode_bgezal(rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_REGIMM, rs, REGIMM_BGEZAL, offset)
}

// ===========================================================================
// I-type arithmetic / logic / upper-immediate
// ===========================================================================

pub fn encode_addi(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_ADDI, rs, rt, imm)
}
pub fn encode_addiu(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_ADDIU, rs, rt, imm)
}
pub fn encode_slti(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_SLTI, rs, rt, imm)
}
pub fn encode_sltiu(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_SLTIU, rs, rt, imm)
}
pub fn encode_andi(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_ANDI, rs, rt, imm)
}
pub fn encode_ori(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_ORI, rs, rt, imm)
}
pub fn encode_xori(rt: u32, rs: u32, imm: i32) -> u32 {
    encode_i_type(OP_XORI, rs, rt, imm)
}
pub fn encode_lui(rt: u32, imm: u32) -> u32 {
    encode_i_type(OP_LUI, 0, rt, imm as i32)
}

// ===========================================================================
// I-type loads / stores
// ===========================================================================

pub fn encode_lb(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_LB, rs, rt, offset)
}
pub fn encode_lh(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_LH, rs, rt, offset)
}
pub fn encode_lw(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_LW, rs, rt, offset)
}
pub fn encode_lbu(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_LBU, rs, rt, offset)
}
pub fn encode_lhu(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_LHU, rs, rt, offset)
}
pub fn encode_sb(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_SB, rs, rt, offset)
}
pub fn encode_sh(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_SH, rs, rt, offset)
}
pub fn encode_sw(rt: u32, rs: u32, offset: i32) -> u32 {
    encode_i_type(OP_SW, rs, rt, offset)
}

// ===========================================================================
// J-type
// ===========================================================================

/// `target` is the 26-bit word-aligned jump target (real address >> 2).
pub fn encode_j(target: u32) -> u32 {
    encode_j_type(OP_J, target)
}
pub fn encode_jal(target: u32) -> u32 {
    encode_j_type(OP_JAL, target)
}

// ===========================================================================
// Byte-stream assembly
// ===========================================================================

/// Convert instruction words to **big-endian** bytes — MIPS R2000's default
/// byte order (unlike RISC-V/ARMv7/x86, which are little-endian).
pub fn assemble(instructions: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(instructions.len() * 4);
    for &inst in instructions {
        result.extend_from_slice(&inst.to_be_bytes());
    }
    result
}
