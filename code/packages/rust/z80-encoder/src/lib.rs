//! # `z80-encoder` — pure Zilog Z80 instruction encoder.
//!
//! Mirror of [`intel8080_encoder`] / [`mips_r2000_encoder`] for the Zilog
//! Z80 (1976) — an Intel 8080-superset that powered the TRS-80, ZX
//! Spectrum, MSX, the original Game Boy (via a variant core), and
//! countless CP/M machines.  Seventh lane of the 9-architecture
//! expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — the canonical `encode_*` helpers (e.g.
//!    `encode_ld_a_n`, `encode_ld_rp_nn`, `HALT`) come from
//!    [`z80_simulator::encoding`] /
//!    [`z80_simulator::opcodes`].  We re-export them here so that
//!    `z80-backend` (Backend trait over CIR) can depend on a small,
//!    IR-agnostic surface without pulling the full simulator's
//!    decode/execute internals into every consumer.  Future ISA-spec
//!    updates land in `z80-simulator::encoding` and propagate
//!    automatically.
//! 2. **Register constants** — `REG_A` and `PAIR_HL`, the byte and word
//!    result locations used by `z80-backend`.
//! 3. **Capacity constant** — `LD_A_N_MAX` for the 8-bit immediate range.
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder calls
//! + the register constant.
//!
//! ## ISA quick reference (subset used here)
//!
//! | Mnemonic | Opcode | Bytes | Effect |
//! |----------|--------|-------|--------|
//! | `HALT` | `0x76` | 1 | halt — `01_110_110` |
//! | `LD A,n` | `0x3E nn` | 2 | A ← 8-bit immediate `n` |
//! | `LD HL,nn` | `0x21 lo hi` | 3 | HL ← 16-bit immediate `nn` |
//! | `RET` | `0xC9` | 1 | return from subroutine |
//!
//! ## Byte-identity with `intel8080-encoder`
//!
//! `encode_ld_a_n` and `HALT` are **byte-identical** to
//! `intel8080_encoder::encode_mvi_a` / `intel8080_encoder::HLT` — the Z80
//! reuses the 8080's `LD A,n` (`0x3E imm`) and `HALT` (`0x76`) encodings
//! verbatim, since both are part of the base 8080-legacy opcode set the
//! Z80 is fully source- and binary-compatible with.  See
//! `code/specs/z80-encoder.md` for the full writeup and
//! `z80-backend`'s test suite for a direct cross-architecture assertion.
//!
//! ## Quick start
//!
//! ```
//! use z80_encoder::{encode_ld_a_n, HALT, RET};
//!
//! // LD A, 42 → [0x3E, 0x2A] -- byte-identical to intel8080_encoder::encode_mvi_a(42)
//! assert_eq!(encode_ld_a_n(42), vec![0x3E, 0x2A]);
//! assert_eq!(HALT, 0x76);
//! assert_eq!(RET, 0xC9);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `z80-simulator::encoding` / `::opcodes` are the in-tree source of truth
// for the Z80 bit layout — it's the only place the opcode constants and
// the instruction-byte-sequence packing logic lives.  We re-export the
// subset `z80-backend` actually uses.

pub use z80_simulator::encoding::{
    assemble, encode_add_hl_rp, encode_alu_reg, encode_ld_a_n, encode_ld_r_n, encode_ld_r_r,
    encode_ld_rp_nn,
};
pub use z80_simulator::opcodes::{
    ALU_ADC, ALU_ADD, ALU_AND, ALU_OR, ALU_SBC, ALU_SUB, ALU_XOR, HALT, PAIR_DE, PAIR_HL, REG_B,
    REG_C, REG_D, REG_E, REG_H, REG_L, RET,
};

// ===========================================================================
// Register-role constant
// ===========================================================================
//
// Like the 8080, the Z80 names its working registers (A, B, C, D, E, H,
// L) rather than numbering them, so `z80-backend` addresses the
// accumulator directly rather than through an indexed register file.

pub use z80_simulator::opcodes::REG_A;

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Maximum unsigned 8-bit `LD A,n` immediate (= 255).
pub const LD_A_N_MAX: u8 = 255;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ret_byte_value() {
        assert_eq!(RET, 0xC9);
    }

    #[test]
    fn halt_byte_value() {
        assert_eq!(HALT, 0x76);
    }

    #[test]
    fn register_constant_matches_convention() {
        assert_eq!(REG_A, 7);
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First (and only) instruction of the Twig `42` lowering:
        // LD A, 42
        assert_eq!(encode_ld_a_n(42), vec![0x3E, 0x2A]);
    }

    #[test]
    fn canonical_word_result_bytes() {
        assert_eq!(encode_ld_rp_nn(PAIR_HL, 0x1234), vec![0x21, 0x34, 0x12]);
    }

    #[test]
    fn word02_pair_and_bytewise_alu_encodings() {
        assert_eq!(encode_add_hl_rp(PAIR_DE), 0x19);
        assert_eq!(encode_alu_reg(ALU_SBC, REG_D), 0x9A);
        assert_eq!(encode_ld_r_r(REG_H, REG_B), 0x60);
    }

    #[test]
    fn canonical_const_42_matches_intel8080_encoder_bytes() {
        // z80-encoder's LD A,n / HALT reuse the exact 8080-legacy
        // encoding -- this is the cross-architecture consistency check
        // called out in code/specs/z80-encoder.md.  We don't depend on
        // intel8080-encoder here (keeping this crate's dependency
        // surface minimal), so this test pins the literal byte values
        // intel8080_encoder::encode_mvi_a(42) / HLT are also pinned to.
        assert_eq!(encode_ld_a_n(42), vec![0x3E, 0x2A]);
        assert_eq!(HALT, 0x76);
    }

    #[test]
    fn assemble_then_halt() {
        assert_eq!(
            assemble(&[encode_ld_a_n(42), vec![HALT]]),
            vec![0x3E, 0x2A, 0x76]
        );
    }

    #[test]
    fn ld_a_n_max_is_255() {
        assert_eq!(LD_A_N_MAX, 255);
    }
}
