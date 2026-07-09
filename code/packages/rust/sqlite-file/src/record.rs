//! # SQLite record format (rows as bytes)
//!
//! Every row SQLite stores — a table row, a `sqlite_schema` entry — is a
//! **record**: a self-describing sequence of column values. This module decodes
//! one record's bytes into a `Vec<SqlValue>`. (The *encoder* is a separate,
//! larger concern — writing byte-identical records is Phase F. This crate is a
//! reader, so it only needs to decode.)
//!
//! ## Anatomy of a record
//!
//! ```text
//!   ┌───────────────┬───────────────────────────┬──────────────────────┐
//!   │ header length │ serial type per column     │ column payloads      │
//!   │  (varint)     │  (varint each)             │  (back-to-back)      │
//!   └───────────────┴───────────────────────────┴──────────────────────┘
//!   └────────────── header ─────────────────────┘
//! ```
//!
//! The **header length** varint counts itself plus all the serial-type varints,
//! so the payload begins exactly `header_length` bytes into the record. We read
//! serial types until we reach that boundary, then peel each column's bytes off
//! the payload in order.
//!
//! ## Serial types — the type/size table
//!
//! A column's *serial type* varint says both what the value is and how many
//! payload bytes it occupies:
//!
//! | serial type   | value                 | payload bytes    |
//! |---------------|-----------------------|------------------|
//! | 0             | NULL                  | 0                |
//! | 1             | signed 8-bit int      | 1                |
//! | 2             | signed 16-bit int BE  | 2                |
//! | 3             | signed 24-bit int BE  | 3                |
//! | 4             | signed 32-bit int BE  | 4                |
//! | 5             | signed 48-bit int BE  | 6                |
//! | 6             | signed 64-bit int BE  | 8                |
//! | 7             | IEEE-754 f64 BE       | 8                |
//! | 8             | integer 0             | 0 (value inline) |
//! | 9             | integer 1             | 0 (value inline) |
//! | 10, 11        | reserved (unused)     | —                |
//! | N ≥ 12, even  | BLOB, `(N−12)/2` long | `(N−12)/2`       |
//! | N ≥ 13, odd   | TEXT, `(N−13)/2` long | `(N−13)/2`       |
//!
//! Serial types 8 and 9 are a neat trick: the integers 0 and 1 are so common
//! (booleans, flags) that SQLite spends *zero* payload bytes on them — the value
//! lives entirely in the serial type.

use crate::varint;

/// A single decoded column value — the five storage classes SQLite has.
#[derive(Clone, Debug, PartialEq)]
pub enum SqlValue {
    /// SQL `NULL`.
    Null,
    /// An integer (serial types 1–6, 8, 9), always widened to `i64`.
    Int(i64),
    /// An IEEE-754 double (serial type 7).
    Real(f64),
    /// UTF-8 text (odd serial types ≥ 13). SQLite databases we read use the
    /// UTF-8 encoding (header text-encoding = 1), so the bytes are decoded as
    /// UTF-8; any stray invalid byte is replaced rather than rejected, matching
    /// how `rusqlite`/`sqlite3` hand back text.
    Text(String),
    /// Opaque bytes (even serial types ≥ 12).
    Blob(Vec<u8>),
}

/// How many payload bytes a column of the given `serial` type occupies.
fn content_size(serial: u64) -> usize {
    match serial {
        0 | 8 | 9 | 10 | 11 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 6,
        6 | 7 => 8,
        // For N ≥ 12, integer division `(N-12)/2` yields the byte length for
        // both BLOB (even) and TEXT (odd): e.g. 14→1, 15→1, 16→2, 17→2.
        n => ((n - 12) / 2) as usize,
    }
}

/// Read a big-endian, two's-complement signed integer out of `bytes`
/// (1, 2, 3, 4, 6, or 8 bytes) and widen it to `i64`, sign-extending from the
/// top bit of the most-significant byte.
fn read_int_be(bytes: &[u8]) -> i64 {
    let mut v: u64 = 0;
    for &b in bytes {
        v = (v << 8) | u64::from(b);
    }
    let bits = bytes.len() * 8;
    // Sign-extend: if the value's top bit is set and it is narrower than 64
    // bits, fill the high bits with ones so the `i64` keeps the same value.
    if bits < 64 && (v & (1u64 << (bits - 1))) != 0 {
        v |= !((1u64 << bits) - 1);
    }
    v as i64
}

/// Turn one `(serial type, its payload bytes)` pair into a [`SqlValue`].
/// Returns `None` only for the reserved serial types 10/11 (which never occur
/// in a well-formed file) or a truncated float.
fn decode_value(serial: u64, content: &[u8]) -> Option<SqlValue> {
    let value = match serial {
        0 => SqlValue::Null,
        1..=6 => SqlValue::Int(read_int_be(content)),
        7 => {
            let arr: [u8; 8] = content.try_into().ok()?;
            SqlValue::Real(f64::from_be_bytes(arr))
        }
        8 => SqlValue::Int(0),
        9 => SqlValue::Int(1),
        10 | 11 => return None, // reserved — corrupt file
        n if n % 2 == 0 => SqlValue::Blob(content.to_vec()),
        _ => SqlValue::Text(String::from_utf8_lossy(content).into_owned()),
    };
    Some(value)
}

/// Decode a complete record (header + payload) into its column values.
///
/// Returns `None` on any inconsistency — a header that overruns the record, a
/// truncated payload, a reserved serial type — so a corrupt or maliciously
/// crafted database surfaces an error to the caller instead of panicking or
/// reading out of bounds.
pub fn decode(record: &[u8]) -> Option<Vec<SqlValue>> {
    let (header_len, mut header_off) = varint::read(record)?;
    let header_len = usize::try_from(header_len).ok()?;
    if header_len > record.len() {
        return None;
    }

    let mut values = Vec::new();
    let mut payload_off = header_len;
    // Walk serial types until we hit the payload boundary...
    while header_off < header_len {
        let (serial, n) = varint::read(record.get(header_off..)?)?;
        header_off += n;
        let serial = u64::try_from(serial).ok()?;
        let size = content_size(serial);
        // ...peeling `size` payload bytes off for each, bounds-checked.
        let end = payload_off.checked_add(size)?;
        let content = record.get(payload_off..end)?;
        payload_off = end;
        values.push(decode_value(serial, content)?);
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_null_int_text_row() {
        // Row [NULL, 42, "hi"]:
        //   serial types 0, 1, 17(=13+2·2)  →  header = [len=4, 0, 1, 17]
        //   payload      (none), 0x2a, "hi"
        let record = [0x04, 0x00, 0x01, 0x11, 0x2a, 0x68, 0x69];
        assert_eq!(
            decode(&record).unwrap(),
            vec![
                SqlValue::Null,
                SqlValue::Int(42),
                SqlValue::Text("hi".to_string())
            ]
        );
    }

    #[test]
    fn zero_and_one_carry_no_payload_bytes() {
        // Row [0, 1, 1.5]: serial types 8, 9, 7. The 0 and 1 use no payload;
        // the float is eight big-endian bytes (0x3ff8_0000_0000_0000 = 1.5).
        let record = [
            0x04, 0x08, 0x09, 0x07, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            decode(&record).unwrap(),
            vec![SqlValue::Int(0), SqlValue::Int(1), SqlValue::Real(1.5)]
        );
    }

    #[test]
    fn negative_integers_sign_extend() {
        // Row [-2] as a 16-bit int (serial type 2): payload 0xff 0xfe.
        let record = [0x02, 0x02, 0xff, 0xfe];
        assert_eq!(decode(&record).unwrap(), vec![SqlValue::Int(-2)]);

        // Row [-1] as an 8-bit int (serial type 1): payload 0xff.
        let record = [0x02, 0x01, 0xff];
        assert_eq!(decode(&record).unwrap(), vec![SqlValue::Int(-1)]);
    }

    #[test]
    fn decodes_blob() {
        // Row [x'DEAD'] — a 2-byte blob is serial type 12 + 2·2 = 16.
        let record = [0x02, 0x10, 0xde, 0xad];
        assert_eq!(
            decode(&record).unwrap(),
            vec![SqlValue::Blob(vec![0xde, 0xad])]
        );
    }

    #[test]
    fn wide_integer_widths_round_out_the_table() {
        // 24-bit (serial 3): 0x010000 = 65536.
        assert_eq!(
            decode(&[0x02, 0x03, 0x01, 0x00, 0x00]).unwrap(),
            vec![SqlValue::Int(65536)]
        );
        // 64-bit (serial 6): 0x0000_0001_0000_0000 = 2^32.
        assert_eq!(
            decode(&[0x02, 0x06, 0, 0, 0, 1, 0, 0, 0, 0]).unwrap(),
            vec![SqlValue::Int(1 << 32)]
        );
    }

    #[test]
    fn corrupt_records_return_none_not_panic() {
        // Header claims 4 bytes but the record is only 1 long.
        assert_eq!(decode(&[0x04]), None);
        // Serial type 6 (needs 8 payload bytes) but payload is short.
        assert_eq!(decode(&[0x02, 0x06, 0x00]), None);
        // Reserved serial type 10.
        assert_eq!(decode(&[0x02, 0x0a]), None);
    }
}
