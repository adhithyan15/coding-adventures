//! Encoding helpers for constructing Intel 8080 machine code (used by tests
//! and by `intel8080-encoder`, which re-exports the `encode_*` functions
//! this backend actually needs).
//!
//! Every `encode_*` function returns an owned `Vec<u8>` since 8080
//! instructions are variable-length (1, 2, or 3 bytes) — unlike MIPS
//! R2000's fixed 32-bit words, there's no single word type to return.
//! 16-bit address/immediate operands (`LXI`, `JMP`, `CALL`, `LDA`/`STA`,
//! `LHLD`/`SHLD`, conditional jump/call) are written **little-endian**
//! (low byte first) — `_fetch_word` in the Python original reads low
//! byte then high byte, matching the 8086/x86 convention this chip's
//! lineage originated.

use crate::opcodes::*;

// ===========================================================================
// Group-00 encoders (data movement / 16-bit ops / immediate loads)
// ===========================================================================

/// `MVI r, d8` — move immediate byte into register `r` (or memory via M).
/// Bit pattern: `00_rrr_110`.
pub fn encode_mvi(reg: u8, imm: u8) -> Vec<u8> {
    vec![0b00_000_110 | ((reg & 0x07) << 3), imm]
}

/// `MVI A, n` — convenience wrapper; the only `encode_*` the minimal-viable
/// `intel8080-backend` actually calls.
pub fn encode_mvi_a(imm: u8) -> Vec<u8> {
    encode_mvi(REG_A, imm)
}

/// `LXI rp, d16` — load a 16-bit immediate into register pair `rp`.
/// Bit pattern: `00_pp0_001`; operand is little-endian.
pub fn encode_lxi(pair: u8, word: u16) -> Vec<u8> {
    vec![
        0b00_000_001 | ((pair & 0x03) << 4),
        (word & 0xFF) as u8,
        (word >> 8) as u8,
    ]
}

/// `INX rp` — increment register pair `rp` (16-bit, no flags).
/// Bit pattern: `00_pp0_011`.
pub fn encode_inx(pair: u8) -> u8 {
    0b00_000_011 | ((pair & 0x03) << 4)
}

/// `DCX rp` — decrement register pair `rp` (16-bit, no flags).
/// Bit pattern: `00_pp1_011`.
pub fn encode_dcx(pair: u8) -> u8 {
    0b00_001_011 | ((pair & 0x03) << 4)
}

/// `DAD rp` — HL ← HL + rp (16-bit add; only CY is affected).
/// Bit pattern: `00_pp1_001`.
pub fn encode_dad(pair: u8) -> u8 {
    0b00_001_001 | ((pair & 0x03) << 4)
}

/// `INR r` — increment register `r` by 1 (S,Z,P,AC; CY untouched).
/// Bit pattern: `00_rrr_100`.
pub fn encode_inr(reg: u8) -> u8 {
    0b00_000_100 | ((reg & 0x07) << 3)
}

/// `DCR r` — decrement register `r` by 1 (S,Z,P,AC; CY untouched).
/// Bit pattern: `00_rrr_101`.
pub fn encode_dcr(reg: u8) -> u8 {
    0b00_000_101 | ((reg & 0x07) << 3)
}

/// `STAX B`/`STAX D` — memory[rp] ← A.  Only `PAIR_B`/`PAIR_D` are valid.
pub fn encode_stax(pair: u8) -> u8 {
    if pair == PAIR_D { STAX_D } else { STAX_B }
}

/// `LDAX B`/`LDAX D` — A ← memory[rp].  Only `PAIR_B`/`PAIR_D` are valid.
pub fn encode_ldax(pair: u8) -> u8 {
    if pair == PAIR_D { LDAX_D } else { LDAX_B }
}

/// `SHLD addr` — memory[addr] ← L; memory[addr+1] ← H.
pub fn encode_shld(addr: u16) -> Vec<u8> {
    vec![SHLD, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LHLD addr` — L ← memory[addr]; H ← memory[addr+1].
pub fn encode_lhld(addr: u16) -> Vec<u8> {
    vec![LHLD, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `STA addr` — memory[addr] ← A.
pub fn encode_sta(addr: u16) -> Vec<u8> {
    vec![STA, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `LDA addr` — A ← memory[addr].
pub fn encode_lda(addr: u16) -> Vec<u8> {
    vec![LDA, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

// ===========================================================================
// Group-01 encoder (MOV) + HLT
// ===========================================================================

/// `MOV dst, src` — register-to-register copy (either side may be M).
/// Bit pattern: `01_ddd_sss`.  `MOV M, M` (`0x76`) is reserved for `HLT`;
/// this function does not special-case it — callers should use
/// [`HLT`](crate::opcodes::HLT) directly rather than `encode_mov(REG_M, REG_M)`.
pub fn encode_mov(dst: u8, src: u8) -> u8 {
    0b01_000_000 | ((dst & 0x07) << 3) | (src & 0x07)
}

// ===========================================================================
// Group-10 / Group-11 ALU encoders
// ===========================================================================

/// `{ADD,ADC,SUB,SBB,ANA,XRA,ORA,CMP} r` — ALU op against a register.
/// Bit pattern: `10_ooo_sss`.
pub fn encode_alu_reg(op: u8, src: u8) -> u8 {
    0b10_000_000 | ((op & 0x07) << 3) | (src & 0x07)
}

/// `{ADI,ACI,SUI,SBI,ANI,XRI,ORI,CPI} d8` — ALU op against an 8-bit
/// immediate.  Bit pattern: `11_ooo_110`.
pub fn encode_alu_imm(op: u8, imm: u8) -> Vec<u8> {
    vec![0b11_000_110 | ((op & 0x07) << 3), imm]
}

// ===========================================================================
// Group-11 control flow: jumps, calls, returns, RST
// ===========================================================================

/// `JMP addr` — unconditional jump.
pub fn encode_jmp(addr: u16) -> Vec<u8> {
    vec![JMP, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `J<cond> addr` — conditional jump.  Bit pattern: `11_ccc_010`.
pub fn encode_jcond(cond: u8, addr: u16) -> Vec<u8> {
    vec![
        0b11_000_010 | ((cond & 0x07) << 3),
        (addr & 0xFF) as u8,
        (addr >> 8) as u8,
    ]
}

/// `CALL addr` — unconditional call.
pub fn encode_call(addr: u16) -> Vec<u8> {
    vec![CALL, (addr & 0xFF) as u8, (addr >> 8) as u8]
}

/// `C<cond> addr` — conditional call.  Bit pattern: `11_ccc_100`.
pub fn encode_ccond(cond: u8, addr: u16) -> Vec<u8> {
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

/// `R<cond>` — conditional return.  Bit pattern: `11_ccc_000`.
pub fn encode_rcond(cond: u8) -> u8 {
    0b11_000_000 | ((cond & 0x07) << 3)
}

/// `RST n` (n = 0..=7) — push PC, jump to `8*n`.  Bit pattern: `11_nnn_111`.
pub fn encode_rst(n: u8) -> u8 {
    0b11_000_111 | ((n & 0x07) << 3)
}

// ===========================================================================
// Group-11 stack / I/O
// ===========================================================================

/// `PUSH rp` (`rp == PAIR_SP` means `PUSH PSW`).  Bit pattern: `11_pp0_101`.
pub fn encode_push(pair: u8) -> u8 {
    0b11_000_101 | ((pair & 0x03) << 4)
}

/// `POP rp` (`rp == PAIR_SP` means `POP PSW`).  Bit pattern: `11_pp0_001`.
pub fn encode_pop(pair: u8) -> u8 {
    0b11_000_001 | ((pair & 0x03) << 4)
}

/// `IN port` — A ← input_port\[port\].
pub fn encode_in(port: u8) -> Vec<u8> {
    vec![IN, port]
}

/// `OUT port` — output_port\[port\] ← A.
pub fn encode_out(port: u8) -> Vec<u8> {
    vec![OUT, port]
}

// ===========================================================================
// Byte-stream assembly
// ===========================================================================

/// Concatenate a sequence of already-encoded instructions into one flat
/// byte stream.  Unlike `mips_r2000_simulator::encoding::assemble` (which
/// converts fixed 32-bit words to big-endian bytes), Intel 8080
/// instructions are already byte sequences of varying length, so this is
/// a plain flatten — no endianness conversion at this layer (individual
/// `encode_*` calls already place 16-bit operands little-endian).
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvi_a_encodes_opcode_and_immediate() {
        assert_eq!(encode_mvi_a(42), vec![0x3E, 0x2A]);
        assert_eq!(encode_mvi(REG_B, 1), vec![0x06, 0x01]);
    }

    #[test]
    fn hlt_is_the_documented_byte() {
        assert_eq!(HLT, 0x76);
    }

    #[test]
    fn lxi_is_little_endian() {
        assert_eq!(encode_lxi(PAIR_H, 0x0100), vec![0x21, 0x00, 0x01]);
    }

    #[test]
    fn jmp_call_are_little_endian() {
        assert_eq!(encode_jmp(0x1234), vec![0xC3, 0x34, 0x12]);
        assert_eq!(encode_call(0x1234), vec![0xCD, 0x34, 0x12]);
    }

    #[test]
    fn mov_a_b_matches_known_byte() {
        // MOV A,B = 01 111 000 = 0x78
        assert_eq!(encode_mov(REG_A, REG_B), 0x78);
    }

    #[test]
    fn alu_reg_add_b_matches_known_byte() {
        // ADD B = 10 000 000 = 0x80
        assert_eq!(encode_alu_reg(ALU_ADD, REG_B), 0x80);
    }

    #[test]
    fn alu_imm_adi_matches_known_bytes() {
        assert_eq!(encode_alu_imm(ALU_ADD, 0x2A), vec![0xC6, 0x2A]);
    }

    #[test]
    fn assemble_flattens_variable_length_instructions() {
        let bytes = assemble(&[encode_mvi_a(42), vec![HLT]]);
        assert_eq!(bytes, vec![0x3E, 0x2A, 0x76]);
    }

    #[test]
    fn rst_encodes_restart_vector() {
        // RST 7 = 11 111 111 = 0xFF
        assert_eq!(encode_rst(7), 0xFF);
    }

    #[test]
    fn push_pop_psw_use_pair_sp() {
        // PUSH PSW = 11 110 101 = 0xF5 ; POP PSW = 11 110 001 = 0xF1
        assert_eq!(encode_push(PAIR_SP), 0xF5);
        assert_eq!(encode_pop(PAIR_SP), 0xF1);
    }
}
