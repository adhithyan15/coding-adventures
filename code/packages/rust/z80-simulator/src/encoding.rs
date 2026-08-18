//! Encoding helpers for constructing Zilog Z80 machine code (used by tests
//! and by `z80-encoder`, which re-exports the `encode_*` functions this
//! backend actually needs).
//!
//! Every `encode_*` function returns an owned `Vec<u8>` since Z80
//! instructions are variable-length (1 to 4 bytes) — same convention as
//! `intel8080_simulator::encoding`.  16-bit address/immediate operands are
//! written **little-endian** (low byte first), matching the 8080 lineage.
//!
//! # Byte-identity with `intel8080-encoder`
//!
//! [`encode_ld_a_n`] and [`HALT`] are **byte-identical** to
//! `intel8080_encoder::encode_mvi_a` / `intel8080_encoder::HLT` — see the
//! `canonical_const_42_matches_intel8080` test at the bottom of this file,
//! and `code/specs/z80-encoder.md` for the full cross-architecture
//! consistency writeup.

use crate::opcodes::*;

// ===========================================================================
// 8080-legacy encoders (byte-identical to `intel8080_simulator::encoding`)
// ===========================================================================

/// `LD r,n` — load immediate byte into register `r` (or memory via
/// `(HL)`).  Bit pattern: `00_rrr_110`.  Zilog's name for 8080's `MVI`.
pub fn encode_ld_r_n(reg: u8, imm: u8) -> Vec<u8> {
    vec![0b00_000_110 | ((reg & 0x07) << 3), imm]
}

/// `LD A,n` — convenience wrapper; the only `encode_*` the minimal-viable
/// `z80-backend` actually calls.  **Byte-identical** to
/// `intel8080_encoder::encode_mvi_a` (`0x3E imm`) — both chips load an
/// 8-bit immediate into the accumulator with the same opcode.
pub fn encode_ld_a_n(imm: u8) -> Vec<u8> {
    encode_ld_r_n(REG_A, imm)
}

/// `LD rp,nn` — load a 16-bit immediate into register pair `rp`.
/// Bit pattern: `00_pp0_001`; operand is little-endian.  Zilog's name for
/// 8080's `LXI`.
pub fn encode_ld_rp_nn(pair: u8, word: u16) -> Vec<u8> {
    vec![
        0b00_000_001 | ((pair & 0x03) << 4),
        (word & 0xFF) as u8,
        (word >> 8) as u8,
    ]
}

/// `INC rp` — increment register pair `rp` (16-bit, no flags).
/// Bit pattern: `00_pp0_011`.
pub fn encode_inc_rp(pair: u8) -> u8 {
    0b00_000_011 | ((pair & 0x03) << 4)
}

/// `DEC rp` — decrement register pair `rp` (16-bit, no flags).
/// Bit pattern: `00_pp1_011`.
pub fn encode_dec_rp(pair: u8) -> u8 {
    0b00_001_011 | ((pair & 0x03) << 4)
}

/// `ADD HL,rp` — HL ← HL + rp (16-bit add; only CY is affected).
/// Bit pattern: `00_pp1_001`.  Zilog's name for 8080's `DAD`.
pub fn encode_add_hl_rp(pair: u8) -> u8 {
    0b00_001_001 | ((pair & 0x03) << 4)
}

/// `INC r` — increment register `r` by 1 (S,Z,P/V,H; CY untouched).
/// Bit pattern: `00_rrr_100`.
pub fn encode_inc_r(reg: u8) -> u8 {
    0b00_000_100 | ((reg & 0x07) << 3)
}

/// `DEC r` — decrement register `r` by 1 (S,Z,P/V,H; CY untouched).
/// Bit pattern: `00_rrr_101`.
pub fn encode_dec_r(reg: u8) -> u8 {
    0b00_000_101 | ((reg & 0x07) << 3)
}

/// `LD (BC),A` / `LD (DE),A` — memory[rp] ← A.  Only `PAIR_BC`/`PAIR_DE`
/// are valid.  Zilog's name for 8080's `STAX`.
pub fn encode_ld_rp_a(pair: u8) -> u8 {
    if pair == PAIR_DE { LD_DE_A } else { LD_BC_A }
}

/// `LD A,(BC)` / `LD A,(DE)` — A ← memory[rp].  Only `PAIR_BC`/`PAIR_DE`
/// are valid.  Zilog's name for 8080's `LDAX`.
pub fn encode_ld_a_rp(pair: u8) -> u8 {
    if pair == PAIR_DE { LD_A_DE } else { LD_A_BC }
}

/// `LD (nn),HL` — memory[nn] ← L; memory[nn+1] ← H.  Zilog's `SHLD`.
pub fn encode_ld_nn_hl(addr: u16) -> Vec<u8> {
    vec![LD_NN_HL, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LD HL,(nn)` — L ← memory[nn]; H ← memory[nn+1].  Zilog's `LHLD`.
pub fn encode_ld_hl_nn(addr: u16) -> Vec<u8> {
    vec![LD_HL_NN, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LD (nn),A` — memory[nn] ← A.  Zilog's `STA`.
pub fn encode_ld_nn_a(addr: u16) -> Vec<u8> {
    vec![LD_NN_A, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LD A,(nn)` — A ← memory[nn].  Zilog's `LDA`.
pub fn encode_ld_a_nn(addr: u16) -> Vec<u8> {
    vec![LD_A_NN, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LD dst,src` — register-to-register copy (either side may be `(HL)`).
/// Bit pattern: `01_ddd_sss`.  `LD (HL),(HL)` (`0x76`) is reserved for
/// `HALT`; this function does not special-case it — callers should use
/// [`HALT`](crate::opcodes::HALT) directly.  Zilog's name for 8080's
/// `MOV`.
pub fn encode_ld_r_r(dst: u8, src: u8) -> u8 {
    0b01_000_000 | ((dst & 0x07) << 3) | (src & 0x07)
}

/// `{ADD,ADC,SUB,SBC,AND,XOR,OR,CP} A,r` — ALU op against a register.
/// Bit pattern: `10_ooo_sss`.
pub fn encode_alu_reg(op: u8, src: u8) -> u8 {
    0b10_000_000 | ((op & 0x07) << 3) | (src & 0x07)
}

/// `{ADD,ADC,SUB,SBC,AND,XOR,OR,CP} A,n` — ALU op against an 8-bit
/// immediate.  Bit pattern: `11_ooo_110`.
pub fn encode_alu_imm(op: u8, imm: u8) -> Vec<u8> {
    vec![0b11_000_110 | ((op & 0x07) << 3), imm]
}

/// `JP nn` — unconditional absolute jump.  Zilog's name for 8080's `JMP`.
pub fn encode_jp(addr: u16) -> Vec<u8> {
    vec![JP, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `JP cc,nn` — conditional absolute jump.  Bit pattern: `11_ccc_010`.
pub fn encode_jp_cond(cond: u8, addr: u16) -> Vec<u8> {
    vec![
        0b11_000_010 | ((cond & 0x07) << 3),
        (addr & 0xFF) as u8,
        (addr >> 8) as u8,
    ]
}

/// `CALL nn` — unconditional call.
pub fn encode_call(addr: u16) -> Vec<u8> {
    vec![CALL, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `CALL cc,nn` — conditional call.  Bit pattern: `11_ccc_100`.
pub fn encode_call_cond(cond: u8, addr: u16) -> Vec<u8> {
    vec![
        0b11_000_100 | ((cond & 0x07) << 3),
        (addr & 0xFF) as u8,
        (addr >> 8) as u8,
    ]
}

/// `RET` — unconditional return.
pub fn encode_ret() -> u8 {
    RET
}

/// `RET cc` — conditional return.  Bit pattern: `11_ccc_000`.
pub fn encode_ret_cond(cond: u8) -> u8 {
    0b11_000_000 | ((cond & 0x07) << 3)
}

/// `RST n` (n = 0..=7) — push PC, jump to `8*n`.  Bit pattern: `11_nnn_111`.
pub fn encode_rst(n: u8) -> u8 {
    0b11_000_111 | ((n & 0x07) << 3)
}

/// `PUSH rp` (`rp == PAIR_AF` means `PUSH AF`).  Bit pattern: `11_pp0_101`.
pub fn encode_push(pair: u8) -> u8 {
    0b11_000_101 | ((pair & 0x03) << 4)
}

/// `POP rp` (`rp == PAIR_AF` means `POP AF`).  Bit pattern: `11_pp0_001`.
pub fn encode_pop(pair: u8) -> u8 {
    0b11_000_001 | ((pair & 0x03) << 4)
}

/// `IN A,(n)` — A ← input_port[n].
pub fn encode_in(port: u8) -> Vec<u8> {
    vec![IN, port]
}

/// `OUT (n),A` — output_port[n] ← A.
pub fn encode_out(port: u8) -> Vec<u8> {
    vec![OUT, port]
}

// ===========================================================================
// Z80-only encoders — no 8080 equivalent
// ===========================================================================

/// `EX AF,AF'` — swap the main/alternate AF register pairs.
pub fn encode_ex_af_af() -> u8 {
    EX_AF_AF
}

/// `EXX` — swap BC/DE/HL with the alternate bank.
pub fn encode_exx() -> u8 {
    EXX
}

/// `DJNZ e` — decrement B; PC-relative jump by signed `e` if nonzero.
/// `e` is the raw signed displacement byte (relative to the instruction
/// *after* this one — the caller is responsible for the `PC+2` bias, same
/// convention `execute::execute` uses).
pub fn encode_djnz(e: i8) -> Vec<u8> {
    vec![DJNZ, e as u8]
}

/// `JR e` — unconditional PC-relative jump.
pub fn encode_jr(e: i8) -> Vec<u8> {
    vec![JR, e as u8]
}

/// `JR NZ,e`.
pub fn encode_jr_nz(e: i8) -> Vec<u8> {
    vec![JR_NZ, e as u8]
}

/// `JR Z,e`.
pub fn encode_jr_z(e: i8) -> Vec<u8> {
    vec![JR_Z, e as u8]
}

/// `JR NC,e`.
pub fn encode_jr_nc(e: i8) -> Vec<u8> {
    vec![JR_NC, e as u8]
}

/// `JR C,e`.
pub fn encode_jr_c(e: i8) -> Vec<u8> {
    vec![JR_C, e as u8]
}

// ===========================================================================
// CB-prefixed encoders — bit manipulation + extended rotate/shift
// ===========================================================================

/// Rotate/shift sub-operation codes for the `CB`-prefixed rotate/shift
/// group (bits 5-3 of the second byte, `op < 0x40`).
pub const ROT_RLC: u8 = 0;
pub const ROT_RRC: u8 = 1;
pub const ROT_RL: u8 = 2;
pub const ROT_RR: u8 = 3;
pub const ROT_SLA: u8 = 4;
pub const ROT_SRA: u8 = 5;
/// Undocumented `SLL` (shifts in a 1, not a 0).
pub const ROT_SLL: u8 = 6;
pub const ROT_SRL: u8 = 7;

/// `CB`-prefixed rotate/shift: `{RLC,RRC,RL,RR,SLA,SRA,SLL,SRL} r`.
/// Bit pattern of the second byte: `00_ooo_rrr`.
pub fn encode_cb_rot(rot_op: u8, reg: u8) -> Vec<u8> {
    vec![CB_PREFIX, ((rot_op & 0x07) << 3) | (reg & 0x07)]
}

/// `RLC r` convenience wrapper.
pub fn encode_rlc_r(reg: u8) -> Vec<u8> {
    encode_cb_rot(ROT_RLC, reg)
}

/// `BIT b,r` — test bit `b` (0-7) of register `r`; sets Z accordingly.
/// Bit pattern of the second byte: `01_bbb_rrr`.
pub fn encode_bit(bit: u8, reg: u8) -> Vec<u8> {
    vec![CB_PREFIX, 0b01_000_000 | ((bit & 0x07) << 3) | (reg & 0x07)]
}

/// `RES b,r` — reset (clear) bit `b` of register `r`.
/// Bit pattern of the second byte: `10_bbb_rrr`.
pub fn encode_res(bit: u8, reg: u8) -> Vec<u8> {
    vec![CB_PREFIX, 0b10_000_000 | ((bit & 0x07) << 3) | (reg & 0x07)]
}

/// `SET b,r` — set bit `b` of register `r`.
/// Bit pattern of the second byte: `11_bbb_rrr`.
pub fn encode_set(bit: u8, reg: u8) -> Vec<u8> {
    vec![CB_PREFIX, 0b11_000_000 | ((bit & 0x07) << 3) | (reg & 0x07)]
}

// ===========================================================================
// DD/FD-prefixed encoders — IX/IY basics (v0.1.0 scope: LD/INC only)
// ===========================================================================

/// `LD IX,nn` — load a 16-bit immediate into IX.
pub fn encode_ld_ix_nn(word: u16) -> Vec<u8> {
    vec![DD_PREFIX, 0x21, (word & 0xFF) as u8, (word >> 8) as u8]
}

/// `LD IY,nn` — load a 16-bit immediate into IY.
pub fn encode_ld_iy_nn(word: u16) -> Vec<u8> {
    vec![FD_PREFIX, 0x21, (word & 0xFF) as u8, (word >> 8) as u8]
}

/// `INC IX` — increment IX by 1 (16-bit, no flags).
pub fn encode_inc_ix() -> Vec<u8> {
    vec![DD_PREFIX, 0x23]
}

/// `INC IY` — increment IY by 1 (16-bit, no flags).
pub fn encode_inc_iy() -> Vec<u8> {
    vec![FD_PREFIX, 0x23]
}

// ===========================================================================
// Byte-stream assembly
// ===========================================================================

/// Concatenate a sequence of already-encoded instructions into one flat
/// byte stream.  Z80 instructions are already byte sequences of varying
/// length — no endianness conversion at this layer (individual `encode_*`
/// calls already place 16-bit operands little-endian).
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ld_a_n_encodes_opcode_and_immediate() {
        assert_eq!(encode_ld_a_n(42), vec![0x3E, 0x2A]);
        assert_eq!(encode_ld_r_n(REG_B, 1), vec![0x06, 0x01]);
    }

    #[test]
    fn halt_is_the_documented_byte() {
        assert_eq!(HALT, 0x76);
    }

    #[test]
    fn canonical_const_42_matches_intel8080() {
        // Byte-identical to intel8080_encoder::encode_mvi_a(42) /
        // intel8080_simulator::opcodes::HLT — the Z80 shares the base
        // 8080 opcode set for this minimal-viable subset (see
        // code/specs/z80-encoder.md for the cross-architecture writeup).
        assert_eq!(encode_ld_a_n(42), vec![0x3E, 0x2A]);
        assert_eq!(HALT, 0x76);
    }

    #[test]
    fn ld_rp_nn_is_little_endian() {
        assert_eq!(encode_ld_rp_nn(PAIR_HL, 0x0100), vec![0x21, 0x00, 0x01]);
    }

    #[test]
    fn jp_call_are_little_endian() {
        assert_eq!(encode_jp(0x1234), vec![0xC3, 0x34, 0x12]);
        assert_eq!(encode_call(0x1234), vec![0xCD, 0x34, 0x12]);
    }

    #[test]
    fn ld_a_b_matches_known_byte() {
        // LD A,B = 01 111 000 = 0x78
        assert_eq!(encode_ld_r_r(REG_A, REG_B), 0x78);
    }

    #[test]
    fn alu_reg_add_b_matches_known_byte() {
        // ADD A,B = 10 000 000 = 0x80
        assert_eq!(encode_alu_reg(ALU_ADD, REG_B), 0x80);
    }

    #[test]
    fn alu_imm_add_matches_known_bytes() {
        assert_eq!(encode_alu_imm(ALU_ADD, 0x2A), vec![0xC6, 0x2A]);
    }

    #[test]
    fn assemble_flattens_variable_length_instructions() {
        let bytes = assemble(&[encode_ld_a_n(42), vec![HALT]]);
        assert_eq!(bytes, vec![0x3E, 0x2A, 0x76]);
    }

    #[test]
    fn rst_encodes_restart_vector() {
        // RST 7 = 11 111 111 = 0xFF
        assert_eq!(encode_rst(7), 0xFF);
    }

    #[test]
    fn push_pop_af_use_pair_af() {
        // PUSH AF = 11 110 101 = 0xF5 ; POP AF = 11 110 001 = 0xF1
        assert_eq!(encode_push(PAIR_AF), 0xF5);
        assert_eq!(encode_pop(PAIR_AF), 0xF1);
    }

    #[test]
    fn ex_af_af_and_exx_bytes() {
        assert_eq!(encode_ex_af_af(), 0x08);
        assert_eq!(encode_exx(), 0xD9);
    }

    #[test]
    fn jr_family_bytes() {
        assert_eq!(encode_jr(5), vec![0x18, 0x05]);
        assert_eq!(encode_jr_nz(-2), vec![0x20, 0xFE]);
        assert_eq!(encode_djnz(-3), vec![0x10, 0xFD]);
    }

    #[test]
    fn cb_prefixed_bit_ops() {
        // RLC B = CB 00 = [0xCB, 0x00]
        assert_eq!(encode_rlc_r(REG_B), vec![0xCB, 0x00]);
        // BIT 7,A = CB 01_111_111 = [0xCB, 0x7F]
        assert_eq!(encode_bit(7, REG_A), vec![0xCB, 0x7F]);
    }

    #[test]
    fn ddfd_ix_iy_basics() {
        assert_eq!(encode_ld_ix_nn(0x1234), vec![0xDD, 0x21, 0x34, 0x12]);
        assert_eq!(encode_ld_iy_nn(0x1234), vec![0xFD, 0x21, 0x34, 0x12]);
        assert_eq!(encode_inc_ix(), vec![0xDD, 0x23]);
        assert_eq!(encode_inc_iy(), vec![0xFD, 0x23]);
    }
}
