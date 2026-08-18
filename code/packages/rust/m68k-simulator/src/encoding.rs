//! Encoding helpers for constructing Motorola 68000 machine code (used by
//! tests and by `m68k-encoder`, which re-exports the `encode_*`
//! functions `m68k-backend` needs).
//!
//! Like `mips-r2000-simulator::encoding` (and unlike
//! `mos6502-simulator::encoding`, which has no word endianness to speak
//! of), every `encode_*` helper here returns big-endian bytes directly —
//! the 68000's own native byte order — so [`assemble`] is a trivial
//! concatenation with no additional byte-order conversion needed by
//! callers.
//!
//! Only the handful of mnemonics `m68k-backend`'s minimal-viable scope
//! and this crate's own tests need get an `encode_*` helper here — the
//! full instruction *semantics* live in `execute.rs`, decoded directly
//! from raw opwords (there is no flat opcode table to encode against,
//! unlike the 6502's `LDA_IMM_OPCODE`-style constants).

/// `MOVE.L #imm, Dn` — move a 32-bit immediate into data register `dn`.
///
/// Encoding: `00 10 ddd 000 111 100` (`ddd` = destination register,
/// `000` = Dn-direct destination mode, `111 100` = immediate source
/// mode) followed by the 32-bit immediate, big-endian.  `dn=0, imm=42`
/// produces the canonical `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]` —
/// verified against the pre-existing Python simulator's own test suite,
/// which uses this exact `0x203C`/`0x223C`/... opword family throughout
/// (`code/packages/python/motorola-68000-simulator/tests/
/// test_instructions.py`).
///
/// # Panics
///
/// Panics if `dn > 7`.
pub fn encode_move_l_imm_to_dn(dn: u8, imm: u32) -> Vec<u8> {
    assert!(dn <= 7, "data register index must be 0-7, got {dn}");
    let opword: u16 = 0x2000 | (u16::from(dn) << 9) | 0x03C;
    let mut bytes = opword.to_be_bytes().to_vec();
    bytes.extend_from_slice(&imm.to_be_bytes());
    bytes
}

/// `MOVEQ #imm, Dn` — move an 8-bit sign-extended immediate into `dn`.
/// Encoding: `0111 ddd0 iiiiiiii`.
///
/// # Panics
///
/// Panics if `dn > 7`.
pub fn encode_moveq(dn: u8, imm: i8) -> Vec<u8> {
    assert!(dn <= 7, "data register index must be 0-7, got {dn}");
    let opword: u16 = 0x7000 | (u16::from(dn) << 9) | u16::from(imm as u8);
    opword.to_be_bytes().to_vec()
}

/// `TRAP #15` — this simulator's HALT sentinel (see the crate-level doc
/// for the derivation).  Encoded value: `0x4E4F`.
pub fn encode_trap15() -> Vec<u8> {
    crate::opcodes::TRAP_15_WORD.to_be_bytes().to_vec()
}

/// `NOP` — no operation.  `0x4E71`.
pub fn encode_nop() -> Vec<u8> {
    0x4E71u16.to_be_bytes().to_vec()
}

/// `RTS` — return from subroutine.  `0x4E75`.
pub fn encode_rts() -> Vec<u8> {
    0x4E75u16.to_be_bytes().to_vec()
}

/// Concatenate a sequence of per-instruction byte vectors into one flat
/// byte stream.  No endianness conversion needed (every `encode_*`
/// helper here already returns big-endian bytes) — provided for parity
/// with `mips_r2000_simulator::encoding::assemble` /
/// `mos6502_simulator::encoding::assemble`.
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flat_map(|i| i.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_l_imm_to_d0_matches_python_reference() {
        assert_eq!(
            encode_move_l_imm_to_dn(0, 42),
            vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]
        );
    }

    #[test]
    fn move_l_imm_to_d1_uses_0x223c_opword() {
        // Matches the Python test suite's `_w(0x223C)` for MOVE.L #imm,D1.
        assert_eq!(encode_move_l_imm_to_dn(1, 0x1234), vec![
            0x22, 0x3C, 0x00, 0x00, 0x12, 0x34
        ]);
    }

    #[test]
    fn move_l_imm_negative_value_stores_twos_complement() {
        assert_eq!(
            encode_move_l_imm_to_dn(0, 0xFFFF_FFFF),
            vec![0x20, 0x3C, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn moveq_encoding() {
        assert_eq!(encode_moveq(0, 42), vec![0x70, 0x2A]);
        assert_eq!(encode_moveq(0, -1), vec![0x70, 0xFF]);
    }

    #[test]
    fn trap15_encoding() {
        assert_eq!(encode_trap15(), vec![0x4E, 0x4F]);
    }

    #[test]
    fn nop_and_rts_encodings() {
        assert_eq!(encode_nop(), vec![0x4E, 0x71]);
        assert_eq!(encode_rts(), vec![0x4E, 0x75]);
    }

    #[test]
    fn assemble_concatenates_in_order() {
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        assert_eq!(bytes, vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]);
    }

    #[test]
    fn round_trip_through_simulator() {
        use crate::simulator::M68kSimulator;
        let bytes = assemble(&[encode_move_l_imm_to_dn(0, 42), encode_trap15()]);
        let mut sim = M68kSimulator::new(65536);
        sim.run(&bytes);
        assert_eq!(sim.d[0], 42);
        assert!(sim.halted);
    }
}
