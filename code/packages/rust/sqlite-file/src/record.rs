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

/// The minimal big-endian signed-integer width (in bytes) that can hold `v`,
/// mapped to the serial type and byte count SQLite would choose. Returns
/// `(serial_type, width_bytes)`. Callers handle 0 and 1 separately (serial 8/9,
/// zero payload) *before* calling this.
///
/// SQLite always picks the SHORTEST width — a byte-compatible writer must too,
/// or the record won't match what `sqlite3` produced for the same row.
fn int_serial(v: i64) -> (u64, usize) {
    // The signed ranges for each on-disk width. `1i64 << (bits-1)` is the
    // magnitude of the most-negative value of a `bits`-wide two's-complement int.
    const I8: i64 = 1 << 7;
    const I16: i64 = 1 << 15;
    const I24: i64 = 1 << 23;
    const I32: i64 = 1 << 31;
    const I48: i64 = 1 << 47;
    if (-I8..I8).contains(&v) {
        (1, 1)
    } else if (-I16..I16).contains(&v) {
        (2, 2)
    } else if (-I24..I24).contains(&v) {
        (3, 3)
    } else if (-I32..I32).contains(&v) {
        (4, 4)
    } else if (-I48..I48).contains(&v) {
        (5, 6)
    } else {
        (6, 8)
    }
}

/// Append the low `width` bytes of `v` in big-endian order (two's complement).
/// `width` is one of 1/2/3/4/6/8 as chosen by [`int_serial`], so the discarded
/// high bytes are pure sign extension and the value round-trips through
/// [`read_int_be`].
fn write_int_be(v: i64, width: usize, out: &mut Vec<u8>) {
    let bytes = (v as u64).to_be_bytes(); // 8 bytes, most-significant first
    out.extend_from_slice(&bytes[8 - width..]);
}

/// The serial type and payload bytes for one column value — the inverse of
/// [`decode_value`].
fn value_serial_and_payload(value: &SqlValue) -> (u64, Vec<u8>) {
    match value {
        SqlValue::Null => (0, Vec::new()),
        SqlValue::Int(0) => (8, Vec::new()), // value carried inline by the type
        SqlValue::Int(1) => (9, Vec::new()),
        SqlValue::Int(i) => {
            let (serial, width) = int_serial(*i);
            let mut payload = Vec::with_capacity(width);
            write_int_be(*i, width, &mut payload);
            (serial, payload)
        }
        SqlValue::Real(f) => (7, f.to_be_bytes().to_vec()),
        // Text: odd serial ≥ 13, length = (N-13)/2, so N = 13 + 2·len.
        SqlValue::Text(s) => (13 + 2 * s.len() as u64, s.as_bytes().to_vec()),
        // Blob: even serial ≥ 12, length = (N-12)/2, so N = 12 + 2·len.
        SqlValue::Blob(b) => (12 + 2 * b.len() as u64, b.clone()),
    }
}

/// Encode a row of column values into one SQLite record (header + payload) —
/// the inverse of [`decode`]. The bytes are byte-for-byte what SQLite writes for
/// the same row, so `decode(encode(row)) == row` and a produced record slots
/// straight into a table b-tree leaf cell.
///
/// The header-length varint counts itself plus every serial-type varint. Its own
/// byte-length depends on the total, which depends on its byte-length — a small
/// self-reference resolved by trying header-varint widths 1, 2, … until the
/// declared length is consistent with the width needed to encode it.
pub fn encode(values: &[SqlValue]) -> Vec<u8> {
    // Serial-type varints and the payload are independent of the header length.
    let mut serial_varints = Vec::new();
    let mut payload = Vec::new();
    for value in values {
        let (serial, bytes) = value_serial_and_payload(value);
        varint::write(serial as i64, &mut serial_varints);
        payload.extend_from_slice(&bytes);
    }

    // Resolve the self-referential header length. `body` is everything in the
    // header after the length varint; the length counts the length varint too.
    let body = serial_varints.len();
    let mut header_varint_len = 1;
    loop {
        let header_len = header_varint_len + body;
        // How many bytes does it take to actually encode this header_len?
        let mut probe = Vec::new();
        let actual = varint::write(header_len as i64, &mut probe);
        if actual == header_varint_len {
            let mut record = probe; // the header-length varint
            record.extend_from_slice(&serial_varints);
            record.extend_from_slice(&payload);
            return record;
        }
        // The length didn't fit in the assumed width — grow and retry. This
        // terminates in at most a couple of iterations (varints are ≤ 9 bytes).
        header_varint_len = actual;
    }
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

    // ── Encoder (Phase F writer groundwork) ──────────────────────────────────

    #[test]
    fn encode_matches_the_golden_decode_vectors_byte_for_byte() {
        // The exact bytes the decode tests above assert on — the encoder must
        // reproduce them (this is what "byte-compatible with SQLite" means).
        assert_eq!(
            encode(&[
                SqlValue::Null,
                SqlValue::Int(42),
                SqlValue::Text("hi".to_string())
            ]),
            vec![0x04, 0x00, 0x01, 0x11, 0x2a, 0x68, 0x69]
        );
        // [0, 1, 1.5] → serial types 8, 9, 7.
        assert_eq!(
            encode(&[SqlValue::Int(0), SqlValue::Int(1), SqlValue::Real(1.5)]),
            vec![0x04, 0x08, 0x09, 0x07, 0x3f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // Small negatives use the MINIMAL 8-bit width (serial 1) — SQLite's
        // decoder accepts a wider encoding (the decode test uses 16-bit `-2` to
        // exercise sign-extension), but the writer always picks the shortest.
        assert_eq!(encode(&[SqlValue::Int(-2)]), vec![0x02, 0x01, 0xfe]);
        assert_eq!(encode(&[SqlValue::Int(-1)]), vec![0x02, 0x01, 0xff]);
        // [x'DEAD'] → serial 16.
        assert_eq!(
            encode(&[SqlValue::Blob(vec![0xde, 0xad])]),
            vec![0x02, 0x10, 0xde, 0xad]
        );
    }

    #[test]
    fn encoder_picks_the_minimal_integer_width() {
        // 127 fits in 8 bits (serial 1); 128 needs 16 (serial 2). The header is
        // 2 bytes in both (length varint + one serial-type varint).
        assert_eq!(encode(&[SqlValue::Int(127)]), vec![0x02, 0x01, 0x7f]);
        assert_eq!(encode(&[SqlValue::Int(128)]), vec![0x02, 0x02, 0x00, 0x80]);
        // 65536 needs 24 bits (serial 3); 2^32 fits in 48 bits (serial 5, six
        // payload bytes) — SQLite reserves the 64-bit serial 6 for values that
        // actually need it (|v| ≥ 2^47).
        assert_eq!(
            encode(&[SqlValue::Int(65536)]),
            vec![0x02, 0x03, 0x01, 0x00, 0x00]
        );
        assert_eq!(
            encode(&[SqlValue::Int(1 << 32)]),
            vec![0x02, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]
        );
        // A value that truly needs 64 bits (2^47) uses serial 6.
        assert_eq!(
            encode(&[SqlValue::Int(1 << 47)]),
            vec![0x02, 0x06, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn decode_of_encode_is_identity_over_a_wide_sweep() {
        // Deterministic LCG — no rng dependency (zero-dep crate). Build assorted
        // rows mixing every storage class and integer width, and assert the
        // record round-trips through the encoder and decoder unchanged.
        let mut state: u64 = 0x0f0e_0d0c_0b0a_0908;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            state
        };
        for _ in 0..20_000 {
            let row = vec![
                SqlValue::Null,
                SqlValue::Int(next() as i64),
                SqlValue::Int((next() % 4) as i64), // hits 0/1 inline-value types
                SqlValue::Real(f64::from_bits(next())),
                {
                    let len = (next() % 6) as usize;
                    SqlValue::Text((0..len).map(|_| (b'a' + (next() % 26) as u8) as char).collect())
                },
                {
                    let len = (next() % 6) as usize;
                    SqlValue::Blob((0..len).map(|_| next() as u8).collect())
                },
            ];
            let encoded = encode(&row);
            let decoded = decode(&encoded).expect("round-trip decodes");
            // NaN != NaN, so compare the float column by bit pattern.
            for (a, b) in row.iter().zip(decoded.iter()) {
                match (a, b) {
                    (SqlValue::Real(x), SqlValue::Real(y)) => {
                        assert_eq!(x.to_bits(), y.to_bits(), "real round-trip")
                    }
                    _ => assert_eq!(a, b, "column round-trip"),
                }
            }
        }
    }

    #[test]
    fn encodes_a_large_header_that_needs_a_two_byte_length_varint() {
        // A row with enough columns that the header length itself exceeds 127,
        // forcing the self-referential header-length varint to widen to 2 bytes.
        // 130 single-byte NULL serial types → body = 130, header_len = 132.
        let row = vec![SqlValue::Null; 130];
        let encoded = encode(&row);
        // Header length 132 encodes as the 2-byte varint 0x81 0x04.
        assert_eq!(&encoded[..2], &[0x81, 0x04]);
        assert_eq!(decode(&encoded).unwrap(), row);
    }
}
