//! # `mos6502-encoder` — pure MOS 6502 instruction encoder.
//!
//! Mirror of [`mips_r2000_encoder`] / [`arm1_encoder`] / [`armv7_encoder`]
//! / [`intel8008_encoder`] for the MOS 6502 (1975) — Chuck Peddle's
//! $25 chip that powered the Apple II, Commodore 64, Atari 8-bit line, BBC
//! Micro, and (via the Ricoh 2A03 variant) the NES/Famicom.  Fifth lane of
//! the 9-architecture expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — `encode_lda_imm`/`encode_brk` (plus
//!    [`assemble`]) come from [`mos6502_simulator::encoding`].  We
//!    re-export them here so `mos6502-backend` (Backend trait over CIR)
//!    can depend on a small, IR-agnostic surface without pulling the full
//!    simulator's decode/execute machinery into every consumer.  Future
//!    ISA-spec updates land in `mos6502_simulator::encoding` and propagate
//!    automatically.
//! 2. **Canonical byte constant** — `HALT_BYTE` for `BRK`, the single-byte
//!    halt sentinel every 6502 program in this backend's scope ends with.
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder calls.
//!
//! ## Why `BRK`, not a KIL/JAM illegal opcode or a self-jump spin loop?
//!
//! Two real-world 6502 halt conventions exist in the wild: (a) an
//! illegal/undocumented opcode that locks the CPU (`KIL`/`JAM`, e.g.
//! `0x02`) — some test-suite emulators treat this as "halted" since the
//! chip stops fetching; (b) a self-targeting `JMP $addr` spin loop, which
//! some halt-less architectures in this repo's 9-architecture expansion
//! use instead.
//!
//! Neither is used here.  `mos6502-simulator` (this crate's dependency)
//! already has a **pre-existing, documented** halt convention it ported
//! directly from `code/packages/python/mos6502-simulator`: `BRK` (opcode
//! `0x00`) sets the simulator's `halted` flag.  That convention predates
//! this encoder/backend lane entirely — the Python original's module doc
//! states it outright: *"BRK (opcode 0x00) is treated as HALT — the
//! simulator stops and sets `halted=True`... This matches the convention
//! used throughout the simulator stack (HLT for 8080, TRAP for IBM 704,
//! etc.)"*.  Mirroring the **existing, established** semantics for this
//! specific ISA in this repo is the correct choice — inventing a KIL/JAM
//! or self-jump convention instead would silently diverge from every
//! other MOS 6502 program already exercised against this simulator stack
//! (see `code/specs/07j-mos6502-simulator.md`'s "Test Programs" section,
//! all of which end in `BRK`).
//!
//! Unlike ARM1 (which has no real halt instruction at all — ARMv1
//! silicon can only spin or await an interrupt, hence `arm1-backend`'s
//! pseudo-halt `SWI`), the 6502 already has a real, well-known,
//! previously-documented HALT convention in this repo.  No pseudo-
//! instruction invention was needed for this lane.
//!
//! ## Quick start
//!
//! ```
//! use mos6502_encoder::{encode_lda_imm, encode_brk, assemble, HALT_BYTE};
//!
//! // const_i64 v=42 lowered to `LDA #42` -- the first instruction
//! // `mos6502-backend` emits for the canonical IIR `const 42; ret` program.
//! let bytes = assemble(&[encode_lda_imm(42), encode_brk()]);
//! assert_eq!(bytes, vec![0xA9, 42, 0x00]);
//! assert_eq!(HALT_BYTE, 0x00);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `mos6502_simulator` is the in-tree source of truth for the MOS 6502
// opcode table -- it's the only place the mnemonic->opcode-byte mapping
// lives.  We re-export the subset of `encode_*` helpers that
// `mos6502-backend` actually uses (plus a few more exercised by this
// crate's own tests).

pub use mos6502_simulator::encoding::{
    assemble, encode_adc_imm, encode_brk, encode_clc, encode_lda_imm, encode_ldx_imm,
    encode_ldy_imm, encode_nop, encode_sbc_imm, encode_sec, encode_sta_zp,
};

// ===========================================================================
// Canonical byte constant
// ===========================================================================

/// `BRK` — the MOS 6502 HALT sentinel (see
/// [`mos6502_simulator::opcodes::BRK_OPCODE`]).  Encoded value: `0x00`.
/// A single byte, so unlike `mips_r2000_encoder::RET_WORD` /
/// `arm1_encoder::HALT_WORD` there is no endianness to speak of.
pub const HALT_BYTE: u8 = mos6502_simulator::opcodes::BRK_OPCODE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_byte_matches_brk_opcode() {
        assert_eq!(HALT_BYTE, mos6502_simulator::opcodes::BRK_OPCODE);
    }

    #[test]
    fn halt_byte_value() {
        assert_eq!(HALT_BYTE, 0x00);
    }

    #[test]
    fn halt_byte_matches_encode_brk() {
        assert_eq!(vec![HALT_BYTE], encode_brk());
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First instruction of the IIR `42` lowering: LDA #42
        assert_eq!(encode_lda_imm(42), vec![0xA9, 42]);
    }

    #[test]
    fn assemble_flattens_lda_then_brk() {
        assert_eq!(
            assemble(&[encode_lda_imm(42), encode_brk()]),
            vec![0xA9, 42, 0x00]
        );
    }
}
