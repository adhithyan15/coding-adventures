//! Encoding helpers for constructing SPARC V8 machine code (used by tests
//! and by `sparc-v8-encoder`, which re-exports the `encode_*` functions).

use crate::opcodes::*;

// ===========================================================================
// Format builders
// ===========================================================================

/// Format 1 — `CALL disp30`.
fn encode_format1(disp30: i32) -> u32 {
    (OP_CALL << 30) | ((disp30 as u32) & 0x3FFF_FFFF)
}

/// Format 2, immediate-word shape — `SETHI rd, imm22` (also used for NOP).
fn encode_format2_imm(rd: u32, op2: u32, imm22: u32) -> u32 {
    (OP_FMT2 << 30) | ((rd & 0x1F) << 25) | ((op2 & 0x7) << 22) | (imm22 & 0x3F_FFFF)
}

/// Format 2, branch shape — `Bicc cond, disp22`.
fn encode_format2_bicc(cond: u32, disp22: i32) -> u32 {
    (OP_FMT2 << 30) | ((cond & 0x1F) << 25) | ((OP2_BICC & 0x7) << 22) | ((disp22 as u32) & 0x3F_FFFF)
}

/// Format 3r — register `rs2` operand (`i` bit = 0).
fn encode_format3_reg(op: u32, rd: u32, op3: u32, rs1: u32, rs2: u32) -> u32 {
    (op << 30) | ((rd & 0x1F) << 25) | ((op3 & 0x3F) << 19) | ((rs1 & 0x1F) << 14) | (rs2 & 0x1F)
}

/// Format 3i — sign-extended 13-bit immediate operand (`i` bit = 1).
fn encode_format3_imm(op: u32, rd: u32, op3: u32, rs1: u32, simm13: i32) -> u32 {
    (op << 30)
        | ((rd & 0x1F) << 25)
        | ((op3 & 0x3F) << 19)
        | ((rs1 & 0x1F) << 14)
        | (1 << 13)
        | ((simm13 as u32) & 0x1FFF)
}

// ===========================================================================
// Format 1: CALL
// ===========================================================================

/// `CALL disp30` — `%o7 = pc`; `pc += disp30 * 4`.  `disp30` is a signed
/// *instruction* count (not bytes), matching `disp22`/`disp30`'s SPARC V8
/// convention.
pub fn encode_call(disp30: i32) -> u32 {
    encode_format1(disp30)
}

// ===========================================================================
// Format 2: SETHI / Bicc / NOP
// ===========================================================================

/// `SETHI rd, imm22` — `rd = imm22 << 10` (clears the low 10 bits).
pub fn encode_sethi(rd: u32, imm22: u32) -> u32 {
    encode_format2_imm(rd, OP2_SETHI, imm22)
}

/// `Bicc cond, disp22` — branch on integer condition codes.  `disp22` is a
/// signed *instruction* count.
pub fn encode_bicc(cond: u32, disp22: i32) -> u32 {
    encode_format2_bicc(cond, disp22)
}

/// `BA disp22` — branch always.
pub fn encode_ba(disp22: i32) -> u32 {
    encode_bicc(COND_BA, disp22)
}

// ===========================================================================
// Format 3: ALU (op = OP_ALU)
// ===========================================================================

/// Generic ALU op with a register `rs2` operand.
pub fn encode_alu_reg(op3: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_format3_reg(OP_ALU, rd, op3, rs1, rs2)
}

/// Generic ALU op with a sign-extended 13-bit immediate operand.
pub fn encode_alu_imm(op3: u32, rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_format3_imm(OP_ALU, rd, op3, rs1, simm13)
}

pub fn encode_add(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_ADD, rd, rs1, rs2)
}
pub fn encode_add_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_ADD, rd, rs1, simm13)
}
pub fn encode_addcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_ADDCC, rd, rs1, rs2)
}
pub fn encode_addcc_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_ADDCC, rd, rs1, simm13)
}
pub fn encode_sub(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_SUB, rd, rs1, rs2)
}
pub fn encode_sub_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_SUB, rd, rs1, simm13)
}
pub fn encode_subcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_SUBCC, rd, rs1, rs2)
}
pub fn encode_subcc_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_SUBCC, rd, rs1, simm13)
}
pub fn encode_and(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_AND, rd, rs1, rs2)
}
pub fn encode_and_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_AND, rd, rs1, simm13)
}
pub fn encode_or(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_OR, rd, rs1, rs2)
}
pub fn encode_or_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_OR, rd, rs1, simm13)
}
pub fn encode_xor(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_XOR, rd, rs1, rs2)
}
pub fn encode_xor_imm(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_XOR, rd, rs1, simm13)
}
pub fn encode_andn(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_ANDN, rd, rs1, rs2)
}
pub fn encode_orn(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_ORN, rd, rs1, rs2)
}
pub fn encode_xnor(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_XNOR, rd, rs1, rs2)
}

pub fn encode_sll(rd: u32, rs1: u32, shamt: u32) -> u32 {
    encode_alu_imm(OP3_SLL, rd, rs1, shamt as i32)
}
pub fn encode_srl(rd: u32, rs1: u32, shamt: u32) -> u32 {
    encode_alu_imm(OP3_SRL, rd, rs1, shamt as i32)
}
pub fn encode_sra(rd: u32, rs1: u32, shamt: u32) -> u32 {
    encode_alu_imm(OP3_SRA, rd, rs1, shamt as i32)
}

pub fn encode_umul(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_UMUL, rd, rs1, rs2)
}
pub fn encode_smul(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_SMUL, rd, rs1, rs2)
}
pub fn encode_udiv(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_UDIV, rd, rs1, rs2)
}
pub fn encode_sdiv(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_SDIV, rd, rs1, rs2)
}

pub fn encode_rdy(rd: u32) -> u32 {
    encode_alu_reg(OP3_RDY, rd, 0, 0)
}
pub fn encode_wry(rs1: u32, rs2: u32) -> u32 {
    encode_alu_reg(OP3_WRY, 0, rs1, rs2)
}

/// `JMPL rd, rs1, reg_or_imm` — `rd = pc`; `pc = rs1 + src2`.
pub fn encode_jmpl(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_JMPL, rd, rs1, simm13)
}

/// `SAVE rd, rs1, reg_or_imm` — rotate CWP backward (procedure entry).
pub fn encode_save(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_SAVE, rd, rs1, simm13)
}

/// `RESTORE rd, rs1, reg_or_imm` — rotate CWP forward (procedure exit).
pub fn encode_restore(rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_alu_imm(OP3_RESTORE, rd, rs1, simm13)
}

/// `Ticc cond, rs1, reg_or_imm` — trap on integer condition.
pub fn encode_ticc(cond: u32, rs1: u32, trap_imm: u32) -> u32 {
    encode_format3_imm(OP_ALU, cond, OP3_TICC, rs1, trap_imm as i32)
}

/// `ta trap_imm` — "trap always" (`cond = COND_BA`).  `ta 0` is
/// [`crate::opcodes::HALT_WORD`], this simulator's HALT sentinel.
pub fn encode_ta(trap_imm: u32) -> u32 {
    encode_ticc(COND_BA, 0, trap_imm)
}

// ===========================================================================
// Format 3: memory (op = OP_MEM)
// ===========================================================================

fn encode_mem_reg(op3: u32, rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_format3_reg(OP_MEM, rd, op3, rs1, rs2)
}
fn encode_mem_imm(op3: u32, rd: u32, rs1: u32, simm13: i32) -> u32 {
    encode_format3_imm(OP_MEM, rd, op3, rs1, simm13)
}

pub fn encode_ld(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_LD, rd, rs1, offset)
}
pub fn encode_ld_reg(rd: u32, rs1: u32, rs2: u32) -> u32 {
    encode_mem_reg(OP3_LD, rd, rs1, rs2)
}
pub fn encode_st(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_ST, rd, rs1, offset)
}
pub fn encode_ldub(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_LDUB, rd, rs1, offset)
}
pub fn encode_ldsb(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_LDSB, rd, rs1, offset)
}
pub fn encode_lduh(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_LDUH, rd, rs1, offset)
}
pub fn encode_ldsh(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_LDSH, rd, rs1, offset)
}
pub fn encode_stb(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_STB, rd, rs1, offset)
}
pub fn encode_sth(rd: u32, rs1: u32, offset: i32) -> u32 {
    encode_mem_imm(OP3_STH, rd, rs1, offset)
}

// ===========================================================================
// Byte-stream assembly
// ===========================================================================

/// Convert instruction words to **big-endian** bytes — SPARC V8's default
/// byte order (same as MIPS R2000; unlike RISC-V/ARM/x86, which are
/// little-endian).
pub fn assemble(instructions: &[u32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(instructions.len() * 4);
    for &inst in instructions {
        result.extend_from_slice(&inst.to_be_bytes());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ta_zero_matches_halt_word() {
        assert_eq!(encode_ta(0), HALT_WORD);
        assert_eq!(HALT_WORD, 0x91D0_2000);
    }

    #[test]
    fn nop_word_matches_sethi_g0_zero() {
        assert_eq!(encode_sethi(0, 0), NOP_WORD);
    }

    #[test]
    fn add_imm_encoding() {
        // ADD %g0, 42, %o0: op=2, rd=8, op3=0, rs1=0, i=1, simm13=42
        let word = encode_add_imm(8, 0, 42);
        assert_eq!(word, (0b10 << 30) | (8 << 25) | (1 << 13) | 42);
    }

    #[test]
    fn assemble_is_big_endian() {
        assert_eq!(assemble(&[0x1234_5678]), vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn call_disp30_roundtrip_bits() {
        let word = encode_call(3);
        assert_eq!(word >> 30, OP_CALL);
        assert_eq!(word & 0x3FFF_FFFF, 3);
    }

    #[test]
    fn sethi_imm22_shifted_at_execute_not_encode() {
        // encode_sethi stores the raw imm22, not imm22<<10 -- the shift
        // happens at execute time.
        let word = encode_sethi(1, 0x3FFFFF);
        assert_eq!(word & 0x3F_FFFF, 0x3FFFFF);
    }
}
