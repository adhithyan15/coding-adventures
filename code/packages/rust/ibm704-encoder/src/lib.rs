//! # `ibm704-encoder` — pure IBM 704 instruction encoder.
//!
//! Mirror of [`ge225-encoder`] / [`intel4004-encoder`] /
//! [`armv7-encoder`] / [`intel8008-encoder`] / [`riscv-encoder`]
//! for the IBM 704 (1954) — the vacuum-tube mainframe John
//! McCarthy and his MIT students first ran Lisp on in 1959.
//!
//! L4 of the McCarthy Lisp implementation — see
//! [`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).
//!
//! ## Why this matters — the Lisp birthplace round-trip
//!
//! `CAR` and `CDR` — the two universal Lisp accessors — were
//! **literally IBM 704 instruction mnemonics** when McCarthy
//! invented Lisp at MIT in 1958-1960:
//!
//! * **CAR** = **C**ontents of **A**ddress part of **R**egister
//! * **CDR** = **C**ontents of **D**ecrement part of **R**egister
//!
//! That comes from the 704's instruction word format: a 36-bit
//! word was split into a "prefix", a "decrement" field, a "tag"
//! field, and an "address" field, and an indirect cons cell
//! happened to fit one cell per word.  McCarthy's `(CAR x)`
//! extracted the address half; `(CDR x)` extracted the decrement
//! half.  The names stuck.
//!
//! Adding the IBM 704 as a LANG VM backend lets McCarthy Lisp
//! programs round-trip to the very silicon they were originally
//! designed for — the symmetric counterpart of the **Dartmouth
//! BASIC → GE-225** round-trip the historical-arch migration
//! already shipped (Dartmouth BASIC was designed *on* the GE-225
//! in 1964).
//!
//! ## Word format (idealised, v0.1.0)
//!
//! The 704 has 36-bit words.  Original SAP (Symbolic Assembly
//! Program) instruction layout was more nuanced (Type A vs
//! Type B; prefix + decrement + tag + address fields), but for
//! the minimal-viable McCarthy compile target we use a
//! documented simplified layout:
//!
//! | Word bits | Field | Notes |
//! |-----------|-------|-------|
//! | 35..27 (9) | Opcode | e.g. `HTR=0o420`, `CLA=0o500` |
//! | 26..15 (12) | (zero) | tag + decrement + unused; not used in v0.1.0 |
//! | 14..0 (15) | Address Y | 15-bit address (≤ 32 K word memory) |
//!
//! ## Wire format — 5 bytes per word
//!
//! 36 bits don't divide evenly into 8.  We pack each word as 5
//! bytes (40 bits, 4 wasted), low byte first, high 4 bits of
//! the top byte always zero.  Same convention `ge225-encoder`
//! uses for its 20-bit words → 3 bytes.  Downstream consumers
//! (a future `ibm704-simulator`) read 5 bytes and mask off the
//! 4 high bits.
//!
//! ## Quick start
//!
//! ```
//! use ibm704_encoder::{encode_cla, encode_htr, pack_word, HTR, CLA};
//!
//! // McCarthy's canonical "42" program:
//! //   CLA 42   ; load 42 into the accumulator
//! //   HTR  0   ; halt; final value lives in AC
//! assert_eq!(HTR, 0o420);
//! assert_eq!(CLA, 0o500);
//!
//! let cla_42 = encode_cla(42);
//! let htr_0  = encode_htr(0);
//!
//! // 36-bit word values
//! assert_eq!(cla_42 & 0xFFF_FFFF_FFFF, 0xA_0000_002A);
//! assert_eq!(htr_0  & 0xFFF_FFFF_FFFF, 0x8_8000_0000);
//!
//! // 5-byte little-endian packing
//! assert_eq!(pack_word(cla_42), [0x2A, 0x00, 0x00, 0x00, 0x0A]);
//! assert_eq!(pack_word(htr_0),  [0x00, 0x00, 0x00, 0x80, 0x08]);
//! ```

// ===========================================================================
// Opcodes
// ===========================================================================
//
// The 704 had ~100 instructions; we only need two for the v0.1.0
// minimal-viable backend (matches the const_*/ret_* scope of every
// historical-arch backend in Phases 4-7).

/// `HTR Y` — **H**alt and **T**ransfe**R**.  Opcode `0o420`
/// (octal 420 = `0b100_010_000`).
///
/// Stops the CPU and parks the program counter at address `Y`.
/// `HTR 0` is the canonical halt sentinel: a jump-to-self halt,
/// safe to fall off the end of an emitted program into.  Same
/// idiom GE-225 / Intel 4004 / Intel 8008 use for their halts.
pub const HTR: u16 = 0o420;

/// `CLA Y` — **CL**ear accumulator and **A**dd memory at `Y`.
/// Opcode `0o500` (octal 500 = `0b101_000_000`).
///
/// Used by the v0.1.0 backend to materialise a 15-bit immediate
/// into the accumulator (`AC ← memory[Y]`, with `Y` itself
/// treated as the immediate value the linker would point at).
pub const CLA: u16 = 0o500;

// ===========================================================================
// Word geometry
// ===========================================================================

/// Number of bits in a 704 word.  Used by `pack_word` to mask off
/// any stray high bits before serialising.
pub const WORD_BITS: u32 = 36;

/// Mask covering exactly the 36 valid word bits.
pub const WORD_MASK: u64 = (1u64 << WORD_BITS) - 1;

/// Number of bytes used to serialise one word on disk.
///
/// 5 bytes × 8 bits = 40 bits ≥ 36-bit word.  The 4 high bits of
/// the top byte are always zero by `pack_word`'s construction —
/// they ride the wire purely as padding to keep the byte layout
/// regular.
pub const BYTES_PER_WORD: usize = 5;

/// Width of the 704's address field in bits.  Caps the 15-bit
/// address space at 32 K words (≈ 144 KB).
pub const ADDR_BITS: u32 = 15;

/// Mask covering exactly the 15-bit address field.
pub const ADDR_MASK: u64 = (1u64 << ADDR_BITS) - 1;

/// Bit position the opcode starts at within a 36-bit word.
///
/// Layout (MSB-first): `[opcode 9 bits @ 35..27][zero 12 bits][address 15 bits @ 14..0]`.
pub const OPCODE_SHIFT: u32 = 27;

// ===========================================================================
// encode_* helpers
// ===========================================================================

/// Encode `<opcode> <address>` into a 36-bit instruction word.
///
/// The opcode occupies bits 35..27 (9 bits); the address occupies
/// bits 14..0 (15 bits).  Bits 26..15 are zero — not yet used by
/// v0.1.0.  An address out of the 15-bit range is masked, never
/// errors.
#[inline]
pub fn encode_instruction(opcode: u16, address: u16) -> u64 {
    let op = (opcode as u64) << OPCODE_SHIFT;
    let addr = (address as u64) & ADDR_MASK;
    (op | addr) & WORD_MASK
}

/// Encode `HTR Y` — halt and transfer to `Y`.
#[inline]
pub fn encode_htr(address: u16) -> u64 {
    encode_instruction(HTR, address)
}

/// Encode `CLA Y` — clear-and-add: `AC ← memory[Y]`.
#[inline]
pub fn encode_cla(address: u16) -> u64 {
    encode_instruction(CLA, address)
}

// ===========================================================================
// Wire format
// ===========================================================================

/// Pack a 36-bit word into 5 bytes, low byte first.
///
/// Word bits 0..7 land in `out[0]`, 8..15 in `out[1]`, 16..23 in
/// `out[2]`, 24..31 in `out[3]`, and 32..35 in the LOW nibble of
/// `out[4]` (with `out[4]`'s high 4 bits always zero).
#[inline]
pub fn pack_word(word: u64) -> [u8; BYTES_PER_WORD] {
    let w = word & WORD_MASK;
    [
        (w & 0xFF) as u8,
        ((w >> 8) & 0xFF) as u8,
        ((w >> 16) & 0xFF) as u8,
        ((w >> 24) & 0xFF) as u8,
        ((w >> 32) & 0x0F) as u8, // top 4 bits — bits 32..35; bits 36+ already masked off
    ]
}

/// Pre-computed packing of the canonical `HTR 0` halt sentinel.
///
/// Every emitted function ends with these 5 bytes — same role as
/// `HLT` for the Intel 8008 backend or the GE-225 `HALT_WORD`.
pub const HTR_HALT_BYTES: [u8; BYTES_PER_WORD] = [0x00, 0x00, 0x00, 0x80, 0x08];
