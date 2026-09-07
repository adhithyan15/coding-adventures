//! Base64 encoding and decoding, per RFC 4648.
//!
//! # What base64 is
//!
//! Base64 rewrites arbitrary bytes using 64 printable characters, so binary
//! data can travel through anything that only carries text — JSON, URLs, mail
//! headers.
//!
//! The trick is that 3 bytes and 4 base64 characters hold the same number of
//! bits:
//!
//! ```text
//!   3 bytes  = 24 bits
//!   4 chars  = 24 bits   (each character carries 6 bits: 2^6 == 64)
//! ```
//!
//! So the encoder walks the input three bytes at a time, glues them into one
//! 24-bit number, and cuts that into four 6-bit pieces:
//!
//! ```text
//!   input:   M         a         n
//!   ascii:   77        97        110
//!   bits:    01001101  01100001  01101110
//!   regroup: 010011 010110 000101 101110
//!   values:  19     22     5      46
//!   symbols: T      W      F      u        ->  "TWFu"
//! ```
//!
//! The cost is 4 characters per 3 bytes: **1.33 bytes on the wire per byte of
//! data**. That is the whole reason this crate exists — the alternative in
//! use, a JSON array of decimal numbers, costs 4.6.
//!
//! # The tail is where implementations go wrong
//!
//! Input length is not always a multiple of 3. With 1 or 2 bytes left over
//! there are not enough bits to fill four symbols, so the encoder emits only
//! the symbols it has bits for and pads the rest:
//!
//! ```text
//!   "foobar" (6 = 3+3)  ->  "Zm9vYmFy"      no padding needed
//!   "fooba"  (5 = 3+2)  ->  "Zm9vYmE="      2 bytes -> 3 symbols + 1 pad
//!   "foob"   (4 = 3+1)  ->  "Zm9vYg=="      1 byte  -> 2 symbols + 2 pads
//! ```
//!
//! Every documented base64 bug lives in those last few bytes, which is why the
//! tests walk every length from 0 to 1024 rather than checking a few sizes.
//!
//! # Usage
//!
//! ```
//! use coding_adventures_base64::{decode, encode, STANDARD, URL_SAFE_NO_PAD};
//!
//! assert_eq!(encode(b"Man", &STANDARD), "TWFu");
//! assert_eq!(decode("TWFu", &STANDARD).unwrap(), b"Man");
//!
//! // The URL-safe alphabet swaps `+/` for `-_` so the output survives a query
//! // string, and dropping padding avoids `%3D` escaping.
//! assert_eq!(encode(&[0xfb, 0xff], &URL_SAFE_NO_PAD), "-_8");
//! ```
//!
//! See `code/specs/DT20-base64.md`.

#![forbid(unsafe_code)]

use std::fmt;

/// One base64 variant: which 64 symbols, and whether output is padded.
///
/// The decode table is built alongside the encode table rather than searched
/// at decode time. A linear scan of 64 symbols per input character is the
/// difference between decoding media in a loop and noticing it in a profile.
pub struct Alphabet {
    symbols: [u8; 64],
    /// `values[b]` is the 6-bit value of byte `b`, or `INVALID`.
    values: [u8; 256],
    padded: bool,
}

/// Marks a byte that is not in this alphabet. 255 cannot collide with a real
/// value, since those are 0..=63.
const INVALID: u8 = 255;

const STANDARD_SYMBOLS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE_SYMBOLS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

const PAD: u8 = b'=';

impl Alphabet {
    /// Build an alphabet and its reverse table.
    ///
    /// `const` so the four variants below cost nothing at runtime: the tables
    /// are computed at compile time and live in the binary.
    const fn new(symbols: &[u8; 64], padded: bool) -> Self {
        let mut values = [INVALID; 256];
        let mut index = 0;
        // A `while` rather than a `for`: iterators are not available in const
        // context on the edition this crate targets.
        while index < 64 {
            values[symbols[index] as usize] = index as u8;
            index += 1;
        }
        Self {
            symbols: *symbols,
            values,
            padded,
        }
    }

    /// Whether this variant writes `=` padding.
    pub const fn is_padded(&self) -> bool {
        self.padded
    }
}

/// RFC 4648 §4 — the classic alphabet, padded.
pub const STANDARD: Alphabet = Alphabet::new(STANDARD_SYMBOLS, true);
/// RFC 4648 §4 alphabet with §3.2 padding omitted.
pub const STANDARD_NO_PAD: Alphabet = Alphabet::new(STANDARD_SYMBOLS, false);
/// RFC 4648 §5 — `-` and `_` in place of `+` and `/`, padded.
pub const URL_SAFE: Alphabet = Alphabet::new(URL_SAFE_SYMBOLS, true);
/// RFC 4648 §5 alphabet with padding omitted; the usual choice for URLs.
pub const URL_SAFE_NO_PAD: Alphabet = Alphabet::new(URL_SAFE_SYMBOLS, false);

/// Why an input could not be decoded, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// A byte that is not in this alphabet, at this offset.
    InvalidByte { offset: usize, byte: u8 },
    /// A length that no encoder can produce. Length % 4 == 1 is impossible:
    /// one leftover symbol carries 6 bits, and no byte count needs exactly 6.
    InvalidLength { length: usize },
    /// Padding where the alphabet declares none, or the wrong amount of it.
    InvalidPadding { offset: usize },
    /// The final symbol carries bits that the decoded output discards, and
    /// they are not zero.
    ///
    /// `"ZG=="` is the example: `d` needs 8 bits, the two symbols carry 12,
    /// and the spare 4 must be zero. When they are not, the string did not
    /// come from encoding those bytes — and accepting it would let two
    /// different strings decode to the same value, so anything computed over
    /// the decoded bytes stops being a function of the input.
    NonCanonical { offset: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidByte { offset, byte } => write!(
                f,
                "byte {byte:#04x} at offset {offset} is not in this base64 alphabet"
            ),
            Self::InvalidLength { length } => write!(
                f,
                "length {length} is not a valid base64 length (length % 4 == 1)"
            ),
            Self::InvalidPadding { offset } => {
                write!(f, "invalid padding at offset {offset}")
            }
            Self::NonCanonical { offset } => write!(
                f,
                "the symbol at offset {offset} sets bits that decoding discards"
            ),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Exactly how many characters `encode` will produce.
pub const fn encoded_len(input_len: usize, alphabet: &Alphabet) -> usize {
    let whole_groups = input_len / 3;
    let remainder = input_len % 3;
    if remainder == 0 {
        whole_groups * 4
    } else if alphabet.padded {
        (whole_groups + 1) * 4
    } else {
        // 1 leftover byte -> 2 symbols, 2 leftover bytes -> 3.
        whole_groups * 4 + remainder + 1
    }
}

/// An upper bound on the bytes `decode` will produce.
pub const fn decoded_len_estimate(input_len: usize) -> usize {
    input_len / 4 * 3 + 3
}

/// Encode `input`, returning a new `String`.
pub fn encode(input: &[u8], alphabet: &Alphabet) -> String {
    let mut out = String::with_capacity(encoded_len(input.len(), alphabet));
    encode_into(input, alphabet, &mut out);
    out
}

/// Encode `input`, appending to a buffer the caller already owns.
///
/// This is the form the JSON writer uses. Returning a fresh `String` per media
/// asset would allocate and copy every payload a second time, which is the
/// class of amplification this crate was added to remove.
pub fn encode_into(input: &[u8], alphabet: &Alphabet, out: &mut String) {
    out.reserve(encoded_len(input.len(), alphabet));

    // `as_chunks` rather than `chunks_exact`: the size is a constant, so the
    // compiler knows each slice is exactly 3 long and drops the bounds checks.
    let (groups, tail) = input.as_chunks::<3>();
    for chunk in groups {
        // Three bytes glued into one 24-bit number, then cut into four 6-bit
        // pieces, high bits first.
        let group = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        out.push(alphabet.symbols[(group >> 18) as usize & 0x3F] as char);
        out.push(alphabet.symbols[(group >> 12) as usize & 0x3F] as char);
        out.push(alphabet.symbols[(group >> 6) as usize & 0x3F] as char);
        out.push(alphabet.symbols[group as usize & 0x3F] as char);
    }

    // The tail: 1 or 2 bytes, which do not fill four symbols. Each leftover
    // byte contributes 8 bits, and each symbol consumes 6, so the last symbol
    // is padded with zero bits on the right.
    match tail.len() {
        0 => {}
        1 => {
            let group = u32::from(tail[0]) << 16;
            out.push(alphabet.symbols[(group >> 18) as usize & 0x3F] as char);
            out.push(alphabet.symbols[(group >> 12) as usize & 0x3F] as char);
            if alphabet.padded {
                out.push(PAD as char);
                out.push(PAD as char);
            }
        }
        // Only 1 and 2 are reachable: `chunks_exact(3)` leaves fewer than 3.
        _ => {
            let group = (u32::from(tail[0]) << 16) | (u32::from(tail[1]) << 8);
            out.push(alphabet.symbols[(group >> 18) as usize & 0x3F] as char);
            out.push(alphabet.symbols[(group >> 12) as usize & 0x3F] as char);
            out.push(alphabet.symbols[(group >> 6) as usize & 0x3F] as char);
            if alphabet.padded {
                out.push(PAD as char);
            }
        }
    }
}

/// Decode `input`, rejecting anything an encoder could not have produced.
///
/// Strict by choice — see `DecodeError` and the spec. A decoder that repairs
/// its input produces bytes nobody encoded.
pub fn decode(input: &str, alphabet: &Alphabet) -> Result<Vec<u8>, DecodeError> {
    let bytes = input.as_bytes();

    // Padding is stripped first so the body below is uniform, and the amount
    // is checked against the alphabet rather than tolerated.
    let mut body_len = bytes.len();
    let mut pad_count = 0usize;
    while body_len > 0 && bytes[body_len - 1] == PAD {
        body_len -= 1;
        pad_count += 1;
        if pad_count > 2 {
            return Err(DecodeError::InvalidPadding { offset: body_len });
        }
    }
    if pad_count > 0 {
        if !alphabet.padded {
            return Err(DecodeError::InvalidPadding { offset: body_len });
        }
        // A padded encoding is always a whole number of 4-symbol groups.
        if !(body_len + pad_count).is_multiple_of(4) {
            return Err(DecodeError::InvalidPadding { offset: body_len });
        }
        // And the padding must be exactly what the body's length calls for:
        // 2 leftover symbols need 2 pads, 3 need 1.
        let expected = match body_len % 4 {
            2 => 2,
            3 => 1,
            _ => return Err(DecodeError::InvalidPadding { offset: body_len }),
        };
        if pad_count != expected {
            return Err(DecodeError::InvalidPadding { offset: body_len });
        }
    } else if alphabet.padded && !body_len.is_multiple_of(4) {
        // A padded alphabet with no padding and a ragged length is a truncated
        // encoding, not a valid unpadded one.
        return Err(DecodeError::InvalidPadding { offset: body_len });
    }

    if body_len % 4 == 1 {
        return Err(DecodeError::InvalidLength { length: input.len() });
    }

    let body = &bytes[..body_len];
    let mut out = Vec::with_capacity(decoded_len_estimate(body_len));

    let mut index = 0;
    // Whole groups of four symbols become three bytes.
    while index + 4 <= body_len {
        let a = symbol_value(alphabet, body, index)?;
        let b = symbol_value(alphabet, body, index + 1)?;
        let c = symbol_value(alphabet, body, index + 2)?;
        let d = symbol_value(alphabet, body, index + 3)?;
        let group = (u32::from(a) << 18) | (u32::from(b) << 12) | (u32::from(c) << 6) | u32::from(d);
        out.push((group >> 16) as u8);
        out.push((group >> 8) as u8);
        out.push(group as u8);
        index += 4;
    }

    // The tail: 2 or 3 symbols (1 was rejected above, 0 means we are done).
    match body_len - index {
        0 => {}
        2 => {
            let a = symbol_value(alphabet, body, index)?;
            let b = symbol_value(alphabet, body, index + 1)?;
            // 12 bits carried, 8 used. The low 4 of `b` are discarded and so
            // must be zero, or this string did not encode this byte.
            if b & 0x0F != 0 {
                return Err(DecodeError::NonCanonical { offset: index + 1 });
            }
            out.push((u32::from(a) << 2 | u32::from(b) >> 4) as u8);
        }
        _ => {
            let a = symbol_value(alphabet, body, index)?;
            let b = symbol_value(alphabet, body, index + 1)?;
            let c = symbol_value(alphabet, body, index + 2)?;
            // 18 bits carried, 16 used; the low 2 of `c` are discarded.
            if c & 0x03 != 0 {
                return Err(DecodeError::NonCanonical { offset: index + 2 });
            }
            out.push((u32::from(a) << 2 | u32::from(b) >> 4) as u8);
            out.push(((u32::from(b) & 0x0F) << 4 | u32::from(c) >> 2) as u8);
        }
    }

    Ok(out)
}

/// The 6-bit value of one symbol, or an error naming where it went wrong.
fn symbol_value(alphabet: &Alphabet, body: &[u8], offset: usize) -> Result<u8, DecodeError> {
    let byte = body[offset];
    let value = alphabet.values[byte as usize];
    if value == INVALID {
        return Err(DecodeError::InvalidByte { offset, byte });
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
