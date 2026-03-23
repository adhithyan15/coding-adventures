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

    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut bytes_consumed: usize = 0;

    for &byte in &data[offset..] {
        // The low 7 bits carry data; shift them into place.
        let data_bits = (byte & 0x7F) as u64;
        value |= data_bits << shift;
        bytes_consumed += 1;
        shift += 7;

        // High bit is the continuation flag.
        if byte & 0x80 == 0 {
            // Flag is clear → this was the last byte.
            return Ok((value, bytes_consumed));
        }

        // Guard against absurdly large encodings (u64 can hold at most 10
        // bytes of LEB128 data).
        if shift >= 70 {
            return Err(Leb128Error {
                message: "LEB128 sequence exceeds maximum u64 width (70 bits)".to_string(),
                offset,
            });
        }
    }

    // If we get here, we ran out of bytes while continuation flag was still
    // set — the sequence is unterminated.
    Err(Leb128Error {
        message: "unexpected end of data: LEB128 sequence is unterminated".to_string(),
        offset,
    })
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

    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut bytes_consumed: usize = 0;

    for &byte in &data[offset..] {
        let data_bits = (byte & 0x7F) as u64;
        value |= data_bits << shift;
        bytes_consumed += 1;
        shift += 7;

        if byte & 0x80 == 0 {
            // Last byte — now check if we need sign extension.
            //
            // Condition: the top data bit of the last byte (bit 6) is set,
            // AND `shift` is less than 64 (meaning we haven't filled all 64
            // bits — if shift == 64 the full value is already there).
            if shift < 64 && (byte & 0x40) != 0 {
                // Fill all bits above `shift` with 1s.
                value |= !0u64 << shift;
            }
            return Ok((value as i64, bytes_consumed));
        }

        if shift >= 70 {
            return Err(Leb128Error {
                message: "LEB128 sequence exceeds maximum i64 width (70 bits)".to_string(),
                offset,
            });
        }
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
