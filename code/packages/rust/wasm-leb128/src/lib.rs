//! # wasm-leb128
//!
//! LEB128 (Little-Endian Base-128) variable-length integer encoding for the
//! WebAssembly binary format.
//!
//! ## What is LEB128?
//!
//! Imagine you need to store the number 3 in a binary file. You *could* always
//! use 8 bytes (a u64), but that wastes 7 bytes when the value is small. LEB128
//! is a compression trick: pack 7 bits of data into each byte, and use the
//! **high bit** (bit 7) as a "more bytes follow" flag.
//!
//! ```text
//! Byte layout:
//!   bit 7  (MSB): continuation flag — 1 means "more bytes follow"
//!   bits 0–6    : 7 bits of actual data
//! ```
//!
//! Small numbers fit in one byte; large numbers use more bytes. Most integers
//! in a WASM module are small (function counts, local counts, instruction
//! immediates), so LEB128 keeps the binary format compact.
//!
//! ## Unsigned vs Signed
//!
//! **Unsigned LEB128** stores non-negative integers. The 7-bit groups are just
//! concatenated from least-significant to most-significant.
//!
//! **Signed LEB128** stores integers that may be negative. It uses two's
//! complement representation. When the last byte's high *data* bit (bit 6) is
//! set and no more bytes follow, the value is sign-extended to fill the full
//! integer width.
//!
//! ## Encoding Example: 624485 (unsigned)
//!
//! ```text
//! 624485 in binary: 0010_0110_0001_0000_0110_0101
//! Split into 7-bit groups (LSB first):
//!   group 0: 110_0101  → 0x65  → set continuation: 0xE5
//!   group 1: 000_1000  → 0x08  → set continuation: 0x88
//!   group 2: 010_0110  → 0x26  → last byte, no continuation
//! Result: [0xE5, 0x88, 0x26]
//! ```
//!
//! ## WASM Context
//!
//! Every integer in a WASM binary file (section lengths, function counts, local
//! variable counts, branch depths, instruction immediates…) is encoded in
//! LEB128. This crate provides the primitives needed by a WASM parser.
//!
//! This crate is part of the coding-adventures monorepo — a ground-up
//! implementation of the computing stack from transistors to operating systems.

use std::fmt;

// ─── Error Type ──────────────────────────────────────────────────────────────

/// An error produced during LEB128 encoding or decoding.
///
/// The two most common errors are:
/// - **Unterminated sequence**: the input ends while the continuation flag is
///   still set. The decoder cannot finish without more bytes.
/// - **Offset out of bounds**: the caller asked us to start decoding at a
///   position that is past the end of the input slice.
///
/// # Example
///
/// ```rust
/// use wasm_leb128::{decode_unsigned, Leb128Error};
///
/// // [0x80] has the continuation flag set but no following byte.
/// let result = decode_unsigned(&[0x80], 0);
/// assert!(result.is_err());
/// let err = result.unwrap_err();
/// assert_eq!(err.offset, 0);
/// ```
#[derive(Debug, PartialEq)]
pub struct Leb128Error {
    /// Human-readable description of what went wrong.
    pub message: String,
    /// The byte offset in the input where the error was detected.
    pub offset: usize,
}

impl fmt::Display for Leb128Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LEB128 error at offset {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Leb128Error {}

// ─── Decoding ────────────────────────────────────────────────────────────────

/// Decode an **unsigned** LEB128 integer from `data` starting at `offset`.
///
/// Returns `(value, bytes_consumed)` on success, where `bytes_consumed` is the
/// number of bytes read from `data[offset..]`.
///
/// ## Algorithm
///
/// We loop byte-by-byte. For each byte:
///
/// 1. Extract the 7 data bits: `byte & 0x7F`.
/// 2. Shift them into position: `(bits as u64) << shift` where `shift` starts
///    at 0 and increments by 7 each iteration.
/// 3. OR the shifted bits into the accumulator.
/// 4. Check the continuation flag: `byte & 0x80`. If it's zero, we're done.
///    If it's one, move to the next byte.
///
/// ## Visual Trace for [0xE5, 0x8E, 0x26]
///
/// ```text
/// byte=0xE5 (1110_0101): data=110_0101, shift=0  → acc = 0x65
/// byte=0x8E (1000_1110): data=000_1110, shift=7  → acc = 0x65 | (0x0E << 7) = 0x765
/// byte=0x26 (0010_0110): data=010_0110, shift=14 → acc = 0x765 | (0x26 << 14) = 624485
/// ```
///
/// ## Errors
///
/// - `offset >= data.len()`: offset is out of bounds.
/// - The sequence ends (data runs out) while the continuation flag is still
///   set: the encoding is incomplete.
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::decode_unsigned;
///
/// // Single-byte: 3
/// assert_eq!(decode_unsigned(&[0x03], 0).unwrap(), (3, 1));
///
/// // Multi-byte: 624485
/// assert_eq!(decode_unsigned(&[0xE5, 0x8E, 0x26], 0).unwrap(), (624485, 3));
///
/// // With offset — skip 2 bytes then decode
/// let buf = [0x00, 0x00, 0xE5, 0x8E, 0x26];
/// assert_eq!(decode_unsigned(&buf, 2).unwrap(), (624485, 3));
/// ```
pub fn decode_unsigned(data: &[u8], offset: usize) -> Result<(u64, usize), Leb128Error> {
    decode_unsigned_bounded(data, offset, 64)
}

/// Decode an **unsigned** LEB128 integer, but bounded to `max_bits`
/// significant bits — the width-aware sibling of [`decode_unsigned`], and the
/// primitive every WASM `uN` field (`u32` section sizes/counts/indices, a
/// `u64` memory64 limit, …) should really be decoded through.
///
/// ## Why `decode_unsigned` alone isn't enough
///
/// [`decode_unsigned`] happily decodes into a full `u64` — reasonable for a
/// truly 64-bit field, but a caller that needs a `u32` (the overwhelming
/// majority: every count, index, and length in the WASM binary format) would
/// otherwise have to narrow the result itself with something like
/// `value as u32`, which **silently discards the high bits** instead of
/// rejecting a value that doesn't actually fit. That was a real bug in this
/// crate's own consumer (`wasm-module-parser`'s `read_u32leb`) before this
/// function existed: a 5-byte-or-fewer LEB128 encoding of, say, `2^32`
/// (one bit too many for a `u32`) decoded successfully and then silently
/// wrapped to `0` on the `as u32` cast.
///
/// The WASM spec's own binary grammar (`webassembly.github.io/spec/core/
/// binary/values.html#binary-int`) defines `uN` recursively over the bit
/// width `N`, which bakes in two rules that a width-*less* decoder cannot
/// enforce:
///
/// 1. **Overlong**: at most `ceil(N / 7)` bytes are allowed. A continuation
///    flag on the last byte the width permits means the encoding is asking
///    for more precision than the type has — malformed ("integer
///    representation too long" in the spec's own wording).
/// 2. **Out of range**: on the final byte, any data bits *above* position
///    `N` must be zero — a nonzero one encodes a value that doesn't fit in
///    `N` bits at all, even though the byte *count* was within budget
///    ("integer too large").
///
/// Non-minimal (but in-budget) encodings are explicitly **not** an error —
/// the WASM spec permits padding a small value out to more bytes than
/// strictly necessary, as long as rule 1's byte cap and rule 2's padding-bits
/// rule both hold. `[0x82, 0x80, 0x80, 0x80, 0x00]` (5 bytes for the value 2,
/// the max allowed for `max_bits = 32`) is perfectly legal; a 6th byte would
/// not be.
///
/// ## Worked example: `max_bits = 32`, rejecting `2^32`
///
/// ```text
/// bytes: [0x80, 0x80, 0x80, 0x80, 0x10]     (5 bytes — within the u32 budget)
/// byte 0..3: data=0, shift=0,7,14,21        → contributes nothing
/// byte 4 (the LAST allowed byte, shift=28): data = 0x10 = 0b0010000
///   valid_bits = 32 - 28 = 4   (only the low 4 data bits are "in range")
///   extra = data >> 4 = 0b001 = 1           ← nonzero!
///   → "integer too large": this byte's bit 4 represents value-bit 32,
///     one bit past what a u32 can hold.
/// ```
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::decode_unsigned_bounded;
///
/// // Non-minimal but in-budget: fine.
/// assert_eq!(
///     decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x00], 0, 32).unwrap(),
///     (2, 5)
/// );
///
/// // One byte past the u32 budget: "integer representation too long".
/// assert!(decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x80, 0x00], 0, 32).is_err());
///
/// // In-budget byte count, but the value is 2^32 (one bit too many): "integer too large".
/// assert!(decode_unsigned_bounded(&[0x80, 0x80, 0x80, 0x80, 0x10], 0, 32).is_err());
/// ```
pub fn decode_unsigned_bounded(data: &[u8], offset: usize, max_bits: u32) -> Result<(u64, usize), Leb128Error> {
    let (value, consumed, _sign_bit) = decode_bits_core(data, offset, max_bits, false)?;
    Ok((value, consumed))
}

/// Decode a **signed** LEB128 integer from `data` starting at `offset`.
///
/// Returns `(value, bytes_consumed)` on success.
///
/// ## Signed vs Unsigned Decoding
///
/// The loop is identical to unsigned decoding. The difference is the final
/// step: **sign extension**. After we stop reading bytes, we check whether the
/// last byte's highest *data* bit (bit 6, i.e. `last_byte & 0x40`) is set. If
/// it is, the original number was negative, and we must fill in all the
/// remaining high bits with 1s.
///
/// ```text
/// Sign extension for a 64-bit result:
///   If (last data bit of last byte) is 1 AND we haven't filled all 64 bits:
///     value |= !0u64 << shift   (turn on every bit above 'shift')
/// ```
///
/// ## Example: [0x7E] → -2
///
/// ```text
/// byte=0x7E (0111_1110): continuation=0, data=111_1110
/// value = 0x7E = 0b0111_1110 = 126 (unsigned)
/// shift = 7
/// last data bit = bit 6 of 0x7E = 1  → negative!
/// sign extend: value |= !0u64 << 7  = 0xFFFF_FFFF_FFFF_FF80
/// result as i64: 0xFFFF_FFFF_FFFF_FF80 = -128... wait, that's wrong.
///
/// Let me redo: data bits of 0x7E = 0x7E & 0x7F = 0x7E = 0b111_1110
/// sign extend: value |= !0u64 << 7
///   value = 0b111_1110 | 0xFFFF_FFFF_FFFF_FF80
///          = 0xFFFF_FFFF_FFFF_FFFE
/// as i64 = -2 ✓
/// ```
///
/// ## Errors
///
/// Same conditions as [`decode_unsigned`].
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::decode_signed;
///
/// // -2 encoded as a single byte
/// assert_eq!(decode_signed(&[0x7E], 0).unwrap(), (-2, 1));
///
/// // min i32 = -2147483648
/// assert_eq!(
///     decode_signed(&[0x80, 0x80, 0x80, 0x80, 0x78], 0).unwrap(),
///     (-2147483648, 5)
/// );
/// ```
pub fn decode_signed(data: &[u8], offset: usize) -> Result<(i64, usize), Leb128Error> {
    decode_signed_bounded(data, offset, 64)
}

/// Decode a **signed** LEB128 integer, bounded to `max_bits` significant
/// bits — the width-aware sibling of [`decode_signed`], analogous to
/// [`decode_unsigned_bounded`] but with two's-complement sign extension
/// instead of zero-fill.
///
/// ## The two padding rules, signed edition
///
/// Same two rules as [`decode_unsigned_bounded`] (byte-count cap, padding
/// bits above `max_bits` must be consistent), except rule 2 is now about
/// *sign* consistency rather than "must be zero": the bits above `max_bits`
/// in the terminal byte must all equal the value's own sign bit (bit
/// `max_bits - 1`) — i.e. the encoding must look like a properly
/// sign-extended `max_bits`-wide two's-complement number, not an arbitrary
/// wider one that happens to share the low bits.
///
/// ```text
/// bytes: [0x80, 0x80, 0x80, 0x80, 0x70]     (5 bytes — i32 budget)
/// byte 4 (shift=28): data = 0x70 = 0b1110000
///   valid_bits = 32 - 28 = 4
///   sign bit = bit (valid_bits - 1) = bit 3 of data = 0        → "positive"
///   extra = data >> 4 = 0b111 = 7
///   expected (sign=0) = 0                    ← 7 ≠ 0, mismatch!
///   → "integer too large": bits 32-34 don't repeat the (positive) sign bit.
/// ```
///
/// The returned `i64` is always the *fully* sign-extended 64-bit value, not
/// truncated to `max_bits` — exactly like [`decode_signed`]'s existing
/// contract, so `decode_signed_bounded(&[0x7F], 0, 7)` and
/// `decode_signed(&[0x7F], 0)` agree: both return `-1`, not some
/// 7-bit-truncated bit pattern.
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::decode_signed_bounded;
///
/// // i32.const -1, minimal encoding: fine, fully sign-extended to i64.
/// assert_eq!(decode_signed_bounded(&[0x7F], 0, 32).unwrap(), (-1, 1));
///
/// // i32.const -1, non-minimal but in-budget (5 bytes): still fine.
/// assert_eq!(
///     decode_signed_bounded(&[0xFF, 0xFF, 0xFF, 0xFF, 0x7F], 0, 32).unwrap(),
///     (-1, 5)
/// );
///
/// // One byte past the i32 budget (6 bytes): "integer representation too long".
/// assert!(decode_signed_bounded(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F], 0, 32).is_err());
///
/// // In-budget byte count, but padding bits don't match the sign: "integer too large".
/// assert!(decode_signed_bounded(&[0x80, 0x80, 0x80, 0x80, 0x70], 0, 32).is_err());
/// ```
pub fn decode_signed_bounded(data: &[u8], offset: usize, max_bits: u32) -> Result<(i64, usize), Leb128Error> {
    let (mut value, consumed, sign_bit) = decode_bits_core(data, offset, max_bits, true)?;
    if let Some(extend_from) = sign_bit {
        if extend_from < 64 {
            value |= !0u64 << extend_from;
        }
    }
    Ok((value as i64, consumed))
}

/// Shared decoding core for [`decode_unsigned_bounded`] and
/// [`decode_signed_bounded`] (and, via `max_bits = 64`, for the original
/// unbounded [`decode_unsigned`]/[`decode_signed`] too — a `u64`/`i64` is
/// itself just a 64-bit-wide `uN`/`sN`, so the SAME padding-bits rule
/// applies at the 10th byte, which is exactly the "zero-extend"/
/// "sign-extend" `assert_malformed` cases the real corpus's own
/// `binary_leb128_64.wast` and `binary-leb128.wast` test).
///
/// Reads 7-bit continuation-flagged groups into a raw accumulator, enforcing
/// the `ceil(max_bits / 7)`-byte cap along the way, then — on the terminal
/// byte — validates that any bits beyond `max_bits` are correctly
/// zero-filled (`signed = false`) or sign-extended (`signed = true`).
///
/// Returns `(value, bytes_consumed, sign_info)`:
/// - `value`: the raw accumulated bits (low `max_bits` of it meaningful;
///   `decode_unsigned_bounded` returns this as-is, `decode_signed_bounded`
///   still needs to fill in the bits *above* `sign_info` itself).
/// - `sign_info`: `None` when `signed = false`; when `signed = true`, `None`
///   if the value's sign bit was 0 (no extension needed — `value`'s upper
///   bits are already correctly zero) or `Some(bit_position)` giving the bit
///   position from which the caller should OR in `!0u64 << bit_position` to
///   finish sign-extending to a full 64 bits.
fn decode_bits_core(data: &[u8], offset: usize, max_bits: u32, signed: bool) -> Result<(u64, usize, Option<u32>), Leb128Error> {
    if offset >= data.len() {
        return Err(Leb128Error {
            message: format!(
                "offset {} is out of bounds for data of length {}",
                offset,
                data.len()
            ),
            offset,
        });
    }

    // ceil(max_bits / 7): the most bytes a `max_bits`-wide LEB128 value is
    // ever allowed to use. E.g. 32 -> 5, 64 -> 10, 7 -> 1.
    let max_bytes = max_bits.div_ceil(7);

    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut bytes_consumed: u32 = 0;

    for &byte in &data[offset..] {
        bytes_consumed += 1;
        let data_bits = (byte & 0x7F) as u64;
        let continues = byte & 0x80 != 0;

        if continues {
            // A continuation flag on the very last byte the width permits
            // means the encoding wants MORE bytes than `max_bits` allows --
            // malformed, regardless of what value those extra bytes would
            // have contributed.
            if bytes_consumed >= max_bytes {
                return Err(Leb128Error {
                    message: format!(
                        "integer representation too long: LEB128 sequence uses more than {max_bytes} bytes for a {max_bits}-bit value"
                    ),
                    offset,
                });
            }
            // Not yet at the width boundary, so all 7 data bits are always
            // "in range" here -- safe to fold in directly.
            value |= data_bits << shift;
            shift += 7;
            continue;
        }

        // Terminal byte. How many of ITS 7 data bits actually fall within
        // `max_bits`? Anywhere short of the width boundary this is the full
        // 7 (nothing to check); AT the boundary (only possible on the last
        // allowed byte, by construction of `max_bytes` as a ceiling) it's
        // fewer, and the bits above that must be zero-filled or
        // sign-extended correctly.
        let valid_bits = max_bits.saturating_sub(shift).min(7);

        if valid_bits == 7 {
            // No width restriction applies to this byte.
            value |= data_bits << shift;
            let sign_info = if signed && (byte & 0x40) != 0 { Some(shift + 7) } else { None };
            return Ok((value, bytes_consumed as usize, sign_info));
        }

        // At the width boundary: `valid_bits` (0..=6) of this byte's data
        // bits are meaningful; the rest are padding that must agree with
        // what a correctly-truncated `max_bits`-wide value would produce.
        let extra = data_bits >> valid_bits;
        let sign_bit_set = signed && valid_bits > 0 && (data_bits >> (valid_bits - 1)) & 1 != 0;
        let expected_extra = if sign_bit_set { 0x7Fu64 >> valid_bits } else { 0 };

        if extra != expected_extra {
            return Err(Leb128Error {
                message: format!(
                    "integer too large: does not fit in {max_bits} bits (padding bits at offset {} don't {} extend correctly)",
                    offset + bytes_consumed as usize - 1,
                    if signed { "sign" } else { "zero" }
                ),
                offset,
            });
        }

        let mask = if valid_bits == 0 { 0 } else { (1u64 << valid_bits) - 1 };
        value |= (data_bits & mask) << shift;
        let sign_info = if sign_bit_set { Some(max_bits) } else { None };
        return Ok((value, bytes_consumed as usize, sign_info));
    }

    Err(Leb128Error {
        message: "unexpected end of data: LEB128 sequence is unterminated".to_string(),
        offset,
    })
}

// ─── Encoding ────────────────────────────────────────────────────────────────

/// Encode an **unsigned** 64-bit integer as LEB128.
///
/// Returns a `Vec<u8>` containing the encoded bytes, always at least 1 byte
/// long (0 encodes as `[0x00]`).
///
/// ## Algorithm
///
/// Loop:
/// 1. Take the low 7 bits of the value: `byte = value & 0x7F`.
/// 2. Shift the value right by 7: `value >>= 7`.
/// 3. If `value != 0`, set the continuation flag: `byte |= 0x80`.
/// 4. Push `byte` to output.
/// 5. Repeat until `value == 0`.
///
/// ## Visual Trace for 624485
///
/// ```text
/// 624485 = 0b10011000011101100101
/// iteration 1: byte = 0b1100101 = 0x65, value >>= 7 → 4878, set flag → 0xE5
/// iteration 2: byte = 0b0001110 = 0x0E, value >>= 7 → 38,   set flag → 0x8E
/// iteration 3: byte = 0b0100110 = 0x26, value >>= 7 → 0,    no flag  → 0x26
/// result: [0xE5, 0x8E, 0x26]
/// ```
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::encode_unsigned;
///
/// assert_eq!(encode_unsigned(0), vec![0x00]);
/// assert_eq!(encode_unsigned(3), vec![0x03]);
/// assert_eq!(encode_unsigned(624485), vec![0xE5, 0x8E, 0x26]);
/// ```
pub fn encode_unsigned(mut value: u64) -> Vec<u8> {
    let mut result = Vec::new();

    loop {
        // Grab the lowest 7 bits.
        let mut byte = (value & 0x7F) as u8;
        // Shift those bits out of the value.
        value >>= 7;

        if value != 0 {
            // More bytes will follow — set the continuation flag.
            byte |= 0x80;
        }

        result.push(byte);

        if value == 0 {
            break;
        }
    }

    result
}

/// Encode a **signed** 64-bit integer as LEB128.
///
/// Returns a `Vec<u8>`. Negative numbers are represented in two's complement
/// and sign-extended during encoding so that decoding recovers the original
/// value.
///
/// ## Algorithm
///
/// The loop is similar to unsigned encoding, but the termination condition is
/// more subtle. We stop when:
/// - The remaining value is 0 **and** the top data bit of the last byte
///   written is 0 (i.e., no spurious sign extension on decode), **or**
/// - The remaining value is -1 **and** the top data bit of the last byte
///   written is 1 (i.e., sign extension will fill in the 1s correctly).
///
/// ```text
/// Termination: value == 0 && (byte & 0x40) == 0   → positive, done
///              value == -1 && (byte & 0x40) != 0   → negative, done
/// ```
///
/// # Examples
///
/// ```rust
/// use wasm_leb128::encode_signed;
///
/// assert_eq!(encode_signed(0), vec![0x00]);
/// assert_eq!(encode_signed(-2), vec![0x7E]);
/// assert_eq!(encode_signed(-2147483648), vec![0x80, 0x80, 0x80, 0x80, 0x78]);
/// ```
pub fn encode_signed(mut value: i64) -> Vec<u8> {
    let mut result = Vec::new();

    loop {
        // Take the low 7 bits (treating as unsigned for the byte).
        let mut byte = (value & 0x7F) as u8;
        // Arithmetic right shift — propagates the sign bit.
        value >>= 7;

        // Check whether we are done after this byte.
        // We are done if no more meaningful bits remain:
        //   - positive numbers: value is 0 and bit 6 of byte is clear
        //     (so decode won't sign-extend)
        //   - negative numbers: value is -1 and bit 6 of byte is set
        //     (so decode will sign-extend to fill in the 1s)
        let done = (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0);

        if !done {
            // More bytes follow — set continuation flag.
            byte |= 0x80;
        }

        result.push(byte);

        if done {
            break;
        }
    }

    result
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unsigned Decoding ───────────────────────────────────────────────────

    #[test]
    fn decode_unsigned_zero() {
        // Test case 1: Zero should decode to 0 and consume exactly 1 byte.
        let result = decode_unsigned(&[0x00], 0).unwrap();
        assert_eq!(result, (0, 1), "zero should decode to (0, 1)");
    }

    #[test]
    fn decode_unsigned_one_byte() {
        // Test case 2: Small value 3 fits in a single byte.
        let result = decode_unsigned(&[0x03], 0).unwrap();
        assert_eq!(result, (3, 1));
    }

    #[test]
    fn decode_unsigned_multi_byte() {
        // Test case 4: 624485 encoded as [0xE5, 0x8E, 0x26], three bytes.
        // Note: 0x88 (from some older references) is incorrect — it decodes to
        // 623717 not 624485. The correct second byte is 0x8E.
        let result = decode_unsigned(&[0xE5, 0x8E, 0x26], 0).unwrap();
        assert_eq!(result, (624485, 3));
    }

    #[test]
    fn decode_unsigned_max_u32() {
        // Test case 5: Maximum 32-bit unsigned value 4294967295 (0xFFFFFFFF).
        // In LEB128 this requires 5 bytes.
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        let result = decode_unsigned(&data, 0).unwrap();
        assert_eq!(result, (4294967295, 5));
    }

    #[test]
    fn decode_unsigned_with_offset() {
        // Test case 10: Non-zero offset — skip two garbage bytes and decode.
        let buf = [0x00, 0x00, 0xE5, 0x8E, 0x26];
        let result = decode_unsigned(&buf, 2).unwrap();
        assert_eq!(result, (624485, 3));
    }

    #[test]
    fn decode_unsigned_unterminated() {
        // Test case 9: Both bytes have continuation flag set — no terminator.
        let result = decode_unsigned(&[0x80, 0x80], 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.offset, 0);
        println!("unterminated error: {}", err);
    }

    #[test]
    fn decode_unsigned_offset_out_of_bounds() {
        // Offset past end of slice must return an error.
        let result = decode_unsigned(&[0x01], 5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.offset, 5);
    }

    // ── Signed Decoding ─────────────────────────────────────────────────────

    #[test]
    fn decode_signed_zero() {
        // Test case 1 (signed): Zero.
        let result = decode_signed(&[0x00], 0).unwrap();
        assert_eq!(result, (0, 1));
    }

    #[test]
    fn decode_signed_one_byte_negative() {
        // Test case 3: 0x7E is -2 in signed LEB128.
        // 0x7E = 0b0111_1110, data bits = 0b111_1110 = 0x7E
        // bit 6 is set → sign extend → result = -2
        let result = decode_signed(&[0x7E], 0).unwrap();
        assert_eq!(result, (-2, 1));
    }

    #[test]
    fn decode_signed_max_i32() {
        // Test case 6: Maximum 32-bit signed value 2147483647.
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0x07];
        let result = decode_signed(&data, 0).unwrap();
        assert_eq!(result, (2147483647, 5));
    }

    #[test]
    fn decode_signed_min_i32() {
        // Test case 7: Minimum 32-bit signed value -2147483648.
        let data = [0x80, 0x80, 0x80, 0x80, 0x78];
        let result = decode_signed(&data, 0).unwrap();
        assert_eq!(result, (-2147483648, 5));
    }

    #[test]
    fn decode_signed_unterminated() {
        // Test case 9 (signed): Unterminated sequence.
        let result = decode_signed(&[0x80, 0x80], 0);
        assert!(result.is_err());
    }

    #[test]
    fn decode_signed_with_offset() {
        // Test case 10 (signed): Non-zero offset.
        // Place 0x7E (= -2) at offset 3.
        let buf = [0x00, 0x00, 0x00, 0x7E];
        let result = decode_signed(&buf, 3).unwrap();
        assert_eq!(result, (-2, 1));
    }

    // ── Unsigned Encoding ───────────────────────────────────────────────────

    #[test]
    fn encode_unsigned_zero() {
        assert_eq!(encode_unsigned(0), vec![0x00]);
    }

    #[test]
    fn encode_unsigned_one_byte() {
        assert_eq!(encode_unsigned(3), vec![0x03]);
    }

    #[test]
    fn encode_unsigned_multi_byte() {
        assert_eq!(encode_unsigned(624485), vec![0xE5, 0x8E, 0x26]);
    }

    #[test]
    fn encode_unsigned_max_u32() {
        assert_eq!(
            encode_unsigned(4294967295),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0x0F]
        );
    }

    // ── Signed Encoding ─────────────────────────────────────────────────────

    #[test]
    fn encode_signed_zero() {
        assert_eq!(encode_signed(0), vec![0x00]);
    }

    #[test]
    fn encode_signed_negative_two() {
        assert_eq!(encode_signed(-2), vec![0x7E]);
    }

    #[test]
    fn encode_signed_min_i32() {
        assert_eq!(
            encode_signed(-2147483648),
            vec![0x80, 0x80, 0x80, 0x80, 0x78]
        );
    }

    #[test]
    fn encode_signed_max_i32() {
        assert_eq!(
            encode_signed(2147483647),
            vec![0xFF, 0xFF, 0xFF, 0xFF, 0x07]
        );
    }

    // ── Round-Trips ─────────────────────────────────────────────────────────

    #[test]
    fn round_trip_unsigned() {
        // Test case 8: encode then decode should return the original value.
        let values: &[u64] = &[
            0,
            1,
            127,
            128,
            255,
            624485,
            4294967295,
            u64::MAX,
        ];
        for &v in values {
            let encoded = encode_unsigned(v);
            let (decoded, consumed) = decode_unsigned(&encoded, 0).unwrap();
            assert_eq!(
                decoded, v,
                "round-trip failed for unsigned {}",
                v
            );
            assert_eq!(
                consumed,
                encoded.len(),
                "bytes_consumed mismatch for {}",
                v
            );
        }
    }

    #[test]
    fn round_trip_signed() {
        // Test case 11: signed negative round-trips.
        let values: &[i64] = &[
            0,
            1,
            -1,
            -2,
            63,
            -64,
            127,
            -128,
            2147483647,
            -2147483648,
            i64::MAX,
            i64::MIN,
        ];
        for &v in values {
            let encoded = encode_signed(v);
            let (decoded, consumed) = decode_signed(&encoded, 0).unwrap();
            assert_eq!(
                decoded, v,
                "round-trip failed for signed {}",
                v
            );
            assert_eq!(
                consumed,
                encoded.len(),
                "bytes_consumed mismatch for signed {}",
                v
            );
        }
    }

    // ── Bounded Decoding: the malformed-encoding classes the real WASM
    //    corpus's `binary-leb128.wast`/`binary_leb128_64.wast` files exist
    //    to exercise. Each class gets its own test rather than relying only
    //    on corpus coverage, per this crate's own testing standard.
    // ─────────────────────────────────────────────────────────────────────

    /// Class 1: **non-minimal but in-budget** encodings are legal, not
    /// malformed -- the WASM spec explicitly permits padding a small value
    /// out to more bytes than strictly necessary, as long as the byte count
    /// stays within `ceil(max_bits / 7)`.
    #[test]
    fn bounded_unsigned_non_minimal_is_not_malformed() {
        // Value 2, padded out to the full 5-byte budget for a 32-bit field.
        let (value, consumed) = decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x00], 0, 32).unwrap();
        assert_eq!((value, consumed), (2, 5));
    }

    /// Class 2: **overlong** -- a continuation flag on the byte the width's
    /// budget says must be the last one. `ceil(32/7) = 5` bytes is the
    /// budget for a 32-bit field; a 6th byte (even one that would decode to
    /// a perfectly reasonable value) is malformed.
    #[test]
    fn bounded_unsigned_overlong_is_rejected() {
        let err = decode_unsigned_bounded(&[0x82, 0x80, 0x80, 0x80, 0x80, 0x00], 0, 32).unwrap_err();
        assert!(err.message.contains("too long"), "unexpected message: {}", err.message);
    }

    #[test]
    fn bounded_signed_overlong_is_rejected() {
        // i32.const -1, padded to 6 bytes -- one past the 5-byte i32 budget.
        let err = decode_signed_bounded(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F], 0, 32).unwrap_err();
        assert!(err.message.contains("too long"), "unexpected message: {}", err.message);
    }

    /// Class 3: **out of range** (unsigned) -- byte count is within budget,
    /// but the value's padding bits are nonzero, meaning it doesn't actually
    /// fit in `max_bits`. `[0x80, 0x80, 0x80, 0x80, 0x10]` decodes as
    /// `2^32` under a width-less reader: one bit past what a `u32` can hold.
    #[test]
    fn bounded_unsigned_out_of_range_is_rejected() {
        let err = decode_unsigned_bounded(&[0x80, 0x80, 0x80, 0x80, 0x10], 0, 32).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected message: {}", err.message);
    }

    /// The same class, but for a genuinely 64-bit field -- the ORIGINAL
    /// bug this crate had before `decode_bits_core` existed: `decode_unsigned`
    /// stored the accumulator natively in a `u64`, so an out-of-range 10-byte
    /// encoding (offset `2^64`, one bit past `u64::MAX`) simply had its
    /// overflow bit silently shifted away instead of being rejected. This is
    /// exactly `binary_leb128_64.wast`'s own `assert_malformed` case (a
    /// memarg offset of `2^64` against a memory64 instruction).
    #[test]
    fn decode_unsigned_rejects_value_one_bit_past_u64_max() {
        // 2^64 - 1 (u64::MAX) is fine: all 64 bits legitimately used.
        let ok = decode_unsigned(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01], 0);
        assert_eq!(ok.unwrap().0, u64::MAX);

        // 2^64 (one unused bit set past bit 63): must be rejected, not
        // silently truncated back down to some smaller wrapped value.
        let err = decode_unsigned(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02], 0).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected message: {}", err.message);
    }

    /// Signed edition of the same 64-bit boundary bug: `i64.const -1` with
    /// unused high bits deliberately left UNSET (not properly sign-extended)
    /// must be rejected -- it doesn't round-trip as a correctly-encoded
    /// 64-bit two's-complement value even though every individual byte is
    /// well-formed LEB128.
    #[test]
    fn decode_signed_rejects_inconsistent_sign_extension_past_i64() {
        // i64.const -1, properly sign-extended through all 10 bytes: fine.
        let ok = decode_signed(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f], 0);
        assert_eq!(ok.unwrap().0, -1);

        // Same low bits, but the 10th byte's padding doesn't match the sign
        // bit it itself carries (bit0=1 => negative, yet bits1-6 are 0
        // instead of the required all-1s).
        let err = decode_signed(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01], 0).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected message: {}", err.message);
    }

    /// Class 3, signed edition, at a 32-bit width: padding bits must repeat
    /// the sign bit, not just be zero. `i32.const 0` encoded with the last
    /// byte's padding bits set to 1 (looks like it should sign-extend to a
    /// huge negative number, inconsistent with the low bits all being 0).
    #[test]
    fn bounded_signed_out_of_range_positive_with_negative_padding() {
        let err = decode_signed_bounded(&[0x80, 0x80, 0x80, 0x80, 0x70], 0, 32).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected message: {}", err.message);
    }

    /// ...and the mirror image: a value whose low bits look negative
    /// (`i32.const -1`) but whose padding bits are 0 instead of sign-extended
    /// 1s -- the exact "unused bits unset" corpus phrasing.
    #[test]
    fn bounded_signed_out_of_range_negative_with_positive_padding() {
        let err = decode_signed_bounded(&[0xff, 0xff, 0xff, 0xff, 0x0f], 0, 32).unwrap_err();
        assert!(err.message.contains("too large"), "unexpected message: {}", err.message);
    }

    /// Class 4: **truncated stream** -- a continuation flag set on the very
    /// last byte actually present in the input, with nothing left to read.
    /// Already covered for the unbounded decoders by
    /// `decode_unsigned_unterminated`/`decode_signed_unterminated` above;
    /// confirmed here to behave identically through the bounded entry point
    /// (same underlying `decode_bits_core`).
    #[test]
    fn bounded_decode_truncated_stream_is_rejected() {
        let err = decode_unsigned_bounded(&[0x80, 0x80], 0, 32).unwrap_err();
        assert!(err.message.contains("unterminated"), "unexpected message: {}", err.message);
    }

    /// A single-byte (`max_bits = 7`) field: the narrowest possible width,
    /// and a useful edge case since `max_bytes = ceil(7/7) = 1` means ANY
    /// continuation byte at all is already "too long".
    #[test]
    fn bounded_unsigned_seven_bit_width_rejects_any_continuation() {
        assert_eq!(decode_unsigned_bounded(&[0x7F], 0, 7).unwrap(), (127, 1));
        let err = decode_unsigned_bounded(&[0x80, 0x00], 0, 7).unwrap_err();
        assert!(err.message.contains("too long"), "unexpected message: {}", err.message);
    }

    /// `decode_signed_bounded` must still return a value fully sign-extended
    /// to a 64-bit `i64`, matching `decode_signed`'s own contract -- a
    /// caller bounding to `max_bits = 7` shouldn't get back some
    /// 7-bit-truncated bit pattern instead of the true mathematical value.
    #[test]
    fn bounded_signed_returns_fully_sign_extended_i64() {
        assert_eq!(decode_signed_bounded(&[0x7F], 0, 7).unwrap(), (-1, 1));
        assert_eq!(decode_signed_bounded(&[0x3F], 0, 7).unwrap(), (63, 1));
        assert_eq!(decode_signed_bounded(&[0x40], 0, 7).unwrap(), (-64, 1));
    }

    // ── Error Display ───────────────────────────────────────────────────────

    #[test]
    fn error_display() {
        let err = Leb128Error {
            message: "test error".to_string(),
            offset: 42,
        };
        let s = format!("{}", err);
        assert!(s.contains("42"));
        assert!(s.contains("test error"));
        println!("{}", err);
    }

    #[test]
    fn error_debug() {
        let err = Leb128Error {
            message: "debug test".to_string(),
            offset: 7,
        };
        let s = format!("{:?}", err);
        assert!(s.contains("debug test"));
    }

    #[test]
    fn error_equality() {
        let a = Leb128Error {
            message: "msg".to_string(),
            offset: 1,
        };
        let b = Leb128Error {
            message: "msg".to_string(),
            offset: 1,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(Leb128Error {
            message: "boxed".to_string(),
            offset: 0,
        });
        assert!(err.to_string().contains("boxed"));
    }
}
