//! # `intel8086-encoder` — pure Intel 8086 instruction encoder.
//!
//! Mirror of [`mos6502_encoder`]/[`arm1_encoder`]/[`riscv_encoder`] for
//! the Intel 8086 (1978) — the 16-bit extension of the 8080 architecture
//! that introduced segmented memory and the ModRM addressing byte, and
//! whose cheaper sibling the 8088 shipped in the original IBM PC (1981),
//! founding the "PC-compatible" industry. Ninth and final lane of the
//! 9-architecture expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — `encode_mov_reg_imm16`/`encode_hlt`
//!    (plus [`assemble`] and a few more) come from
//!    [`intel8086_simulator::encoding`]. We re-export them here so
//!    `intel8086-backend` (the `Backend` trait implementation over CIR)
//!    can depend on a small, IR-agnostic surface without pulling the
//!    full simulator's decode/execute machinery into every consumer.
//! 2. **Canonical byte constant** — `HALT_BYTE` for `HLT`, the byte
//!    every program `intel8086-backend` compiles ends with.
//! 3. **`REG_AX`** — re-exported from `intel8086_simulator::opcodes` so
//!    `intel8086-backend` doesn't need a direct `intel8086-simulator`
//!    dependency just to name the accumulator register.
//!
//! No IR knowledge lives here. Consumers map their IR onto encoder calls.
//!
//! ## Quick start
//!
//! ```
//! use intel8086_encoder::{encode_mov_reg_imm16, encode_hlt, assemble, HALT_BYTE, REG_AX};
//!
//! // const_i64 v=42 lowered to `MOV AX,42` -- the first instruction
//! // `intel8086-backend` emits for the canonical IIR `const 42; ret` program.
//! let bytes = assemble(&[encode_mov_reg_imm16(REG_AX, 42), encode_hlt()]);
//! assert_eq!(bytes, vec![0xB8, 42, 0x00, 0xF4]);
//! assert_eq!(HALT_BYTE, 0xF4);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `intel8086_simulator` is the in-tree source of truth for this crate's
// curated Intel 8086 opcode subset -- we re-export the handful of
// `encode_*` helpers that `intel8086-backend` actually uses (plus a few
// more exercised by this crate's own tests).

pub use intel8086_simulator::encoding::{
    assemble, encode_add_ax_imm16, encode_alu_reg_reg, encode_dec_reg16, encode_hlt,
    encode_inc_reg16, encode_jcc_short, encode_jmp_near, encode_mov_reg_imm16, encode_mov_reg_imm8,
    encode_mov_reg_reg16, encode_mov_reg_reg8, encode_nop, encode_sub_ax_imm16,
};

/// The accumulator register index (`AX`) — re-exported so
/// `intel8086-backend` can name its always-target register without a
/// direct `intel8086-simulator` dependency.
pub use intel8086_simulator::opcodes::{
    REG_AH, REG_AL, REG_AX, REG_BH, REG_BL, REG_BX, REG_CL, REG_CX,
};

// ===========================================================================
// Canonical byte constant
// ===========================================================================

/// `HLT` — the Intel 8086 HALT instruction's opcode byte (see
/// [`intel8086_simulator::opcodes::HLT_OPCODE`]). Encoded value: `0xF4`.
/// A genuine single-byte hardware halt, unlike `mos6502_encoder::
/// HALT_BYTE` (a repurposed `BRK`) or `arm1_encoder::HALT_WORD` (an
/// invented pseudo-halt `SWI`).
pub const HALT_BYTE: u8 = intel8086_simulator::opcodes::HLT_OPCODE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_byte_matches_hlt_opcode() {
        assert_eq!(HALT_BYTE, intel8086_simulator::opcodes::HLT_OPCODE);
    }

    #[test]
    fn halt_byte_value() {
        assert_eq!(HALT_BYTE, 0xF4);
    }

    #[test]
    fn halt_byte_matches_encode_hlt() {
        assert_eq!(vec![HALT_BYTE], encode_hlt());
    }

    #[test]
    fn reg_ax_is_zero() {
        assert_eq!(REG_AX, 0);
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First instruction of the IIR `42` lowering: MOV AX, 42
        assert_eq!(encode_mov_reg_imm16(REG_AX, 42), vec![0xB8, 42, 0x00]);
    }

    #[test]
    fn word02_register_alu_encodings() {
        assert_eq!(
            encode_alu_reg_reg(0, false, REG_AL, REG_BL),
            vec![0x02, 0xC3]
        );
        assert_eq!(
            encode_alu_reg_reg(6, true, REG_AX, REG_BX),
            vec![0x33, 0xC3]
        );
        assert_eq!(encode_mov_reg_reg8(REG_CL, REG_AL), vec![0x8A, 0xC8]);
    }

    #[test]
    fn word03b_near_jump_encoding() {
        assert_eq!(encode_jmp_near(0x1234), vec![0xE9, 0x34, 0x12]);
        assert_eq!(encode_jmp_near(-3), vec![0xE9, 0xFD, 0xFF]);
    }

    #[test]
    fn assemble_flattens_mov_then_hlt() {
        assert_eq!(
            assemble(&[encode_mov_reg_imm16(REG_AX, 42), encode_hlt()]),
            vec![0xB8, 42, 0x00, 0xF4]
        );
    }
}
