//! # The 100-byte database header
//!
//! Every SQLite file opens with a fixed 100-byte header at offset 0 (the start
//! of *page 1*). It is the map to everything else: how big each page is, how
//! many pages there are, how text is encoded. This module reads it.
//!
//! ## Byte layout (the fields this reader uses)
//!
//! ```text
//! offset  size  field
//!    0    16    magic string  "SQLite format 3\0"
//!   16     2    page size (u16-be); the value 1 means 65536
//!   20     1    bytes of reserved space at the end of every page
//!   24     4    file change counter (u32-be)
//!   28     4    database size in pages (u32-be)
//!   32     4    first freelist trunk page (u32-be, 0 = none)
//!   36     4    total freelist pages (u32-be)
//!   40     4    schema cookie (u32-be)
//!   44     4    schema format number (u32-be)
//!   56     4    text encoding (u32-be): 1=UTF-8, 2=UTF-16le, 3=UTF-16be
//! ```
//!
//! (The remaining fields — payload fractions, version numbers, user version,
//! incremental-vacuum settings — do not affect *reading* table rows, so we skip
//! them. They matter to the Phase F writer.)
//!
//! ## The page-size quirk
//!
//! Page size is stored in two bytes, but SQLite supports sizes up to 65536,
//! which does not fit in a `u16`. The trick: the on-disk value `1` is a stand-in
//! for 65536. Every other legal value is the size itself, and must be a power of
//! two no smaller than 512.
//!
//! ## The reserved-space field
//!
//! Some databases reserve a few bytes at the tail of every page (offset 20).
//! The *usable* size of a page — what b-tree cells and overflow math operate on
//! — is `page_size - reserved_space`, not `page_size`. We surface both so the
//! b-tree layer (Phase E3) can compute overflow thresholds correctly.

use crate::error::SqliteError;

/// How text columns are encoded in this database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    /// UTF-8 (header value 1). The only encoding this reader decodes; it is what
    /// Anki collections use.
    Utf8,
    /// UTF-16 little-endian (header value 2) — recognised but not decoded.
    Utf16Le,
    /// UTF-16 big-endian (header value 3) — recognised but not decoded.
    Utf16Be,
}

/// The parsed database header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// Page size in bytes (a power of two, 512..=65536).
    pub page_size: u32,
    /// Bytes reserved at the end of every page (usually 0). The usable page
    /// area is `page_size - reserved_space`.
    pub reserved_space: u8,
    /// Number of pages in the database, per the header. (The file should be
    /// exactly `page_size * page_count` bytes.)
    pub page_count: u32,
    /// Bumped on every write transaction — lets a reader notice concurrent change.
    pub change_counter: u32,
    /// First freelist trunk page (0 if the freelist is empty).
    pub freelist_trunk: u32,
    /// Total number of pages on the freelist.
    pub freelist_count: u32,
    /// Bumped whenever `sqlite_schema` changes.
    pub schema_cookie: u32,
    /// Schema format number (1..=4).
    pub schema_format: u32,
    /// Text encoding of `TEXT` columns.
    pub text_encoding: TextEncoding,
}

/// The magic bytes every SQLite file begins with.
pub const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Read a big-endian `u32` at `offset` (offset+4 is guaranteed in range by the
/// caller, which has already checked the buffer holds ≥ 100 bytes).
fn be_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

impl Header {
    /// Parse the header from the start of a database's bytes.
    pub fn parse(buf: &[u8]) -> Result<Header, SqliteError> {
        // The header is 100 bytes; anything shorter cannot be a database.
        if buf.len() < 100 {
            return Err(SqliteError::Truncated("database header"));
        }
        if &buf[0..16] != MAGIC {
            return Err(SqliteError::BadMagic);
        }

        // Page size: two big-endian bytes, with `1` meaning 65536.
        let raw_page_size = u16::from_be_bytes([buf[16], buf[17]]);
        let page_size: u32 = if raw_page_size == 1 {
            65536
        } else {
            u32::from(raw_page_size)
        };
        // Must be a power of two, at least 512.
        if page_size < 512 || !page_size.is_power_of_two() {
            return Err(SqliteError::BadPageSize(page_size));
        }

        let reserved_space = buf[20];
        // A page must have room for content after its reserved tail (and, on
        // page 1, after the 100-byte header). If reserved space swallows the
        // page, the file is unusable.
        if u32::from(reserved_space) >= page_size {
            return Err(SqliteError::BadPageSize(page_size));
        }

        let text_encoding = match be_u32(buf, 56) {
            1 => TextEncoding::Utf8,
            2 => TextEncoding::Utf16Le,
            3 => TextEncoding::Utf16Be,
            _ => return Err(SqliteError::Unsupported("text encoding")),
        };

        Ok(Header {
            page_size,
            reserved_space,
            page_count: be_u32(buf, 28),
            change_counter: be_u32(buf, 24),
            freelist_trunk: be_u32(buf, 32),
            freelist_count: be_u32(buf, 36),
            schema_cookie: be_u32(buf, 40),
            schema_format: be_u32(buf, 44),
            text_encoding,
        })
    }

    /// The usable bytes per page — the page size minus any reserved tail. This
    /// is the figure b-tree cell layout and overflow thresholds are computed
    /// against, not the raw page size.
    pub fn usable_size(&self) -> u32 {
        self.page_size - u32::from(self.reserved_space)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal but valid 100-byte header with the given page size and
    /// encoding byte, everything else zeroed.
    fn make_header(page_size_field: u16, encoding: u32) -> Vec<u8> {
        let mut buf = vec![0u8; 100];
        buf[0..16].copy_from_slice(MAGIC);
        buf[16..18].copy_from_slice(&page_size_field.to_be_bytes());
        buf[56..60].copy_from_slice(&encoding.to_be_bytes());
        buf
    }

    #[test]
    fn parses_a_typical_4096_utf8_header() {
        let mut buf = make_header(4096, 1);
        buf[28..32].copy_from_slice(&7u32.to_be_bytes()); // 7 pages
        buf[20] = 0; // no reserved space
        let h = Header::parse(&buf).unwrap();
        assert_eq!(h.page_size, 4096);
        assert_eq!(h.page_count, 7);
        assert_eq!(h.reserved_space, 0);
        assert_eq!(h.text_encoding, TextEncoding::Utf8);
        assert_eq!(h.usable_size(), 4096);
    }

    #[test]
    fn page_size_one_means_65536() {
        let h = Header::parse(&make_header(1, 1)).unwrap();
        assert_eq!(h.page_size, 65536);
    }

    #[test]
    fn reserved_space_reduces_usable_size() {
        let mut buf = make_header(4096, 1);
        buf[20] = 32;
        let h = Header::parse(&buf).unwrap();
        assert_eq!(h.usable_size(), 4096 - 32);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut buf = make_header(4096, 1);
        buf[0] = b'X';
        assert_eq!(Header::parse(&buf), Err(SqliteError::BadMagic));
    }

    #[test]
    fn rejects_non_power_of_two_page_size() {
        // 4097 is not a power of two.
        assert_eq!(
            Header::parse(&make_header(4097, 1)),
            Err(SqliteError::BadPageSize(4097))
        );
        // 256 is below the 512 minimum.
        assert_eq!(
            Header::parse(&make_header(256, 1)),
            Err(SqliteError::BadPageSize(256))
        );
    }

    #[test]
    fn rejects_short_buffer() {
        assert_eq!(
            Header::parse(&[0u8; 50]),
            Err(SqliteError::Truncated("database header"))
        );
    }

    #[test]
    fn recognises_utf16_encodings() {
        assert_eq!(
            Header::parse(&make_header(4096, 2)).unwrap().text_encoding,
            TextEncoding::Utf16Le
        );
        assert_eq!(
            Header::parse(&make_header(4096, 3)).unwrap().text_encoding,
            TextEncoding::Utf16Be
        );
    }

    #[test]
    fn rejects_unknown_encoding() {
        assert_eq!(
            Header::parse(&make_header(4096, 9)),
            Err(SqliteError::Unsupported("text encoding"))
        );
    }
}
