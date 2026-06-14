//! # `intel4004-encoder` — pure Intel 4004 instruction encoder.
//!
//! Mirror of [`ge225-encoder`] / [`aarch64-encoder`] for the Intel
//! 4004, the **world's first commercial microprocessor** (1971).
//!
//! ## What's in it
//!
//! - Opcode high-nibble constants (`LDM_OPCODE`, `LD_OPCODE`,
//!   `XCH_OPCODE`).
//! - The canonical halt-loop word (`HALT_LOOP = [0x40, 0x00]`,
//!   `JUN 0x000` — jump-unconditional to ROM address 0).
//! - Capacity constants (`GP_REGISTER_COUNT`, `LDM_MAX`).
//! - `encode_*` helpers — one per opcode family.
//!
//! No IR knowledge.  No `jit-core` dependency.  Consumed by
//! `intel4004-backend` (Phase 4 of the historical-arch backend
//! migration) and re-exported by the deprecated `iir-to-intel4004`
//! for backwards compatibility.
//!
//! ## Background on the architectural correction
//!
//! See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## ISA quick reference (subset used here)
//!
//! | Mnemonic | Opcode | Bytes | Effect |
//! |----------|--------|-------|--------|
//! | `LDM n`  | `1101 nnnn` (`0xD0 \| n`) | 1 | ACC ← 4-bit immediate `n` |
//! | `LD r`   | `1010 rrrr` (`0xA0 \| r`) | 1 | ACC ← register `r` |
//! | `XCH r`  | `1011 rrrr` (`0xB0 \| r`) | 1 | ACC ↔ register `r` |
//! | `JUN a`  | `0100 aaaa aaaaaaaa` | 2 | unconditional jump to 12-bit ROM addr |
//!
//! The 4004 has **no formal HLT** — `JUN 0x000` from ROM address 0
//! loops on itself forever, which every 4004 simulator recognises
//! as "halt".
//!
//! ## Quick start
//!
//! ```
//! use intel4004_encoder::{encode_ldm, encode_xch, HALT_LOOP};
//!
//! // LDM 5 = 0xD5
//! assert_eq!(encode_ldm(5), 0xD5);
//!
//! // XCH r3 = 0xB3
//! assert_eq!(encode_xch(3), 0xB3);
//!
//! // HALT_LOOP = [0x40, 0x00]  (JUN 0x000)
//! assert_eq!(HALT_LOOP, [0x40, 0x00]);
//! ```

// ===========================================================================
// Opcode high nibbles
// ===========================================================================

/// `LDM n` — load 4-bit immediate into ACC.  High nibble.  Form:
/// `0xD0 | (n & 0x0F)`.
pub const LDM_OPCODE: u8 = 0xD0;

/// `LD r` — copy register `r` into ACC.  High nibble.  Form:
/// `0xA0 | (r & 0x0F)`.
pub const LD_OPCODE: u8 = 0xA0;

/// `XCH r` — exchange register `r` with ACC.  High nibble.  Form:
/// `0xB0 | (r & 0x0F)`.  This is the 4004's STORE-equivalent —
/// it's the only way to put ACC's contents into a GP register.
pub const XCH_OPCODE: u8 = 0xB0;

/// `JUN` high nibble — unconditional jump to a 12-bit ROM address.
/// 2-byte instruction: `0100 aaaa aaaaaaaa`.
pub const JUN_OPCODE: u8 = 0x40;

// ===========================================================================
// Canonical word constants
// ===========================================================================

/// Canonical 2-byte "halt loop" — `JUN 0x000`.
///
/// The 4004 has no formal `HLT`.  Emitted at ROM address 0, this
/// 2-byte instruction unconditionally jumps to address 0 — itself
/// — looping forever.  Every 4004 simulator treats this as "the
/// chip is stuck", which is the contract we want for a `ret_void`
/// at the end of a program.
pub const HALT_LOOP: [u8; 2] = [JUN_OPCODE, 0x00];

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Number of GP registers — 16 (`r0..r15`).  Combined with ACC,
/// this is the same 17-slot pool the `intel4004-backend` allocator
/// fills before falling back to `OutOfRegisters`.
pub const GP_REGISTER_COUNT: usize = 16;

/// Maximum unsigned 4-bit immediate `LDM` can carry (= 15).
pub const LDM_MAX: i32 = 15;

/// Minimum signed 4-bit immediate `LDM` can carry (= -8 via two's
/// complement reinterpretation; `LDM` itself is unsigned, but
/// negative `Operand::Int(-1)` etc. is accepted by reinterpreting
/// the low 4 bits).
pub const LDM_MIN_SIGNED: i32 = -8;

// ===========================================================================
// encode_* helpers
// ===========================================================================

/// Encode `LDM n` as a single byte.  The 4-bit immediate `n` is
/// masked into the low nibble.  Out-of-range values are the
/// caller's responsibility (the backend range-checks at lowering
/// time).
#[inline]
pub fn encode_ldm(n: u8) -> u8 {
    LDM_OPCODE | (n & 0x0F)
}

/// Encode `LD r` as a single byte.  The 4-bit register index is
/// masked into the low nibble.
#[inline]
pub fn encode_ld(r: u8) -> u8 {
    LD_OPCODE | (r & 0x0F)
}

/// Encode `XCH r` as a single byte.
#[inline]
pub fn encode_xch(r: u8) -> u8 {
    XCH_OPCODE | (r & 0x0F)
}

/// Encode `JUN addr` as 2 bytes — high nibble + 12-bit address.
///
/// `addr` is masked to 12 bits (`0x0FFF`).
#[inline]
pub fn encode_jun(addr: u16) -> [u8; 2] {
    let masked = addr & 0x0FFF;
    [
        JUN_OPCODE | ((masked >> 8) & 0x0F) as u8,
        (masked & 0xFF) as u8,
    ]
}
