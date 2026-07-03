//! # `armv7-encoder` — pure ARMv7-A (A32) instruction encoder.
//!
//! Mirror of [`ge225-encoder`] / [`intel4004-encoder`] /
//! [`aarch64-encoder`] for the **ARMv7-A** (32-bit ARM) instruction
//! set — the phone-class architecture deployed in billions of
//! Cortex-A7/A8/A9-era SoCs.
//!
//! ## What's in it
//!
//! - Encoding base constants (one per opcode family).
//! - Canonical word constants (`BX_LR`, `BKPT`).
//! - `encode_*` helpers — typed functions that take ARM register
//!   indices / immediates and return `u32` instruction words.
//!
//! No IR knowledge.  Consumed by `armv7-backend` and re-exported
//! by the deprecated `iir-to-armv7` for backwards compatibility.
//!
//! ## ABI assumed
//!
//! AAPCS32 — first integer/pointer argument in `r0`, return value
//! in `r0`, link register `lr`/`r14` holds the return address.
//!
//! ## Phase 5 of the historical-arch backend migration
//!
//! See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Quick start
//!
//! ```
//! use armv7_encoder::{encode_mov_imm, BX_LR};
//!
//! // MOV r0, #42 = 0xE3A0_002A
//! assert_eq!(encode_mov_imm(0, 42), 0xE3A0_002A);
//!
//! // BX LR = 0xE12F_FF1E
//! assert_eq!(BX_LR, 0xE12F_FF1E);
//! ```

// ===========================================================================
// Canonical word constants
// ===========================================================================

/// ARMv7-A `BX LR` (branch and exchange to link register) — the
/// AAPCS32 return-from-function instruction.
pub const BX_LR: u32 = 0xE12F_FF1E;

/// ARMv7-A `BKPT #0` — breakpoint trap, halts an emulator's
/// single-stepper.  Used as a HLT-equivalent in emit-only contexts.
pub const BKPT: u32 = 0xE12F_FF7F;

// ===========================================================================
// MOV-immediate (data-processing immediate)
// ===========================================================================

/// Base encoding for `MOV Rd, #imm8` with `Rd = r0`.  OR in
/// `(rd << 12) | imm8` to specialise.
pub const MOV_IMM_R0_BASE: u32 = 0xE3A0_0000;

/// Encode `MOV Rd, #imm8` — 8-bit immediate move.
///
/// `rd` is masked to 4 bits; `imm8` is masked to 8 bits.  Out-of-
/// range values are the caller's responsibility (the backend
/// range-checks at lowering time).
#[inline]
pub fn encode_mov_imm(rd: u8, imm8: u8) -> u32 {
    MOV_IMM_R0_BASE | (((rd & 0x0F) as u32) << 12) | (imm8 as u32)
}

// ===========================================================================
// MOV-register (data-processing register)
// ===========================================================================

/// Base encoding for `MOV Rd, Rm` (register-to-register).  OR in
/// `(Rd << 12) | Rm`.
pub const MOV_REG_BASE: u32 = 0xE1A0_0000;

/// Encode `MOV Rd, Rm` — register-to-register copy.
#[inline]
pub fn encode_mov_reg(rd: u8, rm: u8) -> u32 {
    MOV_REG_BASE | (((rd & 0x0F) as u32) << 12) | ((rm & 0x0F) as u32)
}

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Number of GP registers in the ABI scratch + saved set we
/// allocate from — 12 (`r0..r11`).  `r12 (ip)`, `r13 (sp)`,
/// `r14 (lr)`, `r15 (pc)` are reserved by the ABI.
pub const GP_REGISTER_COUNT: usize = 12;

/// Maximum 8-bit immediate `MOV` can carry directly (= 255).
/// Wider values require `movw`/`movt` pairs or rotated immediates,
/// neither of which v0.1.0 of `armv7-backend` supports.
pub const MOV_IMM_MAX: u32 = 255;
