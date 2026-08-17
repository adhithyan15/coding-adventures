//! # `arm1-encoder` — pure ARM1 (ARMv1) instruction encoder.
//!
//! Mirror of [`mips_r2000_encoder`] / [`armv7_encoder`] /
//! [`intel8008_encoder`] for the ARM1 (1985) — Sophie Wilson and
//! Steve Furber's original Acorn RISC Machine, the first commercially
//! successful RISC chip and architectural ancestor of the
//! already-migrated `armv7-backend` lane.  Second lane of the
//! 9-architecture expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — the canonical `encode_mov_imm` /
//!    `encode_halt` helpers (plus the `COND_AL` condition constant)
//!    come from [`arm1_simulator`].  We re-export them here so that
//!    `arm1-backend` (Backend trait over CIR) can depend on a small,
//!    IR-agnostic surface without pulling the full simulator's
//!    decode/execute machinery into every consumer.  Future
//!    ISA-spec updates land in `arm1_simulator` and propagate
//!    automatically.
//! 2. **Register-role constant** — `R0`, the return-value register
//!    `arm1-backend` writes `const_*` results into.  ARM1/ARMv1
//!    predates the AAPCS calling-convention documents, but `R0` is
//!    the register every hand-written ARM1 example in
//!    `code/specs/07e-arm1-simulator.md` (and the simulator's own
//!    `test_mov_imm_and_halt`) uses to carry a computed value —
//!    the same role AAPCS32 later formalised, and the same role
//!    `armv7-backend`'s `r0`/`mips-r2000-backend`'s `$v0` play in
//!    their respective lanes.
//! 3. **Canonical word constant** — `HALT_WORD` for the pseudo-halt
//!    `SWI #0x123456` (AL-conditioned) that terminates a program;
//!    a fixed encoding since `encode_halt()` takes no arguments.
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder
//! calls + the register constant.
//!
//! ## Why `SWI`, not `BX LR`?
//!
//! `armv7-backend` (ARMv7-A, 2 decades newer) returns via `BX LR`,
//! the link-register-return convention every modern ARM ABI uses.
//! ARM1/ARMv1 (1985) predates that convention entirely — there is
//! no `BX` instruction, and `MOVS PC, R14` (the ARMv1-era subroutine
//! return) requires a live `R14` set by a preceding `BL`, which the
//! minimal-viable `const_*`/`ret_*` scope never establishes (there
//! is no caller).  `arm1-simulator` instead defines a **pseudo-halt**
//! instruction — `SWI #0x123456` — that its `execute_swi` intercepts
//! specially (see `arm1_simulator::ARM1::execute_swi`): when the
//! 24-bit SWI comment field equals `HALT_SWI` (`0x123456`), the
//! simulator sets its internal `halted` flag and stops, rather than
//! entering Supervisor mode like a real SWI would.  This is a
//! simulator-level convention (analogous to the Intel 8008 backend's
//! `HLT` or the GE-225 backend's `HLT` word), not real ARM1 silicon
//! behaviour — real ARM1 silicon has no way to "halt"; a real
//! program would branch to itself or await an interrupt.  Since
//! `arm1_simulator::ARM1::halted()` is the only way to observe
//! "the program is done" from outside the simulator, lowering
//! `ret_*`/`ret_void` to this pseudo-halt is the semantically
//! correct choice for a `const_*`-only minimal-viable backend: it
//! is what actually stops the fetch-decode-execute loop and leaves
//! the computed value sitting in `R0` for the caller to read via
//! `read_register(0)`.
//!
//! ## Quick start
//!
//! ```
//! use arm1_encoder::{encode_mov_imm, HALT_WORD, COND_AL, R0, encode_halt};
//!
//! // const_i64 v=42 lowered to `MOV R0, #42` — the first word
//! // `arm1-backend` emits for the canonical Twig `42` program.
//! let const_word = encode_mov_imm(COND_AL, R0, 42);
//! assert_eq!(const_word, 0xE3A0_002A);
//!
//! // ret  lowered to the pseudo-halt `SWI #0x123456`.
//! assert_eq!(HALT_WORD, encode_halt());
//! assert_eq!(HALT_WORD, 0xEF12_3456);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `arm1_simulator` is the in-tree source of truth for the ARM1 bit
// layout — it's the only place the condition/opcode field packing
// logic lives.  We re-export the subset of `encode_*` helpers (and
// the `COND_AL` condition constant) that `arm1-backend` actually
// uses.

pub use arm1_simulator::{encode_halt, encode_mov_imm, COND_AL};

// ===========================================================================
// Register layout (the one register arm1-backend touches by index)
// ===========================================================================
//
// The ARM1 has 16 visible general-purpose registers, R0-R15 (with
// R13/R14/R15 carrying architectural roles — stack pointer, link
// register, and combined PC/status register respectively).
// `arm1-backend` v0.1.0 only needs to name the one it writes
// computed values into.

/// `R0` — the ARM1 return-value register.  `const_*` writes its
/// literal here; the caller reads the result via
/// `arm1_simulator::ARM1::read_register(0)` once the pseudo-halt
/// stops execution.  Mirrors `armv7-backend`'s `r0` and
/// `mips-r2000-backend`'s `$v0`/`V0`.
pub const R0: u32 = 0;

// ===========================================================================
// Canonical instruction-word constants
// ===========================================================================
//
// Pre-computed words for the sequence the e2e smoke tests pin.

/// The pseudo-halt instruction — `SWI #0x123456`, `AL`-conditioned.
/// Encoded value: `0xEF12_3456`.  Stored little-endian on disk as
/// `[0x56, 0x34, 0x12, 0xEF]` (ARM1's byte order — see
/// `arm1_simulator::ARM1::read_word`/`write_word`, which use
/// `u32::from_le_bytes`/`to_le_bytes`).
///
/// `encode_halt()` takes no arguments, so its result is fixed
/// regardless of caller context — safe to precompute as a constant
/// rather than a function call, mirroring
/// `mips_r2000_encoder::RET_WORD`.
pub const HALT_WORD: u32 = 0xEF12_3456;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_word_matches_encode_halt() {
        assert_eq!(HALT_WORD, encode_halt());
    }

    #[test]
    fn halt_word_value() {
        assert_eq!(HALT_WORD, 0xEF12_3456);
    }

    #[test]
    fn halt_word_little_endian_bytes() {
        assert_eq!(HALT_WORD.to_le_bytes(), [0x56, 0x34, 0x12, 0xEF]);
    }

    #[test]
    fn r0_is_register_zero() {
        assert_eq!(R0, 0);
    }

    #[test]
    fn cond_al_matches_simulator_constant() {
        assert_eq!(COND_AL, 0xE);
    }

    #[test]
    fn canonical_const_42_word() {
        // First instruction of the Twig `42` lowering: MOV R0, #42
        assert_eq!(encode_mov_imm(COND_AL, R0, 42), 0xE3A0_002A);
    }

    #[test]
    fn canonical_const_42_little_endian_bytes() {
        assert_eq!(
            encode_mov_imm(COND_AL, R0, 42).to_le_bytes(),
            [0x2A, 0x00, 0xA0, 0xE3]
        );
    }
}
