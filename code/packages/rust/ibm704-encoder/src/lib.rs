//! Canonical IBM 704 instruction-word encoder and transport.
//!
//! The 704 uses 36-bit words and IBM's left-to-right bit numbering (`S`, then
//! 1 through 35). This crate constructs the historical Type A and Type B
//! layouts and transports every word as five big-endian bytes with a reserved
//! zero high nibble.
//!
//! ```
//! use ibm704_encoder::{encode_cla, encode_htr, pack_word};
//!
//! assert_eq!(pack_word(encode_cla(2)), [0x01, 0x40, 0, 0, 2]);
//! assert_eq!(pack_word(encode_htr(0)), [0; 5]);
//! ```

use std::fmt;

/// `HTR Y` — Halt and Transfer (`+0000`).
pub const HTR: u16 = 0o000;

/// `HPR Y` — Halt and Proceed (`+0420`).
pub const HPR: u16 = 0o420;

/// `CLA Y` — Clear and Add (`+0500`).
pub const CLA: u16 = 0o500;

/// Number of bits in one IBM 704 word.
pub const WORD_BITS: u32 = 36;

/// Mask covering exactly one IBM 704 word.
pub const WORD_MASK: u64 = (1u64 << WORD_BITS) - 1;

/// Number of transport bytes per word.
pub const BYTES_PER_WORD: usize = 5;

/// Width of an address or decrement field.
pub const ADDR_BITS: u32 = 15;

/// Mask covering an address or decrement field.
pub const ADDR_MASK: u64 = (1u64 << ADDR_BITS) - 1;

/// Mask covering the nine-bit Type B operation magnitude.
pub const OPCODE_MASK: u64 = 0o777;

/// Mask covering the three-bit tag field.
pub const TAG_MASK: u64 = 0b111;

/// Raw bit position of the Type B operation magnitude.
pub const OPCODE_SHIFT: u32 = 24;

/// Raw bit position of the Type A decrement field.
pub const DECREMENT_SHIFT: u32 = 18;

/// Raw bit position of the tag field in both instruction types.
pub const TAG_SHIFT: u32 = 15;

/// Raw sign-bit mask (IBM position `S`).
pub const SIGN_BIT: u64 = 1 << 35;

/// Errors raised while constructing canonical instruction words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// A Type A prefix must fit in three bits and have IBM bits 1–2 non-zero.
    InvalidTypeAPrefix(u8),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTypeAPrefix(prefix) => write!(
                f,
                "IBM 704 Type A prefix must fit in three bits and have its low two bits non-zero, got {prefix:#05b}"
            ),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Errors raised while decoding canonical transport bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// A stream must contain complete five-byte words.
    InvalidLength(usize),
    /// The reserved high nibble in the first byte must be zero.
    ReservedNibble(u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(length) => write!(
                f,
                "IBM 704 byte stream length must be a multiple of {BYTES_PER_WORD}, got {length}"
            ),
            Self::ReservedNibble(byte) => write!(
                f,
                "IBM 704 word has non-zero reserved high nibble in first byte 0x{byte:02X}"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Encode a historical Type B instruction.
///
/// Raw layout: sign at bit 35, two required zero bits, nine-bit operation at
/// bits 32–24, six unused zero bits, tag at bits 17–15, address at bits 14–0.
#[inline]
pub const fn encode_type_b(negative: bool, opcode: u16, tag: u8, address: u16) -> u64 {
    let sign = if negative { SIGN_BIT } else { 0 };
    sign | (((opcode as u64) & OPCODE_MASK) << OPCODE_SHIFT)
        | (((tag as u64) & TAG_MASK) << TAG_SHIFT)
        | ((address as u64) & ADDR_MASK)
}

/// Encode a historical Type A instruction.
///
/// The prefix is IBM bits `S,1,2`; the decrement, tag, and address fields
/// occupy the remaining 33 bits. Prefixes wider than three bits, or whose IBM
/// bits 1–2 are both zero, return [`EncodeError::InvalidTypeAPrefix`] because
/// those words are architecturally Type B.
#[inline]
pub const fn encode_type_a(
    prefix: u8,
    decrement: u16,
    tag: u8,
    address: u16,
) -> Result<u64, EncodeError> {
    if prefix > TAG_MASK as u8 || prefix & 0b11 == 0 {
        return Err(EncodeError::InvalidTypeAPrefix(prefix));
    }
    Ok(((prefix as u64) << 33)
        | (((decrement as u64) & ADDR_MASK) << DECREMENT_SHIFT)
        | (((tag as u64) & TAG_MASK) << TAG_SHIFT)
        | ((address as u64) & ADDR_MASK))
}

/// Encode a positive, untagged Type B instruction.
#[inline]
pub const fn encode_instruction(opcode: u16, address: u16) -> u64 {
    encode_type_b(false, opcode, 0, address)
}

/// Encode `HTR Y`.
#[inline]
pub const fn encode_htr(address: u16) -> u64 {
    encode_instruction(HTR, address)
}

/// Encode `HPR Y`.
#[inline]
pub const fn encode_hpr(address: u16) -> u64 {
    encode_instruction(HPR, address)
}

/// Encode `CLA Y`.
#[inline]
pub const fn encode_cla(address: u16) -> u64 {
    encode_instruction(CLA, address)
}

/// Pack a 36-bit word into the canonical five-byte big-endian transport.
#[inline]
pub const fn pack_word(word: u64) -> [u8; BYTES_PER_WORD] {
    let word = word & WORD_MASK;
    [
        ((word >> 32) & 0x0F) as u8,
        ((word >> 24) & 0xFF) as u8,
        ((word >> 16) & 0xFF) as u8,
        ((word >> 8) & 0xFF) as u8,
        (word & 0xFF) as u8,
    ]
}

/// Decode one canonical five-byte word.
#[inline]
pub fn unpack_word(bytes: [u8; BYTES_PER_WORD]) -> Result<u64, DecodeError> {
    if bytes[0] & 0xF0 != 0 {
        return Err(DecodeError::ReservedNibble(bytes[0]));
    }
    Ok(((bytes[0] as u64) << 32)
        | ((bytes[1] as u64) << 24)
        | ((bytes[2] as u64) << 16)
        | ((bytes[3] as u64) << 8)
        | bytes[4] as u64)
}

/// Decode a stream of canonical five-byte words.
pub fn unpack_words(program: &[u8]) -> Result<Vec<u64>, DecodeError> {
    if !program.len().is_multiple_of(BYTES_PER_WORD) {
        return Err(DecodeError::InvalidLength(program.len()));
    }

    program
        .as_chunks::<BYTES_PER_WORD>()
        .0
        .iter()
        .copied()
        .map(unpack_word)
        .collect()
}

/// Canonical `HTR 0` transport bytes.
pub const HTR_HALT_BYTES: [u8; BYTES_PER_WORD] = pack_word(encode_htr(0));
