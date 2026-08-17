//! # `m68k-encoder` — pure Motorola 68000 instruction encoder.
//!
//! Mirror of [`mos6502_encoder`] / [`arm1_encoder`] for the Motorola
//! 68000 (1979) — the landmark 16/32-bit processor behind the original
//! Macintosh, Commodore Amiga, Atari ST, early Sun workstations, and the
//! Sega Genesis.  Eighth lane of the 9-architecture expansion following
//! the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## What's inside
//!
//! 1. **Encoder re-exports** — `encode_move_l_imm_to_dn`/`encode_trap15`
//!    (plus [`assemble`]) come from [`m68k_simulator::encoding`].  We
//!    re-export them here so `m68k-backend` (Backend trait over CIR) can
//!    depend on a small, IR-agnostic surface without pulling the full
//!    simulator's decode/execute machinery into every consumer.  Future
//!    ISA-spec updates land in `m68k_simulator::encoding` and propagate
//!    automatically.
//! 2. **Register-role constant** — [`D0`], the data register
//!    `m68k-backend` writes `const_*` results into.  `D0`/`D1` are the
//!    68000's conventional scratch/return-value registers (see
//!    `code/packages/python/motorola-68000-simulator/src/
//!    motorola_68000_simulator/simulator.py`'s module doc: *"D0-D1, A0-A1
//!    — scratch / return values"*) — the same role `arm1-encoder`'s `R0`
//!    and `mips-r2000-backend`'s `$v0` play in their respective lanes.
//! 3. **Canonical byte constant** — [`HALT_BYTES`] for `TRAP #15`, the
//!    2-byte halt sentinel every program in `m68k-backend`'s scope ends
//!    with.
//!
//! No IR knowledge lives here.  Consumers map their IR onto encoder
//! calls + the register constant.
//!
//! ## Why `TRAP #15`, not `STOP #imm`?
//!
//! See `m68k_simulator`'s crate-level doc ("Halt convention") for the
//! full derivation — in short, the pre-existing Python simulator's own
//! `state.py` documents both `STOP` and `TRAP #15` as halting
//! conditions, but its own test suite's `_stop()` helper (used 100+
//! times) is `TRAP #15`, making it the dominant, already-established
//! idiom this lane mirrors rather than inventing a fresh convention.
//!
//! ## Quick start
//!
//! ```
//! use m68k_encoder::{assemble, encode_move_l_imm_to_dn, encode_trap15, D0, HALT_BYTES};
//!
//! // const_i64 v=42 lowered to `MOVE.L #42, D0` -- the first instruction
//! // `m68k-backend` emits for the canonical IIR `const 42; ret` program.
//! let bytes = assemble(&[encode_move_l_imm_to_dn(D0, 42), encode_trap15()]);
//! assert_eq!(bytes, vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]);
//! assert_eq!(&bytes[6..8], &HALT_BYTES);
//! ```

// ===========================================================================
// Encoder re-exports
// ===========================================================================
//
// `m68k_simulator` is the in-tree source of truth for the 68000 bit-level
// encoding -- it's the only place the opword-field packing logic lives.
// We re-export the subset of `encode_*` helpers (and `assemble`) that
// `m68k-backend` actually uses (plus a few more exercised by this
// crate's own tests).

pub use m68k_simulator::encoding::{
    assemble, encode_move_l_imm_to_dn, encode_moveq, encode_nop, encode_rts, encode_trap15,
};

// ===========================================================================
// Register-role constant
// ===========================================================================

/// `D0` — the 68000 return-value/scratch data register `m68k-backend`
/// writes `const_*` results into.  Mirrors `arm1-encoder::R0` /
/// `mips_r2000_encoder`'s `V0`.
pub const D0: u8 = 0;

// ===========================================================================
// Canonical byte constant
// ===========================================================================

/// `TRAP #15` — the HALT sentinel (see [`m68k_simulator::opcodes::TRAP_15_WORD`]
/// and this crate's "Why `TRAP #15`, not `STOP #imm`?" doc section).
/// Encoded value: `0x4E4F`, big-endian bytes `[0x4E, 0x4F]` — the
/// 68000's native byte order, so unlike `arm1_encoder::HALT_WORD` (which
/// stores ARM1's little-endian words) there is no endianness flip
/// between this constant and what [`encode_trap15`] returns.
pub const HALT_BYTES: [u8; 2] = [0x4E, 0x4F];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halt_bytes_matches_encode_trap15() {
        assert_eq!(HALT_BYTES.to_vec(), encode_trap15());
    }

    #[test]
    fn halt_bytes_value() {
        assert_eq!(HALT_BYTES, [0x4E, 0x4F]);
    }

    #[test]
    fn d0_is_register_zero() {
        assert_eq!(D0, 0);
    }

    #[test]
    fn canonical_const_42_bytes() {
        // First instruction of the IIR `42` lowering: MOVE.L #42, D0
        assert_eq!(
            encode_move_l_imm_to_dn(D0, 42),
            vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]
        );
    }

    #[test]
    fn assemble_flattens_move_then_trap() {
        assert_eq!(
            assemble(&[encode_move_l_imm_to_dn(D0, 42), encode_trap15()]),
            vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]
        );
    }
}
