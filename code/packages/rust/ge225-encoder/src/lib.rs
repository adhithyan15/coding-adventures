//! # `ge225-encoder` — pure GE-225 instruction encoder.
//!
//! Mirror of [`aarch64-encoder`] / [`x86_64-encoder`] for the GE-225,
//! the 1959 General Electric mainframe at Dartmouth College where
//! Dartmouth BASIC was designed in 1964.  This crate has **no IR
//! knowledge** — it owns the encoding tables (opcode constants and
//! `encode_*` helpers) and nothing else.
//!
//! ## Why a standalone encoder?
//!
//! Real architecture backends in this workspace pair an **encoder**
//! crate with a **backend** crate:
//!
//! | Arch   | Encoder              | Backend (consumes CIR)  |
//! |--------|----------------------|-------------------------|
//! | AArch64 | `aarch64-encoder`    | `aarch64-backend`       |
//! | x86-64  | `x86_64-encoder`     | `x86_64-backend`        |
//! | **GE-225** | **`ge225-encoder` (this)** | `ge225-backend` (Phase 2) |
//!
//! The encoder stays free of CIR / IIR / `jit-core` types so it can
//! also be consumed by a downstream simulator (`ge225-simulator`),
//! a custom decoder, or a fuzzer that just wants to emit valid words.
//!
//! Background on why the historical-arch backends are being moved
//! to this shape: see
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Word packing
//!
//! Each 20-bit GE-225 instruction word is emitted as **3 bytes**
//! (24 bits), big-endian, with the top 4 bits of byte 0 always zero:
//!
//! ```text
//! byte 0: 0000 OOOO   (top 4 bits zero + 4-bit opcode nibble)
//! byte 1: AAAA AAAA   (high 8 bits of immediate / address)
//! byte 2: AAAA AAAA   (low  8 bits — for STA/LD/ADD/SUB the low 4 bits hold the register index)
//! ```
//!
//! ## Opcode map
//!
//! | Nibble | Mnemonic | Word                | Effect                            |
//! |--------|----------|---------------------|-----------------------------------|
//! | `0x0`  | `HLT`    | `[0x00, 0x00, 0x00]` | halt the machine                  |
//! | `0x1`  | `LDA n`  | `[0x01, hi, lo]`     | ACC ← 16-bit immediate `n`        |
//! | `0x2`  | `STA r`  | `[0x02, 0x00, r]`    | ACC ↔ r (exchange — XCH semantics)|
//! | `0x3`  | `LD r`   | `[0x03, 0x00, r]`    | ACC ← r (copy; r unchanged)       |
//! | `0x4`  | `ADD r`  | `[0x04, 0x00, r]`    | ACC ← ACC + r                     |
//! | `0x5`  | `SUB r`  | `[0x05, 0x00, r]`    | ACC ← ACC − r                     |
//! | `0x6`  | `BR a`   | `[0x06, hi, lo]`     | unconditional branch              |
//! | `0x7`  | `BNZ a`  | `[0x07, hi, lo]`     | branch if ACC ≠ 0                 |
//! | `0x8`  | `BZ a`   | `[0x08, hi, lo]`     | branch if ACC = 0                 |
//! | `0x9`  | `JSR a`  | `[0x09, hi, lo]`     | push PC+3, branch to `a`          |
//! | `0xA`  | `RTS`    | `[0x0A, 0x00, 0x00]` | pop, branch to popped address     |
//! | `0xB`  | `BMI a`  | `[0x0B, hi, lo]`     | branch if ACC sign bit set        |
//!
//! Reserved for future ISA extensions: `0xC..0xF`.
//!
//! ## STA semantics (XCH on this skeleton)
//!
//! Real GE-225 silicon's `STA` was a pure store.  This skeleton
//! models `STA r` as **exchange-with-ACC** (`r ↔ ACC`) so the
//! eviction pattern needed by the accumulator-anchored allocator
//! is one instruction instead of two.  Documented in
//! `ge225-backend`'s allocator docs.
//!
//! ## Quick start
//!
//! ```
//! use ge225_encoder::{encode_lda, encode_sta, HALT_WORD};
//!
//! // LDA 5 = [0x01, 0x00, 0x05]
//! assert_eq!(encode_lda(5), [0x01, 0x00, 0x05]);
//!
//! // STA r3 = [0x02, 0x00, 0x03]  (register index masked to 4 bits)
//! assert_eq!(encode_sta(3), [0x02, 0x00, 0x03]);
//!
//! // HLT = all zeros
//! assert_eq!(HALT_WORD, [0x00, 0x00, 0x00]);
//! ```

// ===========================================================================
// Opcode nibbles
// ===========================================================================

/// `LDA` (load accumulator with 16-bit immediate) opcode nibble.
pub const LDA_OPCODE_NIBBLE: u8 = 0x1;

/// `STA` (exchange-with-ACC on this skeleton) opcode nibble.
pub const STA_OPCODE_NIBBLE: u8 = 0x2;

/// `LD` (copy register into ACC) opcode nibble.
pub const LD_OPCODE_NIBBLE: u8 = 0x3;

/// `ADD` opcode nibble.  `ACC ← ACC + r`.
pub const ADD_OPCODE_NIBBLE: u8 = 0x4;

/// `SUB` opcode nibble.  `ACC ← ACC − r`.
pub const SUB_OPCODE_NIBBLE: u8 = 0x5;

/// `BR` (unconditional branch) opcode nibble.
pub const BR_OPCODE_NIBBLE: u8 = 0x6;

/// `BNZ` (branch if non-zero) opcode nibble.
pub const BNZ_OPCODE_NIBBLE: u8 = 0x7;

/// `BZ` (branch if zero) opcode nibble.
pub const BZ_OPCODE_NIBBLE: u8 = 0x8;

/// `JSR` (jump subroutine) opcode nibble.
pub const JSR_OPCODE_NIBBLE: u8 = 0x9;

/// `RTS` (return from subroutine) opcode nibble.
pub const RTS_OPCODE_NIBBLE: u8 = 0xA;

/// `BMI` (branch if minus / ACC sign bit set) opcode nibble.
pub const BMI_OPCODE_NIBBLE: u8 = 0xB;

// ===========================================================================
// Canonical word constants
// ===========================================================================

/// Canonical 3-byte `HLT` word (= all zeros).
pub const HALT_WORD: [u8; 3] = [0x00, 0x00, 0x00];

/// Canonical 3-byte `RTS` word.
pub const RTS_WORD: [u8; 3] = [RTS_OPCODE_NIBBLE, 0x00, 0x00];

// ===========================================================================
// Capacity constants
// ===========================================================================

/// Number of GP registers (`r0..r15`).  Combined with ACC this is
/// a 17-slot register pool — identical to the
/// `intel4004-encoder`'s eventual capacity, kept for symmetry.
pub const GP_REGISTER_COUNT: usize = 16;

/// Largest signed 16-bit immediate `LDA n` can carry (`32_767`).
pub const LDA_MAX_SIGNED: i32 = 32_767;

/// Smallest signed 16-bit immediate `LDA n` can carry (`-32_768`).
pub const LDA_MIN_SIGNED: i32 = -32_768;

/// Largest unsigned 16-bit immediate `LDA n` can carry (`65_535`).
/// Values in `[32_768, 65_535]` are accepted via two's-complement
/// reinterpretation.
pub const LDA_MAX_UNSIGNED: i32 = 65_535;

// ===========================================================================
// encode_* helpers
// ===========================================================================

/// Encode `LDA imm16` as 3 bytes, big-endian.
///
/// The caller is responsible for fitting the value into 16 bits
/// (see `LDA_MIN_SIGNED` / `LDA_MAX_UNSIGNED`).  `ge225-backend`
/// performs the range check at lowering time; this helper just
/// packs whatever `u16` the caller hands in.
#[inline]
pub fn encode_lda(imm16: u16) -> [u8; 3] {
    [
        LDA_OPCODE_NIBBLE,
        ((imm16 >> 8) & 0xFF) as u8,
        (imm16 & 0xFF) as u8,
    ]
}

/// Encode `STA r` as 3 bytes.  `r` is masked to 4 bits.
#[inline]
pub fn encode_sta(r: u8) -> [u8; 3] {
    [STA_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Encode `LD r` as 3 bytes.  `r` is masked to 4 bits.
#[inline]
pub fn encode_ld(r: u8) -> [u8; 3] {
    [LD_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Encode `ADD r` as 3 bytes.  `r` is masked to 4 bits.
#[inline]
pub fn encode_add(r: u8) -> [u8; 3] {
    [ADD_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Encode `SUB r` as 3 bytes.  `r` is masked to 4 bits.
#[inline]
pub fn encode_sub(r: u8) -> [u8; 3] {
    [SUB_OPCODE_NIBBLE, 0x00, r & 0x0F]
}

/// Encode `BR addr` (unconditional branch) as 3 bytes, big-endian
/// 16-bit byte address.
#[inline]
pub fn encode_br(addr: u16) -> [u8; 3] {
    encode_branch(BR_OPCODE_NIBBLE, addr)
}

/// Encode `BNZ addr` (branch if non-zero).
#[inline]
pub fn encode_bnz(addr: u16) -> [u8; 3] {
    encode_branch(BNZ_OPCODE_NIBBLE, addr)
}

/// Encode `BZ addr` (branch if zero).
#[inline]
pub fn encode_bz(addr: u16) -> [u8; 3] {
    encode_branch(BZ_OPCODE_NIBBLE, addr)
}

/// Encode `BMI addr` (branch if ACC sign bit set).
#[inline]
pub fn encode_bmi(addr: u16) -> [u8; 3] {
    encode_branch(BMI_OPCODE_NIBBLE, addr)
}

/// Encode `JSR addr` (push PC+3 and branch).
#[inline]
pub fn encode_jsr(addr: u16) -> [u8; 3] {
    encode_branch(JSR_OPCODE_NIBBLE, addr)
}

/// Shared big-endian-address encoder used by `BR` / `BNZ` / `BZ`
/// / `BMI` / `JSR`.
#[inline]
fn encode_branch(opcode_nibble: u8, addr: u16) -> [u8; 3] {
    [
        opcode_nibble,
        ((addr >> 8) & 0xFF) as u8,
        (addr & 0xFF) as u8,
    ]
}

// ===========================================================================
// Decoding helpers (for downstream simulators / decoders)
// ===========================================================================

/// Decode a 3-byte big-endian word into `(opcode_nibble,
/// address_or_immediate_low_16_bits)`.  Strips the top 4 bits of
/// byte 0 (which are always zero on this skeleton).
///
/// Useful for the eventual `ge225-simulator` (which can step
/// the bytes a `ge225-backend` produced), or for any
/// downstream decoder.
#[inline]
pub fn decode_word(word: [u8; 3]) -> (u8, u16) {
    let opcode = word[0] & 0x0F;
    let payload = ((word[1] as u16) << 8) | (word[2] as u16);
    (opcode, payload)
}
