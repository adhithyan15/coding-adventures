//! # protobuf — a zero-dependency Protocol Buffers *wire-format* codec
//!
//! This crate implements just the **wire format** of Protocol Buffers
//! (<https://protobuf.dev/programming-guides/encoding/>) — enough to encode and
//! decode messages byte-for-byte compatibly with Google's protobuf and any
//! conforming implementation (e.g. the one Anki uses inside `.apkg` files).
//! It deliberately does **not** include a `.proto` compiler or a `derive`
//! macro: callers hand-write the handful of `encode`/`decode` functions for the
//! specific messages they need. For a two-or-three-field message that is a few
//! lines, and it keeps this crate tiny and dependency-free.
//!
//! ## The wire format in one paragraph
//!
//! A protobuf message is a flat sequence of `(field)` records with no framing,
//! length prefix, or field ordering guarantees. Each record begins with a
//! **tag** — a varint whose low 3 bits are the *wire type* and whose remaining
//! bits are the *field number*:
//!
//! ```text
//!   tag = (field_number << 3) | wire_type
//! ```
//!
//! The wire type tells the reader how to read the value that follows, so a
//! reader can skip fields it does not recognise (forward compatibility):
//!
//! | wire type | name             | payload                                   |
//! |-----------|------------------|-------------------------------------------|
//! | 0         | `Varint`         | one LEB128 varint (ints, bools, enums)    |
//! | 1         | `Fixed64`        | 8 little-endian bytes (`fixed64`/`double`)|
//! | 2         | `LengthDelimited`| a varint length `n`, then `n` bytes       |
//! | 5         | `Fixed32`        | 4 little-endian bytes (`fixed32`/`float`) |
//!
//! `LengthDelimited` (2) carries `string`, `bytes`, *and* embedded messages —
//! they are all "a length then that many bytes"; the difference is only in how
//! the caller interprets the bytes.
//!
//! ## Varints (LEB128, unsigned)
//!
//! A varint stores an unsigned integer 7 bits at a time, little-endian, with
//! the top bit of each byte set to 1 while more bytes follow and 0 on the last
//! byte. `300` (`0b1_0010_1100`) encodes as `[0xAC, 0x02]`:
//!
//! ```text
//!   0xAC = 1010_1100  continuation=1, low 7 bits  = 010_1100
//!   0x02 = 0000_0010  continuation=0, next 7 bits = 000_0010
//!   value = (0b000_0010 << 7) | 0b010_1100 = 300
//! ```
//!
//! ## Defaults
//!
//! By proto3 convention, scalar fields equal to their zero value are usually
//! *omitted* on the wire, and a reader treats a missing field as that zero
//! value. This crate does not impose that policy: [`Writer`] writes exactly the
//! fields you ask it to (so you can match a specific producer's bytes), and a
//! decoder should initialise its output to defaults and overwrite per field.

#![forbid(unsafe_code)]

/// Protobuf wire types (the low 3 bits of a field tag).
///
/// Only the four defined by the current spec are represented; the deprecated
/// group types (3, 4) are rejected as malformed, which is correct for any
/// modern producer including Anki.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    /// LEB128 varint — `int32/64`, `uint32/64`, `sint*`, `bool`, `enum`.
    Varint = 0,
    /// 8 little-endian bytes — `fixed64`, `sfixed64`, `double`.
    Fixed64 = 1,
    /// varint length then that many bytes — `string`, `bytes`, message.
    LengthDelimited = 2,
    /// 4 little-endian bytes — `fixed32`, `sfixed32`, `float`.
    Fixed32 = 5,
}

impl WireType {
    fn from_bits(bits: u64) -> Result<WireType, Error> {
        match bits {
            0 => Ok(WireType::Varint),
            1 => Ok(WireType::Fixed64),
            2 => Ok(WireType::LengthDelimited),
            5 => Ok(WireType::Fixed32),
            other => Err(Error::UnknownWireType(other as u8)),
        }
    }
}

/// A decode error. Encoding cannot fail, so there is no encode error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A varint ran past the end of the buffer, or exceeded 10 bytes (the
    /// maximum for a 64-bit value — a longer one would overflow).
    TruncatedVarint,
    /// A length-delimited / fixed field claimed more bytes than remain.
    UnexpectedEof,
    /// The tag carried a wire type this codec does not implement (3, 4, 6, 7).
    UnknownWireType(u8),
    /// A field number of zero, which protobuf forbids.
    ZeroFieldNumber,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::TruncatedVarint => write!(f, "truncated or over-long varint"),
            Error::UnexpectedEof => write!(f, "unexpected end of protobuf buffer"),
            Error::UnknownWireType(w) => write!(f, "unknown protobuf wire type {w}"),
            Error::ZeroFieldNumber => write!(f, "protobuf field number 0 is illegal"),
        }
    }
}

impl std::error::Error for Error {}

// ===========================================================================
// Writer
// ===========================================================================

/// Builds a protobuf message by appending fields, in the order you call them.
///
/// The writer never inserts framing of its own — the output is exactly the
/// concatenation of the fields written, which is a complete protobuf message.
/// Nest messages by building the inner message into its own `Writer`, then
/// writing its bytes with [`Writer::message`].
#[derive(Default, Debug, Clone)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    /// A new, empty message writer.
    pub fn new() -> Self {
        Writer { buf: Vec::new() }
    }

    /// Consume the writer and return the encoded message bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Append a raw LEB128 varint (no tag). Used internally; public for callers
    /// who need to hand-assemble a nested length prefix.
    pub fn write_varint(&mut self, mut value: u64) {
        // Emit 7 bits per byte, continuation flag on all but the last.
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(byte);
                break;
            }
            self.buf.push(byte | 0x80);
        }
    }

    fn write_tag(&mut self, field: u32, wire: WireType) {
        self.write_varint(((field as u64) << 3) | (wire as u64));
    }

    /// A varint-typed field (`int32/64`, `uint32/64`, `bool`, `enum`).
    pub fn varint(&mut self, field: u32, value: u64) -> &mut Self {
        self.write_tag(field, WireType::Varint);
        self.write_varint(value);
        self
    }

    /// A length-delimited field carrying arbitrary `bytes`.
    pub fn bytes(&mut self, field: u32, value: &[u8]) -> &mut Self {
        self.write_tag(field, WireType::LengthDelimited);
        self.write_varint(value.len() as u64);
        self.buf.extend_from_slice(value);
        self
    }

    /// A length-delimited `string` field (UTF-8 bytes).
    pub fn string(&mut self, field: u32, value: &str) -> &mut Self {
        self.bytes(field, value.as_bytes())
    }

    /// A length-delimited field carrying an embedded message (already encoded).
    pub fn message(&mut self, field: u32, encoded: &[u8]) -> &mut Self {
        self.bytes(field, encoded)
    }

    /// A `fixed32` / `sfixed32` / `float` field (4 little-endian bytes).
    pub fn fixed32(&mut self, field: u32, value: u32) -> &mut Self {
        self.write_tag(field, WireType::Fixed32);
        self.buf.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// A `fixed64` / `sfixed64` / `double` field (8 little-endian bytes).
    pub fn fixed64(&mut self, field: u32, value: u64) -> &mut Self {
        self.write_tag(field, WireType::Fixed64);
        self.buf.extend_from_slice(&value.to_le_bytes());
        self
    }
}

// ===========================================================================
// Reader
// ===========================================================================

/// One decoded field: its number and its value, borrowing from the input.
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    /// The 1-based field number from the tag.
    pub number: u32,
    /// The value, already read according to the wire type.
    pub value: Value<'a>,
}

/// A field value, tagged by wire type. Length-delimited payloads borrow the
/// input slice so no copying happens until the caller decides to.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// A varint — interpret as the integer/bool/enum the schema expects.
    Varint(u64),
    /// 8 raw little-endian bytes as a `u64`.
    Fixed64(u64),
    /// A borrowed slice — `string`, `bytes`, or a nested message.
    Bytes(&'a [u8]),
    /// 4 raw little-endian bytes as a `u32`.
    Fixed32(u32),
}

impl<'a> Value<'a> {
    /// The varint payload, or an error if this field was not a varint.
    pub fn as_varint(&self) -> Option<u64> {
        match self {
            Value::Varint(v) => Some(*v),
            _ => None,
        }
    }

    /// The length-delimited payload, or `None` for other wire types.
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::Bytes(b) => Some(*b),
            _ => None,
        }
    }
}

/// A cursor over a protobuf message. Iterate fields with [`Reader::next_field`];
/// unknown field numbers are yielded too, so the caller can ignore them (this
/// is how protobuf stays forward-compatible).
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// A reader over an encoded message.
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// Whether every field has been consumed.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn read_varint(&mut self) -> Result<u64, Error> {
        let mut result: u64 = 0;
        // A u64 needs at most ceil(64/7) = 10 varint bytes; more means overflow.
        for shift in (0..64).step_by(7) {
            let byte = *self.data.get(self.pos).ok_or(Error::TruncatedVarint)?;
            self.pos += 1;
            result |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }
        Err(Error::TruncatedVarint)
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(len).ok_or(Error::UnexpectedEof)?;
        if end > self.data.len() {
            return Err(Error::UnexpectedEof);
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Read the next field, or `Ok(None)` at end of message.
    pub fn next_field(&mut self) -> Result<Option<Field<'a>>, Error> {
        if self.is_empty() {
            return Ok(None);
        }
        let tag = self.read_varint()?;
        let number = (tag >> 3) as u32;
        if number == 0 {
            return Err(Error::ZeroFieldNumber);
        }
        let value = match WireType::from_bits(tag & 0x7)? {
            WireType::Varint => Value::Varint(self.read_varint()?),
            WireType::Fixed64 => {
                let b = self.read_slice(8)?;
                Value::Fixed64(u64::from_le_bytes(b.try_into().unwrap()))
            }
            WireType::LengthDelimited => {
                let len = self.read_varint()? as usize;
                Value::Bytes(self.read_slice(len)?)
            }
            WireType::Fixed32 => {
                let b = self.read_slice(4)?;
                Value::Fixed32(u32::from_le_bytes(b.try_into().unwrap()))
            }
        };
        Ok(Some(Field { number, value }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip_boundaries() {
        for v in [0u64, 1, 127, 128, 300, 16383, 16384, u32::MAX as u64, u64::MAX] {
            let mut w = Writer::new();
            w.write_varint(v);
            let bytes = w.into_bytes();
            let mut r = Reader::new(&bytes);
            assert_eq!(r.read_varint().unwrap(), v, "varint {v} round-trip");
            assert!(r.is_empty());
        }
    }

    #[test]
    fn varint_300_matches_spec_bytes() {
        // The canonical example from the protobuf docs.
        let mut w = Writer::new();
        w.write_varint(300);
        assert_eq!(w.into_bytes(), vec![0xac, 0x02]);
    }

    #[test]
    fn field_roundtrip_all_wire_types() {
        let mut w = Writer::new();
        w.varint(1, 150)
            .string(2, "testing")
            .bytes(3, &[0xde, 0xad, 0xbe, 0xef])
            .fixed32(4, 0x1234_5678)
            .fixed64(5, 0x0102_0304_0506_0708);
        let encoded = w.into_bytes();

        let mut r = Reader::new(&encoded);
        assert_eq!(r.next_field().unwrap().unwrap(), Field { number: 1, value: Value::Varint(150) });
        assert_eq!(r.next_field().unwrap().unwrap(), Field { number: 2, value: Value::Bytes(b"testing") });
        assert_eq!(r.next_field().unwrap().unwrap(), Field { number: 3, value: Value::Bytes(&[0xde, 0xad, 0xbe, 0xef]) });
        assert_eq!(r.next_field().unwrap().unwrap(), Field { number: 4, value: Value::Fixed32(0x1234_5678) });
        assert_eq!(r.next_field().unwrap().unwrap(), Field { number: 5, value: Value::Fixed64(0x0102_0304_0506_0708) });
        assert!(r.next_field().unwrap().is_none());
    }

    #[test]
    fn field_1_varint_150_matches_spec_bytes() {
        // protobuf docs: field 1, varint 150 → tag 0x08, then 0x96 0x01.
        let mut w = Writer::new();
        w.varint(1, 150);
        assert_eq!(w.into_bytes(), vec![0x08, 0x96, 0x01]);
    }

    #[test]
    fn reader_skips_unknown_fields() {
        // A message with a field (7) the "schema" ignores, between two it wants.
        let mut w = Writer::new();
        w.varint(1, 11).varint(7, 999).string(2, "keep");
        let encoded = w.into_bytes();

        // Consume all, keeping only 1 and 2 — proves unknown-field skipping.
        let mut r = Reader::new(&encoded);
        let mut kept = Vec::new();
        while let Some(f) = r.next_field().unwrap() {
            if f.number == 1 || f.number == 2 {
                kept.push(f.number);
            }
        }
        assert_eq!(kept, vec![1, 2]);
    }

    #[test]
    fn nested_message_roundtrip() {
        let inner = {
            let mut w = Writer::new();
            w.string(1, "inner");
            w.into_bytes()
        };
        let mut outer = Writer::new();
        outer.message(1, &inner).varint(2, 5);
        let encoded = outer.into_bytes();

        let mut r = Reader::new(&encoded);
        let f = r.next_field().unwrap().unwrap();
        assert_eq!(f.number, 1);
        let mut inner_r = Reader::new(f.value.as_bytes().unwrap());
        assert_eq!(
            inner_r.next_field().unwrap().unwrap().value,
            Value::Bytes(b"inner")
        );
    }

    #[test]
    fn truncated_varint_errors() {
        // A continuation bit set but no following byte.
        let mut r = Reader::new(&[0x80]);
        assert_eq!(r.next_field(), Err(Error::TruncatedVarint));
    }

    #[test]
    fn overlong_length_errors() {
        // Field 1, length-delimited, claims 100 bytes but supplies none.
        let encoded = vec![0x0a, 0x64];
        let mut r = Reader::new(&encoded);
        assert_eq!(r.next_field(), Err(Error::UnexpectedEof));
    }
}
