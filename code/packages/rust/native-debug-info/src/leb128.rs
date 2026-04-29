//! LEB128 variable-length integer encoding used throughout DWARF.
//!
//! ULEB128 encodes unsigned integers; SLEB128 encodes signed integers.
//! Both use 7 bits per byte, with the high bit set on all bytes except the last.
//!
//! # ULEB128 example
//!
//! ```text
//! 624485 = 0x98765
//! Bytes: 0xe5, 0x8e, 0x26  (3 bytes)
//! ```
//!
//! # SLEB128 example
//!
//! ```text
//! -123456 → Bytes: 0xc0, 0xbb, 0x78  (3 bytes)
//! ```

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

/// Encode a non-negative integer as ULEB128.
///
/// # Panics
///
/// Panics if `value < 0`.  Use [`encode_sleb128`] for signed values.
///
/// # Example
///
/// ```
/// use native_debug_info::{encode_uleb128, encode_sleb128};
///
/// assert_eq!(encode_uleb128(0), vec![0x00]);
/// assert_eq!(encode_uleb128(127), vec![0x7F]);
/// assert_eq!(encode_uleb128(128), vec![0x80, 0x01]);
/// assert_eq!(encode_uleb128(624485), vec![0xe5, 0x8e, 0x26]);
/// ```
pub fn encode_uleb128(mut value: u64) -> Vec<u8> {
    let mut result = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            result.push(byte | 0x80); // more bytes follow
        } else {
            result.push(byte);
            break;
        }
    }
    result
}

/// Encode a signed integer as SLEB128.
///
/// # Example
///
/// ```
/// use native_debug_info::encode_sleb128;
///
/// assert_eq!(encode_sleb128(0), vec![0x00]);
/// assert_eq!(encode_sleb128(-1), vec![0x7F]);
/// assert_eq!(encode_sleb128(63), vec![0x3F]);
/// assert_eq!(encode_sleb128(-123456), vec![0xc0, 0xbb, 0x78]);
/// ```
pub fn encode_sleb128(mut value: i64) -> Vec<u8> {
    let mut result = Vec::new();
    let mut more = true;
    while more {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        // We are done when:
        // - value is 0 and the sign bit (bit 6) of byte is NOT set (positive done)
        // - value is -1 and the sign bit (bit 6) of byte IS set (negative done)
        if (value == 0 && (byte & 0x40) == 0) || (value == -1 && (byte & 0x40) != 0) {
            more = false;
            result.push(byte);
        } else {
            result.push(byte | 0x80);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Decode ULEB128 from `data` at `offset`.
///
/// Returns `(value, bytes_consumed)`.
///
/// Decoding stops safely at the end of `data`: if no terminating byte is
/// found before the buffer ends, the function returns the partial value
/// decoded so far together with `data.len() - offset` consumed bytes.
/// Shift values beyond 63 bits are clamped so that bits are silently dropped
/// rather than causing an arithmetic overflow.
///
/// # Example
///
/// ```
/// use native_debug_info::decode_uleb128;
///
/// let (v, n) = decode_uleb128(&[0xe5, 0x8e, 0x26, 0xFF], 0);
/// assert_eq!(v, 624485);
/// assert_eq!(n, 3);
/// ```
pub fn decode_uleb128(data: &[u8], offset: usize) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut consumed = 0usize;
    loop {
        // Bounds guard: stop if we have consumed all available bytes.
        if offset + consumed >= data.len() {
            break;
        }
        let byte = data[offset + consumed];
        consumed += 1;
        // Shift guard: a valid u64 ULEB128 is at most 10 bytes (70 bits).
        // If shift would exceed 63, the remaining bits are outside the u64
        // range and are silently dropped to prevent arithmetic overflow.
        if shift < 64 {
            result |= ((byte & 0x7F) as u64) << shift;
        }
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
    }
    (result, consumed)
}

/// Decode SLEB128 from `data` at `offset`.
///
/// Returns `(value, bytes_consumed)`.
///
/// Like [`decode_uleb128`], decoding stops safely at end-of-buffer without
/// panicking.  Shift values beyond 63 bits are clamped.
///
/// # Example
///
/// ```
/// use native_debug_info::decode_sleb128;
///
/// let (v, n) = decode_sleb128(&[0xc0, 0xbb, 0x78], 0);
/// assert_eq!(v, -123456);
/// assert_eq!(n, 3);
/// ```
pub fn decode_sleb128(data: &[u8], offset: usize) -> (i64, usize) {
    let mut result: i64 = 0;
    let mut shift = 0u32;
    let mut consumed = 0usize;
    let mut last_byte = 0u8;
    loop {
        // Bounds guard: stop if we have consumed all available bytes.
        if offset + consumed >= data.len() {
            break;
        }
        let byte = data[offset + consumed];
        consumed += 1;
        last_byte = byte;
        // Shift guard: silently drop bits beyond the i64 range.
        if shift < 64 {
            result |= ((byte & 0x7F) as i64) << shift;
        }
        shift += 7;
        if (byte & 0x80) == 0 {
            break;
        }
    }
    // Sign-extend if the sign bit of the last byte is set.
    if shift < 64 && (last_byte & 0x40) != 0 {
        result |= -(1i64 << shift);
    }
    (result, consumed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uleb128_zero() {
        assert_eq!(encode_uleb128(0), [0x00]);
    }

    #[test]
    fn uleb128_single_byte_max() {
        assert_eq!(encode_uleb128(127), [0x7F]);
    }

    #[test]
    fn uleb128_two_bytes_128() {
        assert_eq!(encode_uleb128(128), [0x80, 0x01]);
    }

    #[test]
    fn uleb128_624485() {
        assert_eq!(encode_uleb128(624485), [0xe5, 0x8e, 0x26]);
    }

    #[test]
    fn sleb128_zero() {
        assert_eq!(encode_sleb128(0), [0x00]);
    }

    #[test]
    fn sleb128_negative_one() {
        assert_eq!(encode_sleb128(-1), [0x7F]);
    }

    #[test]
    fn sleb128_positive_63() {
        assert_eq!(encode_sleb128(63), [0x3F]);
    }

    #[test]
    fn sleb128_negative_123456() {
        assert_eq!(encode_sleb128(-123456), [0xc0, 0xbb, 0x78]);
    }

    #[test]
    fn decode_uleb128_roundtrip() {
        for val in [0u64, 1, 127, 128, 255, 624485, u32::MAX as u64] {
            let encoded = encode_uleb128(val);
            let (decoded, _) = decode_uleb128(&encoded, 0);
            assert_eq!(decoded, val, "roundtrip failed for {val}");
        }
    }

    #[test]
    fn decode_uleb128_with_offset() {
        let data = [0xFF, 0xe5, 0x8e, 0x26, 0xFF];
        let (v, n) = decode_uleb128(&data, 1);
        assert_eq!(v, 624485);
        assert_eq!(n, 3);
    }

    #[test]
    fn decode_sleb128_roundtrip() {
        for val in [0i64, -1, 63, -64, 127, -128, -123456, i32::MAX as i64, i32::MIN as i64] {
            let encoded = encode_sleb128(val);
            let (decoded, _) = decode_sleb128(&encoded, 0);
            assert_eq!(decoded, val, "roundtrip failed for {val}");
        }
    }

    #[test]
    fn decode_sleb128_negative_123456() {
        let (v, n) = decode_sleb128(&[0xc0, 0xbb, 0x78], 0);
        assert_eq!(v, -123456);
        assert_eq!(n, 3);
    }
}
