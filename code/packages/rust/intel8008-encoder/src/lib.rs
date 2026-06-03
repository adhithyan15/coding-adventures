//! # `intel8008-encoder` — pure Intel 8008 instruction encoder.
//!
//! Mirror of [`ge225-encoder`] / [`intel4004-encoder`] /
//! [`armv7-encoder`] for the Intel 8008 (1972) — the
//! second-generation 8-bit Intel microprocessor.
//!
//! Phase 6 of the historical-arch backend migration.
//!
//! ## ISA quick reference (subset used here)
//!
//! | Mnemonic | Opcode | Bytes | Effect |
//! |----------|--------|-------|--------|
//! | `HLT` | `0x76` | 1 | halt — `01_110_110` |
//! | `MVI A, n` | `0x3E nn` | 2 | A ← 8-bit immediate `n` |
//! | `RET` | `0x07` | 1 | return from subroutine |
//!
//! ## Quick start
//!
//! ```
//! use intel8008_encoder::{encode_mvi_a, HLT, RET};
//!
//! // MVI A, 42 → [0x3E, 0x2A]
//! assert_eq!(encode_mvi_a(42), [0x3E, 0x2A]);
//! assert_eq!(HLT, 0x76);
//! assert_eq!(RET, 0x07);
//! ```

// ===========================================================================
// Opcodes
// ===========================================================================

/// `HLT` — halt the CPU.  Bit pattern: `01_110_110`.
pub const HLT: u8 = 0x76;

/// `RET` — return from subroutine (unconditional, encoded as
/// `00 000 111`).  Used for non-entry-function returns; the entry
/// function emits `HLT` instead.
pub const RET: u8 = 0x07;

/// `MVI A, n` — load 8-bit immediate into accumulator (register A).
/// 2-byte instruction: `0x3E nn`.
pub const MVI_A: u8 = 0x3E;

/// `JMP addr` — unconditional jump to a 14-bit address.  3-byte
/// instruction (`0x44` is JFC, NOT JMP — bit-2/3 family hazard).
pub const JMP: u8 = 0x7C;

/// `CAL addr` — call subroutine at 14-bit address.  3-byte
/// instruction.  Pairs with `RET` for function returns.
pub const CAL: u8 = 0x7E;

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Number of 7 GP registers (A, B, C, D, E, H, L).  The 8008 has
/// no fully general r0..r15 — these are named registers in the
/// AAPCS-like ABI the LANG VM uses.
pub const GP_REGISTER_COUNT: usize = 7;

/// Maximum unsigned 8-bit `MVI A` immediate (= 255).
pub const MVI_MAX: u8 = 255;

// ===========================================================================
// encode_* helpers
// ===========================================================================

/// Encode `MVI A, n` as a 2-byte instruction: `[0x3E, n]`.
#[inline]
pub fn encode_mvi_a(n: u8) -> [u8; 2] {
    [MVI_A, n]
}

/// Encode `JMP addr` as a 3-byte instruction.  The 14-bit address
/// is encoded as low byte first, then `(high_byte | 0x40)`.
#[inline]
pub fn encode_jmp(addr: u16) -> [u8; 3] {
    let masked = addr & 0x3FFF; // 14 bits
    [
        JMP,
        (masked & 0xFF) as u8,
        ((masked >> 8) & 0x3F) as u8,
    ]
}

/// Encode `CAL addr` as a 3-byte instruction (same address shape
/// as JMP).
#[inline]
pub fn encode_cal(addr: u16) -> [u8; 3] {
    let masked = addr & 0x3FFF;
    [
        CAL,
        (masked & 0xFF) as u8,
        ((masked >> 8) & 0x3F) as u8,
    ]
}
