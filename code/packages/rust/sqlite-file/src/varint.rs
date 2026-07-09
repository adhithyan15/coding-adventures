//! # SQLite variable-length integers ("varints")
//!
//! A *varint* is how SQLite squeezes a 64-bit integer into as few bytes as it
//! can. It appears everywhere in the file format — record header lengths,
//! serial types, rowids, b-tree child pointers — so getting it exactly right is
//! the foundation everything else stands on.
//!
//! ## The encoding, one byte at a time
//!
//! A varint is **big-endian, base-128**, 1 to 9 bytes long. For the first eight
//! bytes, the high bit (`0x80`) is a *continuation flag* and the low seven bits
//! (`0x7f`) are payload:
//!
//! ```text
//!   byte:  1ppppppp  1ppppppp  0ppppppp        (high bit set ⇒ "more follows")
//!          └── keep reading ──┘ └ last byte ┘   (high bit clear ⇒ "stop")
//! ```
//!
//! The ninth byte is special: if we have already read eight continuation bytes
//! (7 bits × 8 = 56 bits) we still need 8 more bits to reach a full 64, so the
//! **ninth byte contributes all eight of its bits**, not seven.
//!
//! ## Worked examples
//!
//! | value        | bytes (hex)        | why                                   |
//! |--------------|--------------------|---------------------------------------|
//! | `0`          | `00`               | fits in 7 bits, high bit clear        |
//! | `127`        | `7f`               | largest 1-byte varint                 |
//! | `128`        | `81 00`            | `1` in the high group, `0` in the low |
//! | `300`        | `82 2c`            | `0b10_0101100` split 7+7              |
//! | `2^64 - 1`   | `ff ff … ff` (9)   | all nine bytes, last uses 8 bits      |
//!
//! ## Signedness
//!
//! On disk a varint carries a raw 64-bit pattern. SQLite *interprets* it as a
//! signed two's-complement `i64` (rowids and the small-int serial types are
//! signed). We therefore decode into a `u64` bit pattern and hand back an `i64`
//! via a bit cast — the caller decides whether the value is a length (always
//! non-negative in practice) or a signed integer.

/// Read a varint from the front of `buf`.
///
/// Returns `(value, len)` where `value` is the decoded integer (as the raw
/// two's-complement `i64`) and `len` is how many bytes were consumed (1..=9).
/// Returns `None` if `buf` ends before the varint does — a truncated /
/// corrupt file, which the reader surfaces rather than panicking on.
pub fn read(buf: &[u8]) -> Option<(i64, usize)> {
    let mut result: u64 = 0;
    // Bytes 1..=8 carry seven payload bits each behind a continuation flag.
    for i in 0..8 {
        let byte = *buf.get(i)?;
        result = (result << 7) | u64::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Some((result as i64, i + 1));
        }
    }
    // Byte 9 (index 8) is reached only when the first eight all had the
    // continuation bit set; it donates all eight of its bits to fill out 64.
    let ninth = *buf.get(8)?;
    result = (result << 8) | u64::from(ninth);
    Some((result as i64, 9))
}

/// Encode `value` (a raw two's-complement `i64`) into its minimal varint form,
/// appending the bytes to `out`. Returns the number of bytes written.
///
/// "Minimal" is not cosmetic: SQLite always uses the shortest encoding, so to
/// round-trip a file byte-for-byte (Phase F, the writer) we must too. The
/// reader ships this alongside `read` so the two can be property-tested against
/// each other — every value must survive `read(write(v)) == v`.
pub fn write(value: i64, out: &mut Vec<u8>) -> usize {
    let v = value as u64;

    // How many of the low bits are actually significant? A varint of the first
    // eight bytes holds 7 bits each (max 56 bits); if the value needs a 57th
    // bit we must fall back to the full nine-byte form.
    if v > 0x00ff_ffff_ffff_ffff {
        // Nine-byte form: eight 7-bit groups (bits 63..8) then a final full byte
        // (bits 7..0).
        for shift in (8..=57).rev().step_by(7) {
            out.push(0x80 | ((v >> shift) & 0x7f) as u8);
        }
        out.push((v & 0xff) as u8);
        return 9;
    }

    // Fewer than 57 significant bits: standard 7-bits-per-byte form. Find the
    // highest non-empty 7-bit group, then emit groups high→low with the
    // continuation bit set on every byte except the last.
    let mut len = 1;
    let mut shift = 7;
    while shift < 63 && (v >> shift) != 0 {
        len += 1;
        shift += 7;
    }
    for i in (0..len).rev() {
        let group = ((v >> (i * 7)) & 0x7f) as u8;
        // Continuation bit on all but the least-significant group.
        let cont = if i == 0 { 0 } else { 0x80 };
        out.push(cont | group);
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden vectors taken from the SQLite file-format spec and hand-computed.
    /// `(value, encoded-bytes)`.
    const GOLDEN: &[(i64, &[u8])] = &[
        (0, &[0x00]),
        (1, &[0x01]),
        (127, &[0x7f]),
        (128, &[0x81, 0x00]),
        (129, &[0x81, 0x01]),
        (255, &[0x81, 0x7f]),
        (256, &[0x82, 0x00]),
        (300, &[0x82, 0x2c]),
        (16383, &[0xff, 0x7f]),
        (16384, &[0x81, 0x80, 0x00]),
        (2097151, &[0xff, 0xff, 0x7f]),
    ];

    #[test]
    fn golden_vectors_read_and_write() {
        for &(value, bytes) in GOLDEN {
            let mut out = Vec::new();
            let n = write(value, &mut out);
            assert_eq!(out, bytes, "encoding {value}");
            assert_eq!(n, bytes.len(), "length for {value}");

            let (decoded, consumed) = read(bytes).expect("golden decodes");
            assert_eq!(decoded, value, "decoding {bytes:?}");
            assert_eq!(consumed, bytes.len(), "consumed for {bytes:?}");
        }
    }

    #[test]
    fn max_u64_uses_nine_bytes_with_full_last_byte() {
        // 2^64 - 1 is the widest varint: nine bytes, and the last one carries a
        // full eight bits (0xff), not seven.
        let value = -1i64; // all ones == u64::MAX bit pattern
        let mut out = Vec::new();
        let n = write(value, &mut out);
        assert_eq!(n, 9);
        assert_eq!(out, &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        let (decoded, consumed) = read(&out).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, 9);
    }

    #[test]
    fn round_trips_across_a_wide_sweep() {
        // Deterministic LCG — no rng dependency (zero-dep crate).
        let mut state: u64 = 0x1234_5678_9abc_def1;
        for _ in 0..50_000 {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let value = state as i64;
            let mut out = Vec::new();
            let n = write(value, &mut out);
            let (decoded, consumed) = read(&out).expect("sweep decodes");
            assert_eq!(decoded, value, "round-trip value");
            assert_eq!(consumed, n, "round-trip length");
            assert!((1..=9).contains(&n), "length in range");
        }
    }

    #[test]
    fn truncated_input_returns_none_not_panic() {
        // A lone continuation byte promises more that never arrives.
        assert_eq!(read(&[0x81]), None);
        assert_eq!(read(&[]), None);
        // Eight continuation bytes but no ninth.
        assert_eq!(read(&[0x80; 8]), None);
    }
}
