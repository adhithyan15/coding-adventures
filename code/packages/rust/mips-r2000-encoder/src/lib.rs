//! # `mips-r2000-encoder` — pure MIPS R2000 instruction encoder.
//!
//! Mirror of [`riscv-encoder`] / [`armv7-encoder`] / [`intel8008-encoder`]
//! for the MIPS R2000 (1985) — the first commercially successful RISC
//! processor.  First lane of the 9-architecture expansion following the
//! pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — the canonical `encode_*` helpers (e.g.
//!    `encode_addiu`, `encode_jr`) come from
//!    [`mips_r2000_simulator::encoding`].  We re-export them here so that
//!    `mips-r2000-backend` (Backend trait over CIR) can depend on a small,
//!    IR-agnostic surface without pulling the full simulator into every
//!    consumer.  Future ISA-spec updates land in
//!    `mips-r2000-simulator::encoding` and propagate automatically.
//! 2. **Register-role constants** — the small subset of architectural
//!    registers `mips-r2000-backend` actually touches: `ZERO`, `V0`,
//!    `RA`, plus a `TEMP_REGISTERS` pool for future increments.  These
//!    mirror the MIPS o32-ish calling convention documented in
//!    `mips_r2000_simulator/state.py`.
//! 3. **Canonical word constant** — `RET_WORD` for `JR $ra` (the
//!    universal MIPS R2000 return-from-function; a fixed encoding since
//!    it carries no immediate).
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder calls
//! + the register table.
//!
//! ## ISA quick reference (subset used by the backend)
//!
//! | Mnemonic | Encoding | Effect |
//! |----------|----------|--------|
//! | `addiu rt, rs, imm` (I-type) | `op(9) \| rs \| rt \| imm16` | `rt ← rs + sext(imm)`, no overflow trap |
//! | `jr rs` (R-type) | `op(0) \| rs \| 0 \| 0 \| 0 \| funct(0x08)` | `pc ← rs` |
//!
//! `JR $ra` (i.e. `0x03E0_0008`) is the canonical "return" used at every
//! function epilogue — unlike RISC-V's `jalr`, it carries no immediate, so
//! it is a single fixed word.
//!
//! ## Quick start
//!
//! ```
//! use mips_r2000_encoder::{encode_addiu, RET_WORD, ZERO, V0, encode_jr, RA};
//!
//! // const_i64 v=42 lowered to `addiu $v0, $zero, 42` — the bytes
//! // `mips-r2000-backend` emits for the canonical Twig `42` program.
//! let const_word = encode_addiu(V0, ZERO, 42);
//! assert_eq!(const_word, 0x2402_002A);
//!
//! // ret  lowered to `jr $ra`.
//! let ret_word = encode_jr(RA);
//! assert_eq!(ret_word, RET_WORD);
//! assert_eq!(RET_WORD, 0x03E0_0008);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `mips-r2000-simulator::encoding` is the in-tree source of truth for the
// MIPS R2000 bit layout — it's the only place the opcode/funct constants
// and the R/I/J-format packing logic lives.  We re-export the subset of
// `encode_*` helpers that `mips-r2000-backend` actually uses.

pub use mips_r2000_simulator::encoding::{assemble, encode_addiu, encode_jr};

// ===========================================================================
// Register layout (the small subset mips-r2000-backend touches by index)
// ===========================================================================
//
// MIPS R2000 has 32 general-purpose registers: R0..R31.  Convention
// assigns canonical roles to several of them (see
// `mips_r2000_simulator::state` in the Python original for the full
// alias table).  `mips-r2000-backend` only needs to name a few directly —
// the rest come from the temporary pool below.

/// `R0` (`$zero`) — hardwired to zero.  Writes are silently discarded;
/// reads always yield zero.  Used as `rs` for `addiu rt, $zero, n` to
/// materialise immediates.
pub const ZERO: u32 = 0;

/// `R2` (`$v0`) — return value (also the syscall number on MIPS Linux).
/// After a `ret`, this holds the integer return value the caller reads.
pub const V0: u32 = 2;

/// `R3` (`$v1`) — second return-value word (used for wide/pair returns
/// in a future increment).
pub const V1: u32 = 3;

/// `R4` (`$a0`) — first argument register.
pub const A0: u32 = 4;

/// `R29` (`$sp`) — stack pointer.  Reserved here for future
/// call-prologue support.
pub const SP: u32 = 29;

/// `R31` (`$ra`) — return address, set by `JAL`/`JALR` and consumed by
/// `JR $ra` at every function epilogue.
pub const RA: u32 = 31;

// ---------------------------------------------------------------------------
// Temporary registers — `$t0..$t7` per convention
// ---------------------------------------------------------------------------
//
// A future register-allocator increment to `mips-r2000-backend` can hand
// out temps from this pool, one per `dest`, in declaration order — the
// same shape `riscv-encoder::TEMP_REGISTERS` uses.  v0.1.0 of the backend
// does not use this pool yet (it writes `const_*` directly into `$v0`,
// mirroring the single-accumulator style of `armv7-backend`/
// `intel8008-backend`), but the constant is declared now so the encoder's
// public surface does not need a breaking change when a real allocator
// lands.

/// `$t0..$t7` = `[R8..R15]` — caller-saved general-purpose temporaries.
pub const TEMP_REGISTERS: [u32; 8] = [8, 9, 10, 11, 12, 13, 14, 15];

// ===========================================================================
// Canonical instruction-word constants
// ===========================================================================
//
// Pre-computed words for the sequences the e2e smoke tests pin.

/// `JR $ra` — the canonical MIPS R2000 "return from function".  Encoded
/// value: `0x03E0_0008`.  Stored big-endian on disk as
/// `[0x03, 0xE0, 0x00, 0x08]` (MIPS R2000's default byte order).
///
/// Unlike RISC-V's `jalr x0, x1, 0` (which carries an immediate field),
/// `JR` has no immediate — the word is fixed regardless of caller
/// context, so it is safe to precompute as a constant rather than a
/// function call.
pub const RET_WORD: u32 = 0x03E0_0008;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ret_word_matches_encode_jr_ra() {
        assert_eq!(RET_WORD, encode_jr(RA));
    }

    #[test]
    fn ret_word_value() {
        assert_eq!(RET_WORD, 0x03E0_0008);
    }

    #[test]
    fn ret_word_big_endian_bytes() {
        assert_eq!(RET_WORD.to_be_bytes(), [0x03, 0xE0, 0x00, 0x08]);
    }

    #[test]
    fn register_constants_match_convention() {
        assert_eq!(ZERO, 0);
        assert_eq!(V0, 2);
        assert_eq!(V1, 3);
        assert_eq!(A0, 4);
        assert_eq!(SP, 29);
        assert_eq!(RA, 31);
    }

    #[test]
    fn temp_registers_are_t0_through_t7() {
        assert_eq!(TEMP_REGISTERS, [8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn canonical_const_42_word() {
        // First instruction of the Twig `42` lowering:
        // ADDIU $v0, $zero, 42
        assert_eq!(encode_addiu(V0, ZERO, 42), 0x2402_002A);
    }

    #[test]
    fn assemble_flattens_to_big_endian() {
        assert_eq!(assemble(&[RET_WORD]), vec![0x03, 0xE0, 0x00, 0x08]);
    }
}
