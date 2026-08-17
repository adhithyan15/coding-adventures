//! Encoding helpers for constructing MOS 6502 machine code (used by tests
//! and by `mos6502-encoder`, which re-exports the `encode_*` functions).
//!
//! Unlike `mips-r2000-simulator::encoding` (which assembles fixed 32-bit
//! *words*) or `arm1-encoder` (same), the 6502 is a byte-oriented ISA with
//! no word endianness to speak of — every `encode_*` helper here returns a
//! `Vec<u8>` of the instruction's raw bytes directly, and [`assemble`] is a
//! trivial concatenation (no byte-order conversion needed).
//!
//! Only a modest subset of the 151-opcode table gets an `encode_*` helper
//! here — the ones exercised by this crate's own unit tests and by
//! `mos6502-backend` (whose v0.1.0 scope is `LDA #imm` + `BRK`).  The full
//! opcode table lives in `opcodes::lookup` for the decoder; encoding the
//! remaining mnemonics is a future increment (mirrors how
//! `mips-r2000-encoder`/`arm1-encoder` only re-export the subset their
//! backend needs today, not the encoder's own simulator's complete
//! internal table).

use crate::opcodes;

/// `LDA #imm` — immediate-mode load accumulator.  `[0xA9, imm]`.
pub fn encode_lda_imm(imm: u8) -> Vec<u8> {
    vec![opcodes::LDA_IMM_OPCODE, imm]
}

/// `LDX #imm` — immediate-mode load X.  `[0xA2, imm]`.
pub fn encode_ldx_imm(imm: u8) -> Vec<u8> {
    vec![0xA2, imm]
}

/// `LDY #imm` — immediate-mode load Y.  `[0xA0, imm]`.
pub fn encode_ldy_imm(imm: u8) -> Vec<u8> {
    vec![0xA0, imm]
}

/// `STA $zp` — zero-page store accumulator.  `[0x85, zp]`.
pub fn encode_sta_zp(zp: u8) -> Vec<u8> {
    vec![0x85, zp]
}

/// `ADC #imm` — immediate-mode add with carry.  `[0x69, imm]`.
pub fn encode_adc_imm(imm: u8) -> Vec<u8> {
    vec![0x69, imm]
}

/// `SBC #imm` — immediate-mode subtract with borrow.  `[0xE9, imm]`.
pub fn encode_sbc_imm(imm: u8) -> Vec<u8> {
    vec![0xE9, imm]
}

/// `CLC` — clear carry.  `[0x18]`.
pub fn encode_clc() -> Vec<u8> {
    vec![0x18]
}

/// `SEC` — set carry.  `[0x38]`.
pub fn encode_sec() -> Vec<u8> {
    vec![0x38]
}

/// `INX` — increment X.  `[0xE8]`.
pub fn encode_inx() -> Vec<u8> {
    vec![0xE8]
}

/// `DEX` — decrement X.  `[0xCA]`.
pub fn encode_dex() -> Vec<u8> {
    vec![0xCA]
}

/// `TAX` — transfer A to X.  `[0xAA]`.
pub fn encode_tax() -> Vec<u8> {
    vec![0xAA]
}

/// `NOP` — no operation.  `[0xEA]`.
pub fn encode_nop() -> Vec<u8> {
    vec![opcodes::NOP_OPCODE]
}

/// `BRK` — the HALT sentinel (see [`crate::opcodes::BRK_OPCODE`]).  `[0x00]`.
pub fn encode_brk() -> Vec<u8> {
    vec![opcodes::BRK_OPCODE]
}

/// Concatenate a sequence of per-instruction byte vectors into one flat
/// byte stream.  Trivial for the 6502 (no word-endianness conversion, no
/// fixed instruction width) — provided for API parity with
/// `mips_r2000_simulator::encoding::assemble` / consistency with how
/// `mos6502-backend` builds its output.
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flat_map(|i| i.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lda_imm_encoding() {
        assert_eq!(encode_lda_imm(42), vec![0xA9, 42]);
    }

    #[test]
    fn brk_encoding() {
        assert_eq!(encode_brk(), vec![0x00]);
    }

    #[test]
    fn assemble_concatenates() {
        let bytes = assemble(&[encode_lda_imm(42), encode_brk()]);
        assert_eq!(bytes, vec![0xA9, 42, 0x00]);
    }

    #[test]
    fn round_trip_through_simulator() {
        use crate::simulator::Mos6502Simulator;
        let bytes = assemble(&[encode_lda_imm(7), encode_adc_imm(5), encode_brk()]);
        let mut sim = Mos6502Simulator::new(65536);
        sim.run(&bytes);
        assert_eq!(sim.a, 12);
        assert!(sim.halted);
    }
}
