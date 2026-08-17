//! # `sparc-v8-encoder` — pure SPARC V8 instruction encoder.
//!
//! Mirror of [`mips_r2000_encoder`] / [`arm1_encoder`] for the SPARC V8
//! (1987) — the first **open** RISC instruction-set standard, designed
//! by Sun Microsystems and later powering Sun SPARCstation
//! workstations and Solaris servers for two decades.  Sixth lane of the
//! 9-architecture expansion following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — the canonical `encode_*` helpers (e.g.
//!    `encode_add_imm`, `encode_ta`) come from
//!    [`sparc_v8_simulator::encoding`].  We re-export them here so that
//!    `sparc-v8-backend` (Backend trait over CIR) can depend on a
//!    small, IR-agnostic surface without pulling the full simulator's
//!    decode/execute machinery into every consumer.  Future ISA-spec
//!    updates land in `sparc-v8-simulator::encoding` and propagate
//!    automatically.
//! 2. **Register-role constants** — the small subset of architectural
//!    registers `sparc-v8-backend` actually touches: `G0` (hardwired
//!    zero) and `O0` (the SPARC calling-convention return-value
//!    register).  See the crate-level rationale below for why `%o0`,
//!    not a `%g` register, is the return-value choice.
//! 3. **Canonical word constant** — `HALT_WORD` for `ta 0`, ported
//!    from [`sparc_v8_simulator::opcodes::HALT_WORD`].
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder
//! calls + the register constants.
//!
//! ## Why `%o0`, not a `%g` register, for the return value?
//!
//! Real SPARC ABI convention (the "C calling convention" documented in
//! the SPARC V8 manual and every SunOS/Solaris ABI doc) returns
//! integer values in `%o0` — the register that becomes `%i0` in the
//! *caller's* view once the callee's `RESTORE` rotates the window back.
//! `%g1`-`%g7` are explicitly reserved as scratch/library-private
//! registers, not the return-value slot.
//!
//! `%o0` (virtual register 8) is **not** one of the 8 CWP-independent
//! globals — it is a windowed register, physically
//! `8 + CWP*16` when read/written.  This lane's v0.1.0 scope never
//! executes `SAVE`/`RESTORE`, so CWP is always 0 for the lifetime of a
//! compiled program: `%o0` therefore always resolves to the same fixed
//! physical register (index 8), exactly as if it were a global, with
//! zero risk of window-rotation surprises.  This is the same
//! "sidestep the windowing complexity by never rotating the window"
//! scoping the task spec calls out — using `%o0` doesn't reintroduce
//! that complexity, since the window literally never moves in this
//! lane's programs.
//!
//! ## Quick start
//!
//! ```
//! use sparc_v8_encoder::{encode_add_imm, HALT_WORD, G0, O0, encode_ta};
//!
//! // const_i64 v=42 lowered to `ADD %g0, 42, %o0` — the first word
//! // `sparc-v8-backend` emits for the canonical Twig `42` program.
//! let const_word = encode_add_imm(O0, G0, 42);
//! assert_eq!(const_word, 0x9000_202A);
//!
//! // ret  lowered to `ta 0`.
//! let ret_word = encode_ta(0);
//! assert_eq!(ret_word, HALT_WORD);
//! assert_eq!(HALT_WORD, 0x91D0_2000);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `sparc-v8-simulator::encoding` is the in-tree source of truth for the
// SPARC V8 bit layout — it's the only place the op/op2/op3 field
// packing logic lives.  We re-export the subset of `encode_*` helpers
// that `sparc-v8-backend` actually uses.

pub use sparc_v8_simulator::encoding::{assemble, encode_add_imm, encode_ta};
pub use sparc_v8_simulator::opcodes::HALT_WORD;

// ===========================================================================
// Register layout (the small subset sparc-v8-backend touches by index)
// ===========================================================================
//
// SPARC V8 has 32 logical registers visible in any window: %g0-%g7
// (globals), %o0-%o7 (outs), %l0-%l7 (locals), %i0-%i7 (ins).  See
// `sparc_v8_simulator::registers` for the full windowed-addressing
// derivation.  `sparc-v8-backend` only needs to name the two it
// touches directly.

/// `%g0` — hardwired to zero.  Writes are silently discarded; reads
/// always yield zero.  Used as `rs1` for `ADD %g0, imm, rd` to
/// materialise immediates (the SPARC idiom for "load a small constant"
/// when the value doesn't need `SETHI`'s upper-22-bit range).
pub const G0: u32 = 0;

/// `%o0` (virtual register 8) — the SPARC calling-convention
/// return-value register.  After a `RESTORE`, this same physical
/// register is read by the caller as `%i0`.  See this crate's
/// module docs for why it's safe to treat as a fixed register in a
/// backend that never executes `SAVE`/`RESTORE`.
pub const O0: u32 = 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_word_matches_encode_ta_zero() {
        assert_eq!(HALT_WORD, encode_ta(0));
    }

    #[test]
    fn halt_word_value() {
        assert_eq!(HALT_WORD, 0x91D0_2000);
    }

    #[test]
    fn halt_word_big_endian_bytes() {
        assert_eq!(HALT_WORD.to_be_bytes(), [0x91, 0xD0, 0x20, 0x00]);
    }

    #[test]
    fn register_constants_match_convention() {
        assert_eq!(G0, 0);
        assert_eq!(O0, 8);
    }

    #[test]
    fn canonical_const_42_word() {
        // First instruction of the Twig `42` lowering: ADD %g0, 42, %o0
        assert_eq!(encode_add_imm(O0, G0, 42), 0x9000_202A);
    }

    #[test]
    fn assemble_flattens_to_big_endian() {
        assert_eq!(assemble(&[HALT_WORD]), vec![0x91, 0xD0, 0x20, 0x00]);
    }
}
