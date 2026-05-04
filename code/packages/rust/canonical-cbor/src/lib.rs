//! # coding_adventures_canonical_cbor — deterministic CBOR codec
//!
//! ## What this crate does, in one sentence
//!
//! Encode and decode [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949)
//! values using a **canonical** (deterministic) profile, so that
//! `decode(encode(v)) == v` *and* `encode(v)` is the same bytes on
//! every platform / language / process for the same `v`.
//!
//! ## Why "canonical" and not just CBOR
//!
//! Plain CBOR has freedom that wrecks our use case:
//!
//! * The integer 0 can be `0x00`, `0x18 0x00`, `0x19 0x00 0x00`,
//!   `0x1A 0x00 0x00 0x00 0x00`, or `0x1B 0x00 0x00 0x00 0x00 0x00
//!   0x00 0x00 0x00`.
//! * An array's elements have a defined order, but a map's entries
//!   do not — `{"a":1,"b":2}` and `{"b":2,"a":1}` are both legal.
//! * Lengths can be definite (`0x82 …`) or indefinite-length with a
//!   `0xFF` break marker.
//!
//! The Vault stack needs **stable record bytes** because:
//!
//! 1. AEAD AAD binding (`namespace || 0x00 || key`) and authenticated
//!    record content require that "the same record" hashes to the
//!    same bytes — otherwise re-encoding the same logical record
//!    invalidates the existing tag.
//! 2. Sync conflict detection compares record revisions byte-for-byte.
//!    Floating encoding kills that.
//! 3. COSE-Key, used by FIDO2 / WebAuthn-PRF (VLT05 bind-mode), is a
//!    canonical-CBOR-derived format. We need exactly that ordering.
//!
//! ## Profile
//!
//! This crate implements **RFC 8949 §4.2.3 "Length-First Map Key
//! Ordering"** (sometimes called "CTAP2 canonical" or "RFC 7049
//! canonical"). Specifically:
//!
//! * **Definite length only.** Indefinite-length items (`0x9F`,
//!   `0xBF`, `0x5F`, `0x7F`, `0xFF` break) are rejected by the
//!   decoder and never produced by the encoder.
//! * **Smallest-form integer encoding.** Lengths and values use the
//!   shortest of inline (5-bit additional info), 1-byte, 2-byte,
//!   4-byte, 8-byte representation that fits the value. Decoder
//!   rejects "expanded" forms (e.g. `0x18 0x05` — a 1-byte 5 — must
//!   be `0x05`).
//! * **Map keys sorted length-first.** When encoding a map, keys are
//!   first sorted by the *length* of their canonical CBOR encoding,
//!   ties broken by bytewise lex of the encoded form. This is the
//!   ordering CTAP2 / COSE / WebAuthn use; matches RFC 8949 §4.2.3.
//! * **No floats** in this version. Vault records do not need them,
//!   and the shortest-form rule for floats (preserve-the-value
//!   among float16/32/64) is a separate beast. Future work.
//! * **Tags pass through.** The encoder writes whatever tag number
//!   the caller supplies; the decoder produces a `Tag` value but
//!   does not interpret semantics. Higher layers may reject unknown
//!   tags.
//! * **No `undefined`** in this version. Decoder rejects
//!   `0xF7`. Vault records use `Null` (0xF6) for "absent."
//!
//! ## What canonical-CBOR is *not*
//!
//! Not the place for: cryptographic AEAD framing, schema evolution
//! beyond version tags, signature formats. Those live higher up
//! (VLT02 records, VLT01 sealed store, COSE-Sign / JWS bindings).
//!
//! ## Usage
//!
//! ```
//! use coding_adventures_canonical_cbor::{CborValue, encode, decode};
//!
//! let v = CborValue::Map(vec![
//!     (CborValue::text("title"), CborValue::text("hello")),
//!     (CborValue::text("count"), CborValue::Unsigned(42)),
//! ]);
//! let bytes = encode(&v);
//! // Map keys are reordered length-first ("count" comes before "title"
//! // because their canonical encodings are 5 bytes vs 6 bytes — the
//! // text-string-header makes them tied at 1, so the lex tiebreak runs
//! // and "count" < "title" lex-wise. Either way, the same input always
//! // produces the same bytes.)
//!
//! let back = decode(&bytes).unwrap();
//! // back is a Map with the SAME entries, in the canonical order.
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use core::cmp::Ordering;

// ─────────────────────────────────────────────────────────────────────
// 1. The value type
// ─────────────────────────────────────────────────────────────────────

/// A CBOR value in the canonical profile this crate supports.
///
/// `Unsigned(n)` represents the non-negative integer `n`.
/// `Negative(n)` represents `-1 - n` (so `Negative(0) == -1`,
/// `Negative(1) == -2`, …, `Negative(u64::MAX)` is the most
/// negative representable integer in CBOR — namely
/// `−2^64`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CborValue {
    /// Unsigned integer (CBOR major type 0).
    Unsigned(u64),
    /// Negative integer (CBOR major type 1). The encoded value
    /// represents `-1 - n`, so callers should think of it as "how
    /// many below −1": `Negative(0) = −1`, `Negative(41) = −42`.
    Negative(u64),
    /// Byte string (CBOR major type 2).
    Bytes(Vec<u8>),
    /// UTF-8 text string (CBOR major type 3). The decoder validates
    /// UTF-8.
    Text(String),
    /// Array (CBOR major type 4). Element order is preserved.
    Array(Vec<CborValue>),
    /// Map (CBOR major type 5). Key order is canonicalised (length-
    /// first then bytewise lex) at encode time. The decoder returns
    /// keys in their on-wire order *and* verifies that order matches
    /// the canonical sort, rejecting non-canonical inputs.
    Map(Vec<(CborValue, CborValue)>),
    /// Tagged value (CBOR major type 6). Tag number is opaque to
    /// this crate.
    Tag(u64, Box<CborValue>),
    /// Boolean (CBOR major type 7, simple values 20 / 21).
    Bool(bool),
    /// Null (CBOR major type 7, simple value 22).
    Null,
}

impl CborValue {
    /// Build a [`CborValue::Text`] from anything `Into<String>`.
    pub fn text(s: impl Into<String>) -> Self {
        CborValue::Text(s.into())
    }

    /// Build a [`CborValue::Bytes`] from anything `Into<Vec<u8>>`.
    pub fn bytes(b: impl Into<Vec<u8>>) -> Self {
        CborValue::Bytes(b.into())
    }
}

// ─────────────────────────────────────────────────────────────────────
// 2. Errors
// ─────────────────────────────────────────────────────────────────────

/// Errors returned by [`decode`].
///
/// Display strings are sourced from literals in this crate — never
/// from persisted bytes — so a malicious input can't inject error
/// messages into our logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CborError {
    /// Hit end-of-input mid-item.
    UnexpectedEof,
    /// Trailing bytes after a single decoded item.
    TrailingBytes,
    /// Reserved additional-info value (28, 29, 30) was seen. RFC 8949
    /// reserves these and the canonical profile rejects them.
    Reserved,
    /// Indefinite-length item (additional info 31) was seen. The
    /// canonical profile is definite-length only.
    Indefinite,
    /// Integer or length was encoded in a longer form than necessary
    /// (e.g. 5 written as `0x18 0x05` instead of `0x05`). The
    /// canonical profile is smallest-form only.
    NonMinimalInteger,
    /// Text string (major type 3) was not valid UTF-8.
    InvalidUtf8,
    /// Map keys were not in length-first canonical order, or a key
    /// appeared twice.
    NonCanonicalMapOrder,
    /// A simple value other than `true (21)`, `false (20)`, or
    /// `null (22)` was seen. The canonical profile rejects
    /// `undefined (23)` and all unassigned simple values.
    UnsupportedSimple,
    /// A floating-point value was seen. This crate does not (yet)
    /// support floats; encode/decode of floats will be added in a
    /// future version.
    FloatNotSupported,
    /// Recursion depth exceeded [`MAX_DECODE_DEPTH`]. Attackers can
    /// craft small inputs (chains of nested arrays / tags) that
    /// recurse deeply enough to blow the OS stack; the decoder
    /// caps depth defensively.
    TooDeep,
    /// A length / count field declared more elements than could
    /// possibly be present in the remaining bytes (or larger than
    /// `usize` on this platform). Either truncation or a DoS
    /// attempt — either way the input is invalid.
    LengthTooLarge,
}

impl core::fmt::Display for CborError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            CborError::UnexpectedEof => "canonical-cbor: unexpected end of input",
            CborError::TrailingBytes => "canonical-cbor: trailing bytes after decoded item",
            CborError::Reserved => "canonical-cbor: reserved additional-info value (28/29/30)",
            CborError::Indefinite => "canonical-cbor: indefinite-length items rejected by canonical profile",
            CborError::NonMinimalInteger => "canonical-cbor: integer not in smallest-form encoding",
            CborError::InvalidUtf8 => "canonical-cbor: text string is not valid UTF-8",
            CborError::NonCanonicalMapOrder => "canonical-cbor: map keys not in length-first canonical order or duplicated",
            CborError::UnsupportedSimple => "canonical-cbor: unsupported simple value (only true / false / null permitted)",
            CborError::FloatNotSupported => "canonical-cbor: floats are not supported in this version",
            CborError::TooDeep => "canonical-cbor: nesting depth exceeded the configured maximum",
            CborError::LengthTooLarge => "canonical-cbor: length / count exceeds the remaining bytes or platform usize",
        };
        write!(f, "{}", s)
    }
}

impl std::error::Error for CborError {}

// ─────────────────────────────────────────────────────────────────────
// 3. Encoder — emits canonical bytes by construction
// ─────────────────────────────────────────────────────────────────────
//
// CBOR header byte = (major_type << 5) | additional_info
//   additional_info encodes either the value itself (0..23) or
//   the size of the following big-endian integer field:
//     24 -> 1 byte, 25 -> 2 bytes, 26 -> 4 bytes, 27 -> 8 bytes.
//
// For canonical encoding, we always pick the SHORTEST form that fits.

/// Encode a [`CborValue`] into canonical CBOR bytes.
///
/// The output is deterministic: the same input value always produces
/// the same bytes. Map entries are sorted length-first then bytewise
/// at encode time; the caller can pass them in any order.
pub fn encode(v: &CborValue) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(v, &mut out);
    out
}

/// Same as [`encode`] but writes into a caller-provided buffer.
pub fn encode_into(v: &CborValue, out: &mut Vec<u8>) {
    match v {
        CborValue::Unsigned(n) => write_type_and_argument(out, 0, *n),
        CborValue::Negative(n) => write_type_and_argument(out, 1, *n),
        CborValue::Bytes(b) => {
            write_type_and_argument(out, 2, b.len() as u64);
            out.extend_from_slice(b);
        }
        CborValue::Text(s) => {
            write_type_and_argument(out, 3, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        CborValue::Array(items) => {
            write_type_and_argument(out, 4, items.len() as u64);
            for item in items {
                encode_into(item, out);
            }
        }
        CborValue::Map(entries) => {
            // Canonicalise: encode each key, sort (length-first then
            // bytewise), then emit (key_bytes, value_bytes) in order.
            //
            // We allocate a temporary list of (encoded_key, value_ref)
            // so we don't double-encode keys. Memory overhead is one
            // pass over the map, which is acceptable for vault records
            // (small) and reasonable for everything else.
            let mut encoded: Vec<(Vec<u8>, &CborValue)> = entries
                .iter()
                .map(|(k, v)| (encode(k), v))
                .collect();
            encoded.sort_by(length_first_lex);
            // Note: duplicates among the encoded keys are NOT detected
            // here; the encoder is permissive on input. The decoder
            // rejects duplicates. Most callers should pre-check.
            write_type_and_argument(out, 5, encoded.len() as u64);
            for (k_bytes, v_ref) in &encoded {
                out.extend_from_slice(k_bytes);
                encode_into(v_ref, out);
            }
        }
        CborValue::Tag(tag, inner) => {
            write_type_and_argument(out, 6, *tag);
            encode_into(inner, out);
        }
        CborValue::Bool(false) => out.push(0xF4), // major 7, info 20
        CborValue::Bool(true) => out.push(0xF5),  // major 7, info 21
        CborValue::Null => out.push(0xF6),        // major 7, info 22
    }
}

/// Length-first then bytewise lex comparator on encoded-key bytes.
fn length_first_lex(a: &(Vec<u8>, &CborValue), b: &(Vec<u8>, &CborValue)) -> Ordering {
    match a.0.len().cmp(&b.0.len()) {
        Ordering::Equal => a.0.cmp(&b.0),
        other => other,
    }
}

/// Write a CBOR header byte plus its big-endian argument in the
/// shortest form that fits.
fn write_type_and_argument(out: &mut Vec<u8>, major: u8, arg: u64) {
    debug_assert!(major <= 7);
    let mt = major << 5;
    if arg <= 23 {
        out.push(mt | arg as u8);
    } else if arg <= 0xFF {
        out.push(mt | 24);
        out.push(arg as u8);
    } else if arg <= 0xFFFF {
        out.push(mt | 25);
        out.extend_from_slice(&(arg as u16).to_be_bytes());
    } else if arg <= 0xFFFF_FFFF {
        out.push(mt | 26);
        out.extend_from_slice(&(arg as u32).to_be_bytes());
    } else {
        out.push(mt | 27);
        out.extend_from_slice(&arg.to_be_bytes());
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. Decoder — strict canonical parser
// ─────────────────────────────────────────────────────────────────────
//
// Strategy: a tiny cursor over the input. Each `read_*` method
// advances the cursor and returns the parsed value or a CborError.
// At the top, we parse one item then assert the cursor is at EOF.

/// Maximum recursion depth accepted by [`decode`]. CTAP2 / COSE
/// payloads are flat by design (depth ≤ ~5); 128 is a generous
/// cap that protects against attacker-crafted "chain of nested
/// arrays" inputs that would otherwise blow the OS stack.
pub const MAX_DECODE_DEPTH: usize = 128;

/// Decode a single CBOR value from a byte slice. The slice must
/// contain exactly one canonical CBOR item — trailing bytes are an
/// error.
///
/// Defensive caps applied during decoding:
///
/// * **Recursion depth** is capped at [`MAX_DECODE_DEPTH`].
/// * **Length / count fields** are bounded by the remaining input
///   length (an array claiming `2^63` elements when only 9 input
///   bytes remain is rejected without ever calling
///   `Vec::with_capacity`).
/// * **Cursor arithmetic** is checked: `pos + n` overflow is treated
///   as `LengthTooLarge`, never as a wraparound that would bypass
///   the bounds check.
pub fn decode(bytes: &[u8]) -> Result<CborValue, CborError> {
    let mut cur = Cursor { bytes, pos: 0 };
    let v = read_value(&mut cur, 0)?;
    if cur.pos != cur.bytes.len() {
        return Err(CborError::TrailingBytes);
    }
    Ok(v)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn read_u8(&mut self) -> Result<u8, CborError> {
        if self.pos >= self.bytes.len() {
            return Err(CborError::UnexpectedEof);
        }
        let b = self.bytes[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// Read exactly `n` bytes and advance the cursor.
    ///
    /// Uses checked arithmetic so a hostile `n` value cannot wrap
    /// `pos + n` past `usize::MAX` and slip past the bounds check.
    /// Overflow is reported as [`CborError::LengthTooLarge`]; an
    /// in-bounds `pos + n` that exceeds the buffer is reported as
    /// [`CborError::UnexpectedEof`].
    fn read_n(&mut self, n: usize) -> Result<&'a [u8], CborError> {
        let end = self.pos.checked_add(n).ok_or(CborError::LengthTooLarge)?;
        if end > self.bytes.len() {
            return Err(CborError::UnexpectedEof);
        }
        let s = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    /// Number of bytes the input still has left after the cursor.
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }
}

/// Convert a CBOR-declared u64 length to a `usize`, rejecting any
/// declared length larger than the remaining input (or larger than
/// `usize::MAX` on a small target). `min_per_unit` is the lower
/// bound on the wire-byte cost of each unit being counted (1 for
/// element bytes, 2 for map entries — at least one byte for key
/// header and one for value header).
fn length_within_remaining(
    declared: u64,
    remaining: usize,
    min_per_unit: usize,
) -> Result<usize, CborError> {
    let declared = usize::try_from(declared).map_err(|_| CborError::LengthTooLarge)?;
    // Each declared unit consumes at least `min_per_unit` wire
    // bytes; if the input doesn't have that many bytes left, no
    // amount of subsequent reading can succeed.
    if let Some(min_required) = declared.checked_mul(min_per_unit) {
        if min_required > remaining {
            return Err(CborError::LengthTooLarge);
        }
    } else {
        return Err(CborError::LengthTooLarge);
    }
    Ok(declared)
}

/// Read a CBOR header and its argument.
///
/// Returns `(major, info, arg)` where:
///
/// * `major` is the 3-bit major type (0..=7).
/// * `info` is the 5-bit additional-info code (0..=31), preserved as
///   read from the wire — callers that need to distinguish float
///   payloads from integer-encoded values (only major type 7) need
///   the info byte distinct from the argument.
/// * `arg` is the unsigned argument:
///     - For info 0..=23: arg = info (the value is inline).
///     - For info 24/25/26/27: arg is the big-endian 1/2/4/8-byte
///       payload.
///
/// For major types 0..=6 we enforce **smallest-form integer
/// encoding** — `0x18 0x05` (5 in 1-byte form) is rejected because
/// it should have been encoded inline as `0x05`. For major type 7,
/// info 25/26/27 are float bit patterns (not integers); their
/// values are not "re-encodings of smaller integers," so the
/// smallest-form check does not apply.
///
/// Reserved (28..=30) and indefinite (31) info values are always
/// rejected.
fn read_header(cur: &mut Cursor) -> Result<(u8, u8, u64), CborError> {
    let b = cur.read_u8()?;
    let major = b >> 5;
    let info = b & 0x1F;
    let enforce_minimal = major != 7;
    let arg = match info {
        0..=23 => info as u64,
        24 => {
            let v = cur.read_u8()? as u64;
            if enforce_minimal && v <= 23 {
                return Err(CborError::NonMinimalInteger);
            }
            v
        }
        25 => {
            let bs = cur.read_n(2)?;
            let v = u16::from_be_bytes([bs[0], bs[1]]) as u64;
            if enforce_minimal && v <= 0xFF {
                return Err(CborError::NonMinimalInteger);
            }
            v
        }
        26 => {
            let bs = cur.read_n(4)?;
            let v = u32::from_be_bytes([bs[0], bs[1], bs[2], bs[3]]) as u64;
            if enforce_minimal && v <= 0xFFFF {
                return Err(CborError::NonMinimalInteger);
            }
            v
        }
        27 => {
            let bs = cur.read_n(8)?;
            let v = u64::from_be_bytes([bs[0], bs[1], bs[2], bs[3], bs[4], bs[5], bs[6], bs[7]]);
            if enforce_minimal && v <= 0xFFFF_FFFF {
                return Err(CborError::NonMinimalInteger);
            }
            v
        }
        28 | 29 | 30 => return Err(CborError::Reserved),
        31 => return Err(CborError::Indefinite),
        _ => unreachable!(),
    };
    Ok((major, info, arg))
}

fn read_value(cur: &mut Cursor, depth: usize) -> Result<CborValue, CborError> {
    // Defence against attacker-crafted deeply-nested input. A small
    // payload like 0xC6 0xC6 0xC6 ... (a chain of tags) or 0x81 0x81
    // 0x81 ... (nested singleton arrays) would otherwise recurse
    // until the OS stack overflows.
    if depth > MAX_DECODE_DEPTH {
        return Err(CborError::TooDeep);
    }
    // Note the start position so we can extract the encoded form of
    // a key when validating map ordering.
    let item_start = cur.pos;
    let _ = item_start;
    let (major, info, arg) = read_header(cur)?;
    match major {
        0 => Ok(CborValue::Unsigned(arg)),
        1 => Ok(CborValue::Negative(arg)),
        2 => {
            // Reject impossible lengths *before* allocating. Each
            // body byte costs one wire byte, so a length > remaining
            // can never succeed.
            let len = length_within_remaining(arg, cur.remaining(), 1)?;
            let s = cur.read_n(len)?;
            Ok(CborValue::Bytes(s.to_vec()))
        }
        3 => {
            let len = length_within_remaining(arg, cur.remaining(), 1)?;
            let s = cur.read_n(len)?;
            let s = std::str::from_utf8(s).map_err(|_| CborError::InvalidUtf8)?;
            Ok(CborValue::Text(s.to_string()))
        }
        4 => {
            // Each array element costs at least one wire byte. So
            // if `arg` exceeds `remaining`, the input is invalid;
            // we reject without ever allocating an N-element Vec.
            let count = length_within_remaining(arg, cur.remaining(), 1)?;
            let mut items = Vec::with_capacity(count);
            for _ in 0..count {
                items.push(read_value(cur, depth + 1)?);
            }
            Ok(CborValue::Array(items))
        }
        5 => {
            // Each map entry is at least 2 wire bytes (one-byte
            // header for key, one-byte header for value). So
            // `arg * 2 > remaining` ⇒ invalid.
            let count = length_within_remaining(arg, cur.remaining(), 2)?;
            let mut entries = Vec::with_capacity(count);
            // We track the encoded-key bytes so we can verify
            // length-first canonical order afterwards.
            let mut prev_key_bytes: Option<&[u8]> = None;
            for _ in 0..count {
                let key_start = cur.pos;
                let k = read_value(cur, depth + 1)?;
                let key_end = cur.pos;
                let v = read_value(cur, depth + 1)?;

                // Compare against prev key bytes — length-first then bytewise.
                if let Some(prev) = prev_key_bytes {
                    let cur_key = &cur.bytes[key_start..key_end];
                    if !key_strictly_less(prev, cur_key) {
                        return Err(CborError::NonCanonicalMapOrder);
                    }
                }
                prev_key_bytes = Some(&cur.bytes[key_start..key_end]);
                entries.push((k, v));
            }
            Ok(CborValue::Map(entries))
        }
        6 => {
            let inner = read_value(cur, depth + 1)?;
            Ok(CborValue::Tag(arg, Box::new(inner)))
        }
        7 => {
            // For major type 7, the info code (not the argument) tells us
            // whether this is a simple value or a float:
            //   info 0..=19   — simple value 0..=19 (unassigned; reject)
            //   info 20       — false
            //   info 21       — true
            //   info 22       — null
            //   info 23       — undefined (rejected by canonical profile)
            //   info 24       — simple value in next byte (range 32..=255;
            //                   none are currently assigned, reject)
            //   info 25/26/27 — half / single / double float (unsupported v1)
            //   info 28..=31  — already rejected by read_header
            //
            // Note: the extra suppression below is a non-warning hack for
            // unused `arg` in arms that ignore it.
            let _ = arg;
            match info {
                20 => Ok(CborValue::Bool(false)),
                21 => Ok(CborValue::Bool(true)),
                22 => Ok(CborValue::Null),
                25 | 26 | 27 => Err(CborError::FloatNotSupported),
                _ => Err(CborError::UnsupportedSimple),
            }
        }
        _ => unreachable!(), // major is 3 bits, 0..=7
    }
}

/// Strict length-first then bytewise-lex `<` on key encodings.
/// Equality returns false (so duplicates are rejected as
/// non-canonical).
fn key_strictly_less(a: &[u8], b: &[u8]) -> bool {
    match a.len().cmp(&b.len()) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => a < b,
    }
}

// ─────────────────────────────────────────────────────────────────────
// 5. Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // --- Smallest-form integer encoding ---

    #[test]
    fn encode_small_unsigned_inline() {
        // 0..=23 use the additional-info field directly.
        for n in 0..=23u64 {
            let bytes = encode(&CborValue::Unsigned(n));
            assert_eq!(bytes, vec![n as u8]);
        }
    }

    #[test]
    fn encode_unsigned_24_uses_one_byte_form() {
        let bytes = encode(&CborValue::Unsigned(24));
        assert_eq!(bytes, vec![0x18, 24]);
    }

    #[test]
    fn encode_unsigned_255_uses_one_byte_form() {
        let bytes = encode(&CborValue::Unsigned(255));
        assert_eq!(bytes, vec![0x18, 255]);
    }

    #[test]
    fn encode_unsigned_256_uses_two_byte_form() {
        let bytes = encode(&CborValue::Unsigned(256));
        assert_eq!(bytes, vec![0x19, 0x01, 0x00]);
    }

    #[test]
    fn encode_unsigned_65536_uses_four_byte_form() {
        let bytes = encode(&CborValue::Unsigned(65536));
        assert_eq!(bytes, vec![0x1A, 0x00, 0x01, 0x00, 0x00]);
    }

    #[test]
    fn encode_unsigned_max_uses_eight_byte_form() {
        let bytes = encode(&CborValue::Unsigned(u64::MAX));
        let mut expected = vec![0x1B];
        expected.extend_from_slice(&u64::MAX.to_be_bytes());
        assert_eq!(bytes, expected);
    }

    // --- Decoder rejects non-minimal forms ---

    #[test]
    fn decode_rejects_one_byte_form_holding_small_value() {
        // 0x18 0x05 is "5 in 1-byte form" — 5 should have been inline.
        let bytes = vec![0x18, 0x05];
        assert!(matches!(decode(&bytes), Err(CborError::NonMinimalInteger)));
    }

    #[test]
    fn decode_rejects_two_byte_form_holding_byte_value() {
        let bytes = vec![0x19, 0x00, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::NonMinimalInteger)));
    }

    #[test]
    fn decode_rejects_four_byte_form_holding_short_value() {
        let bytes = vec![0x1A, 0x00, 0x00, 0xFF, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::NonMinimalInteger)));
    }

    #[test]
    fn decode_rejects_eight_byte_form_holding_int_value() {
        let bytes = vec![0x1B, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::NonMinimalInteger)));
    }

    // --- Negatives ---

    #[test]
    fn negative_one_encodes_to_0x20() {
        // -1 = Negative(0). Major 1 with info 0 = 0x20.
        let bytes = encode(&CborValue::Negative(0));
        assert_eq!(bytes, vec![0x20]);
    }

    #[test]
    fn negative_minus_24_encodes_to_0x37() {
        // -24 = Negative(23). Major 1 with info 23 = 0x37.
        let bytes = encode(&CborValue::Negative(23));
        assert_eq!(bytes, vec![0x37]);
    }

    #[test]
    fn negative_minus_25_uses_one_byte_form() {
        // -25 = Negative(24). Cannot fit inline (info 24 means
        // "next byte is value"), so 1-byte form.
        let bytes = encode(&CborValue::Negative(24));
        assert_eq!(bytes, vec![0x38, 24]);
    }

    // --- Bytes / Text ---

    #[test]
    fn encode_empty_bytes() {
        let bytes = encode(&CborValue::Bytes(vec![]));
        assert_eq!(bytes, vec![0x40]); // major 2, length 0
    }

    #[test]
    fn encode_short_bytes() {
        let bytes = encode(&CborValue::Bytes(vec![1, 2, 3, 4]));
        assert_eq!(bytes, vec![0x44, 1, 2, 3, 4]);
    }

    #[test]
    fn encode_text_short() {
        let bytes = encode(&CborValue::Text("abc".to_string()));
        assert_eq!(bytes, vec![0x63, b'a', b'b', b'c']);
    }

    #[test]
    fn decode_rejects_invalid_utf8() {
        // text-string of length 1 with byte 0xFF (invalid UTF-8 lead byte).
        let bytes = vec![0x61, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::InvalidUtf8)));
    }

    // --- Arrays ---

    #[test]
    fn encode_empty_array() {
        let bytes = encode(&CborValue::Array(vec![]));
        assert_eq!(bytes, vec![0x80]); // major 4, length 0
    }

    #[test]
    fn encode_array_preserves_order() {
        let v = CborValue::Array(vec![
            CborValue::Unsigned(1),
            CborValue::Unsigned(2),
            CborValue::Unsigned(3),
        ]);
        let bytes = encode(&v);
        assert_eq!(bytes, vec![0x83, 0x01, 0x02, 0x03]);

        // Reverse order should encode differently.
        let v2 = CborValue::Array(vec![
            CborValue::Unsigned(3),
            CborValue::Unsigned(2),
            CborValue::Unsigned(1),
        ]);
        let bytes2 = encode(&v2);
        assert_eq!(bytes2, vec![0x83, 0x03, 0x02, 0x01]);
    }

    // --- Maps: length-first canonical ordering ---

    #[test]
    fn encode_map_orders_keys_length_first() {
        // Two text keys. Both encode as "(0x60 + len) || utf8 bytes".
        // "a" is 2 bytes (0x61 + 'a'); "bb" is 3 bytes.
        // Length-first puts "a" before "bb" regardless of input order.
        let m1 = CborValue::Map(vec![
            (CborValue::text("a"), CborValue::Unsigned(1)),
            (CborValue::text("bb"), CborValue::Unsigned(2)),
        ]);
        let m2 = CborValue::Map(vec![
            (CborValue::text("bb"), CborValue::Unsigned(2)),
            (CborValue::text("a"), CborValue::Unsigned(1)),
        ]);
        assert_eq!(encode(&m1), encode(&m2));
        // First key on wire is "a" (header 0x61, value 'a').
        let bytes = encode(&m1);
        assert_eq!(bytes[0], 0xA2); // map of length 2
        assert_eq!(bytes[1], 0x61);
        assert_eq!(bytes[2], b'a');
    }

    #[test]
    fn encode_map_breaks_length_ties_lex() {
        // Two keys of the same encoded length: "a" (0x61 'a') and
        // "b" (0x61 'b'). "a" < "b" lex.
        let m = CborValue::Map(vec![
            (CborValue::text("b"), CborValue::Unsigned(2)),
            (CborValue::text("a"), CborValue::Unsigned(1)),
        ]);
        let bytes = encode(&m);
        // Expected: 0xA2 (map-2), 0x61 'a' 0x01, 0x61 'b' 0x02.
        assert_eq!(bytes, vec![0xA2, 0x61, b'a', 0x01, 0x61, b'b', 0x02]);
    }

    #[test]
    fn decode_accepts_canonical_map() {
        let bytes = vec![0xA2, 0x61, b'a', 0x01, 0x61, b'b', 0x02];
        let v = decode(&bytes).unwrap();
        match v {
            CborValue::Map(entries) => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].0, CborValue::text("a"));
                assert_eq!(entries[1].0, CborValue::text("b"));
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn decode_rejects_non_canonical_map_order() {
        // Same map but b before a. Since both keys encode to length 2,
        // decoder must reject "0x61 b" preceding "0x61 a".
        let bytes = vec![0xA2, 0x61, b'b', 0x02, 0x61, b'a', 0x01];
        assert!(matches!(decode(&bytes), Err(CborError::NonCanonicalMapOrder)));
    }

    #[test]
    fn decode_rejects_duplicate_map_keys() {
        // Two entries with the same key "a".
        let bytes = vec![0xA2, 0x61, b'a', 0x01, 0x61, b'a', 0x02];
        assert!(matches!(decode(&bytes), Err(CborError::NonCanonicalMapOrder)));
    }

    // --- Round-trip ---

    #[test]
    fn roundtrip_complex_structure() {
        let v = CborValue::Map(vec![
            (
                CborValue::text("title"),
                CborValue::Text("hello world".to_string()),
            ),
            (CborValue::text("count"), CborValue::Unsigned(42)),
            (
                CborValue::text("tags"),
                CborValue::Array(vec![
                    CborValue::text("urgent"),
                    CborValue::text("draft"),
                ]),
            ),
            (
                CborValue::text("meta"),
                CborValue::Map(vec![
                    (CborValue::text("v"), CborValue::Unsigned(1)),
                    (CborValue::text("draft"), CborValue::Bool(true)),
                ]),
            ),
            (CborValue::text("note"), CborValue::Null),
            (
                CborValue::text("blob"),
                CborValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]),
            ),
        ]);
        let bytes = encode(&v);
        let back = decode(&bytes).unwrap();
        // After decode, the map keys come back in canonical order
        // — possibly different from the input order. Re-encode and
        // compare bytes for equality.
        assert_eq!(encode(&back), bytes);
        // And re-encoding from `back` must be idempotent.
        let rebytes = encode(&back);
        assert_eq!(bytes, rebytes);
    }

    #[test]
    fn roundtrip_idempotent_on_unsorted_input() {
        let v = CborValue::Map(vec![
            (CborValue::text("z"), CborValue::Unsigned(26)),
            (CborValue::text("a"), CborValue::Unsigned(1)),
            (CborValue::text("m"), CborValue::Unsigned(13)),
        ]);
        let bytes = encode(&v);
        let back = decode(&bytes).unwrap();
        assert_eq!(encode(&back), bytes);
    }

    // --- Tags ---

    #[test]
    fn tag_roundtrip() {
        let v = CborValue::Tag(0, Box::new(CborValue::text("2026-05-04")));
        let bytes = encode(&v);
        // tag 0 inline = 0xC0.
        assert_eq!(bytes[0], 0xC0);
        let back = decode(&bytes).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn tag_with_large_number() {
        let v = CborValue::Tag(1234567, Box::new(CborValue::Unsigned(0)));
        let bytes = encode(&v);
        let back = decode(&bytes).unwrap();
        assert_eq!(back, v);
    }

    // --- Reject indefinite, reserved, undefined, floats ---

    #[test]
    fn decode_rejects_indefinite_array() {
        let bytes = vec![0x9F, 0x01, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::Indefinite)));
    }

    #[test]
    fn decode_rejects_indefinite_map() {
        let bytes = vec![0xBF, 0x61, b'a', 0x01, 0xFF];
        assert!(matches!(decode(&bytes), Err(CborError::Indefinite)));
    }

    #[test]
    fn decode_rejects_reserved_info_28() {
        let bytes = vec![0x1C]; // major 0, info 28
        assert!(matches!(decode(&bytes), Err(CborError::Reserved)));
    }

    #[test]
    fn decode_rejects_undefined() {
        let bytes = vec![0xF7]; // major 7, info 23 -> simple value 23 = undefined
        assert!(matches!(decode(&bytes), Err(CborError::UnsupportedSimple)));
    }

    #[test]
    fn decode_rejects_float_half() {
        let bytes = vec![0xF9, 0x00, 0x00];
        assert!(matches!(decode(&bytes), Err(CborError::FloatNotSupported)));
    }

    #[test]
    fn decode_rejects_float_single() {
        let bytes = vec![0xFA, 0x00, 0x00, 0x00, 0x00];
        assert!(matches!(decode(&bytes), Err(CborError::FloatNotSupported)));
    }

    #[test]
    fn decode_rejects_float_double() {
        let bytes = vec![0xFB, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(matches!(decode(&bytes), Err(CborError::FloatNotSupported)));
    }

    // --- Trailing bytes ---

    #[test]
    fn decode_rejects_trailing_bytes() {
        // 0x01 (unsigned 1) followed by an extra byte.
        let bytes = vec![0x01, 0x00];
        assert!(matches!(decode(&bytes), Err(CborError::TrailingBytes)));
    }

    // --- EOF ---

    #[test]
    fn decode_rejects_eof_in_header() {
        let bytes = vec![];
        assert!(matches!(decode(&bytes), Err(CborError::UnexpectedEof)));
    }

    #[test]
    fn decode_rejects_eof_in_argument() {
        // 0x18 says "next byte is value", but no next byte.
        let bytes = vec![0x18];
        assert!(matches!(decode(&bytes), Err(CborError::UnexpectedEof)));
    }

    #[test]
    fn decode_rejects_truncated_byte_string() {
        // 0x44 says "byte string of length 4" — only 2 bytes follow.
        // The new defensive cap converts this into LengthTooLarge
        // *before* attempting the read, so an attacker can never
        // make the decoder allocate a buffer they didn't actually
        // pay for. (Old behaviour was UnexpectedEof from read_n.)
        let bytes = vec![0x44, 0xAA, 0xBB];
        assert!(matches!(decode(&bytes), Err(CborError::LengthTooLarge)));
    }

    // --- Stress / large inputs ---

    #[test]
    fn large_array_roundtrip() {
        let n = 1000;
        let arr: Vec<CborValue> = (0..n as u64).map(CborValue::Unsigned).collect();
        let v = CborValue::Array(arr);
        let bytes = encode(&v);
        let back = decode(&bytes).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn large_map_roundtrip_and_canonical() {
        // Build a map of 100 keys, shuffled, and assert encode is
        // deterministic regardless of input order.
        let n = 100;
        let mut entries: Vec<(CborValue, CborValue)> =
            (0..n).map(|i| (CborValue::Unsigned(i), CborValue::Unsigned(i * 7))).collect();
        let mut shuffled = entries.clone();
        shuffled.reverse();
        let bytes_a = encode(&CborValue::Map(entries.clone()));
        let bytes_b = encode(&CborValue::Map(shuffled));
        assert_eq!(bytes_a, bytes_b);
        // A round trip preserves entries (in canonical order).
        let back = decode(&bytes_a).unwrap();
        if let CborValue::Map(got) = back {
            // Sort the original by integer key for comparison.
            entries.sort_by_key(|(k, _)| match k {
                CborValue::Unsigned(n) => *n,
                _ => unreachable!(),
            });
            // Canonical order on Unsigned keys 0..100 happens to be
            // length-first — values 0..23 are 1-byte, 24..99 are 2-byte.
            // So "23" comes before "24" but ordering inside each
            // length bucket is by encoded bytes, which for unsigned
            // ints is the same as numeric order within a bucket.
            // Lengths bucket: [0..23], [24..99].
            // Ordering: 0,1,…,23, then 24,25,…,99.
            let canonical_order: Vec<u64> = (0..n).collect();
            for (i, expected_key) in canonical_order.iter().enumerate() {
                match &got[i].0 {
                    CborValue::Unsigned(k) => assert_eq!(*k, *expected_key),
                    _ => panic!("expected unsigned key"),
                }
            }
        } else {
            panic!("expected map");
        }
    }

    // --- Major-7 boundary (simple values) ---

    #[test]
    fn simple_values_roundtrip() {
        for v in [CborValue::Bool(false), CborValue::Bool(true), CborValue::Null] {
            let bytes = encode(&v);
            let back = decode(&bytes).unwrap();
            assert_eq!(back, v);
        }
    }

    // --- DoS defences (security-review Round 1 fixes) ---

    #[test]
    fn decode_rejects_deeply_nested_arrays() {
        // 0x81 = array of length 1. Chain MAX_DECODE_DEPTH+10 of them
        // followed by a single Unsigned(0). The decoder must reject
        // before recursing into stack overflow territory.
        let depth = MAX_DECODE_DEPTH + 10;
        let mut bytes = vec![0x81u8; depth];
        bytes.push(0x00);
        match decode(&bytes) {
            Err(CborError::TooDeep) => {}
            other => panic!("expected TooDeep, got {:?}", other),
        }
    }

    #[test]
    fn decode_rejects_deeply_nested_tags() {
        // 0xC6 = tag 6. Same idea — small payload, big recursion.
        let depth = MAX_DECODE_DEPTH + 10;
        let mut bytes = vec![0xC6u8; depth];
        bytes.push(0x00);
        match decode(&bytes) {
            Err(CborError::TooDeep) => {}
            other => panic!("expected TooDeep, got {:?}", other),
        }
    }

    #[test]
    fn decode_accepts_nesting_at_the_limit() {
        // Build exactly MAX_DECODE_DEPTH levels — should succeed.
        // Note: `depth=0` corresponds to the outermost call, so
        // MAX_DECODE_DEPTH levels means MAX_DECODE_DEPTH wrapper bytes
        // before the leaf.
        let mut bytes = vec![0x81u8; MAX_DECODE_DEPTH];
        bytes.push(0x00);
        let v = decode(&bytes).unwrap();
        // Sanity check: we decoded *something*.
        let _ = v;
    }

    #[test]
    fn decode_rejects_array_with_oversized_length() {
        // 0x9B + 8-byte payload 0x00 00 01 00 00 00 00 00 = 2^40 = 1 TiB
        // worth of declared elements. Comfortably above the
        // smallest-form threshold (so passes that gate) and
        // comfortably above any realistic remaining-bytes count.
        let bytes = vec![
            0x9B, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ];
        match decode(&bytes) {
            Err(CborError::LengthTooLarge) => {}
            other => panic!("expected LengthTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn decode_rejects_map_with_oversized_length() {
        let bytes = vec![
            0xBB, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        match decode(&bytes) {
            Err(CborError::LengthTooLarge) => {}
            other => panic!("expected LengthTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn decode_rejects_byte_string_with_oversized_length() {
        let bytes = vec![0x5B, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
        match decode(&bytes) {
            Err(CborError::LengthTooLarge) => {}
            other => panic!("expected LengthTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn decode_rejects_max_u64_length() {
        // Array claiming u64::MAX elements, no payload.
        let mut bytes = vec![0x9B];
        bytes.extend_from_slice(&u64::MAX.to_be_bytes());
        match decode(&bytes) {
            Err(CborError::LengthTooLarge) => {}
            // Could also be UnexpectedEof on a 32-bit target if
            // try_from succeeds — but on every supported platform
            // Vec<CborValue> capacity * size is bigger than memory,
            // so length_within_remaining catches it first.
            other => panic!("expected LengthTooLarge, got {:?}", other),
        }
    }

    // --- Display strings come from literals ---

    #[test]
    fn error_messages_are_static() {
        // Smoke: each variant's Display output mentions
        // "canonical-cbor:" prefix and contains a fixed substring.
        let err_msgs: Vec<String> = [
            CborError::UnexpectedEof,
            CborError::TrailingBytes,
            CborError::Reserved,
            CborError::Indefinite,
            CborError::NonMinimalInteger,
            CborError::InvalidUtf8,
            CborError::NonCanonicalMapOrder,
            CborError::UnsupportedSimple,
            CborError::FloatNotSupported,
            CborError::TooDeep,
            CborError::LengthTooLarge,
        ]
        .iter()
        .map(|e| e.to_string())
        .collect();
        for msg in &err_msgs {
            assert!(msg.starts_with("canonical-cbor:"));
        }
    }
}
