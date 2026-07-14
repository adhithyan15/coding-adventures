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

    /// Serialise this header into the 100 bytes that begin a SQLite database
    /// file — the exact inverse of [`Header::parse`], so `parse(encode(h)) == h`.
    ///
    /// This is a *write*-path rung: paired with [`crate::page_writer`], a caller
    /// can emit `encode()` as the first 100 bytes of page 1 and a leaf page as
    /// page 2 to produce a file our own reader (and SQLite) accepts.
    ///
    /// ## The bytes we write
    ///
    /// ```text
    ///  offset  bytes  field                      value
    ///    0      16    magic string               "SQLite format 3\0"
    ///   16       2    page size (u16-be)          page_size, or 1 for 65536
    ///   18       1    file format write version   1 (legacy/rollback journal)
    ///   19       1    file format read version    1
    ///   20       1    reserved space per page     reserved_space
    ///   21       1    max embedded payload frac.  64  (SQLite requires this)
    ///   22       1    min embedded payload frac.  32  (ditto)
    ///   23       1    leaf payload fraction       32  (ditto)
    ///   24       4    file change counter         change_counter
    ///   28       4    database size in pages      page_count
    ///   32       4    first freelist trunk page   freelist_trunk
    ///   36       4    total freelist pages        freelist_count
    ///   40       4    schema cookie               schema_cookie
    ///   44       4    schema format number        schema_format
    ///   56       4    text encoding               1/2/3 for UTF-8/16le/16be
    ///   …             everything else             0
    /// ```
    ///
    /// The three payload-fraction bytes (64/32/32) are constants SQLite fixes;
    /// writing anything else makes the file unreadable by the C library. Fields
    /// our reader does not surface (default page-cache size, largest-root-page,
    /// user/application version, the version-valid-for and library-version
    /// numbers) are left zero — valid, since a zeroed field simply reads back as
    /// zero. Page size 65536 is stored as the special value 1.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = vec![0u8; 100];
        buf[0..16].copy_from_slice(MAGIC);

        // Page size: 65536 does not fit in the u16 field, so SQLite stores it as
        // the reserved value 1. Every other (power-of-two) size fits directly.
        // A page size outside 512..=65536 is a caller bug — flag it in debug
        // builds; the `as u16` below would otherwise silently truncate.
        debug_assert!(
            self.page_size == 65536 || (512..=0xFFFF).contains(&self.page_size),
            "page_size {} out of the encodable 512..=65536 range",
            self.page_size
        );
        let raw_page_size: u16 = if self.page_size == 65536 {
            1
        } else {
            self.page_size as u16
        };
        buf[16..18].copy_from_slice(&raw_page_size.to_be_bytes());

        // File format read/write versions: 1 = the legacy rollback-journal mode
        // our reader assumes (WAL would be 2).
        buf[18] = 1;
        buf[19] = 1;
        buf[20] = self.reserved_space;
        // Payload fractions — fixed constants required by the format.
        buf[21] = 64;
        buf[22] = 32;
        buf[23] = 32;

        buf[24..28].copy_from_slice(&self.change_counter.to_be_bytes());
        buf[28..32].copy_from_slice(&self.page_count.to_be_bytes());
        buf[32..36].copy_from_slice(&self.freelist_trunk.to_be_bytes());
        buf[36..40].copy_from_slice(&self.freelist_count.to_be_bytes());
        buf[40..44].copy_from_slice(&self.schema_cookie.to_be_bytes());
        buf[44..48].copy_from_slice(&self.schema_format.to_be_bytes());

        let encoding: u32 = match self.text_encoding {
            TextEncoding::Utf8 => 1,
            TextEncoding::Utf16Le => 2,
            TextEncoding::Utf16Be => 3,
        };
        buf[56..60].copy_from_slice(&encoding.to_be_bytes());

        buf
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

    /// `encode` is the exact inverse of `parse`: a header with every
    /// reader-surfaced field set round-trips byte-for-byte back to itself.
    #[test]
    fn encode_round_trips_through_parse() {
        let h = Header {
            page_size: 4096,
            reserved_space: 0,
            page_count: 7,
            change_counter: 42,
            freelist_trunk: 3,
            freelist_count: 2,
            schema_cookie: 9,
            schema_format: 4,
            text_encoding: TextEncoding::Utf8,
        };
        let bytes = h.encode();
        assert_eq!(bytes.len(), 100, "header must be exactly 100 bytes");
        assert_eq!(&bytes[0..16], MAGIC);
        // Payload-fraction constants SQLite requires.
        assert_eq!((bytes[21], bytes[22], bytes[23]), (64, 32, 32));
        assert_eq!(Header::parse(&bytes).unwrap(), h);
    }

    /// Page size 65536 is stored as the special value 1 and decodes back to
    /// 65536; reserved space and UTF-16 encodings also survive the round-trip.
    #[test]
    fn encode_handles_65536_and_reserved_and_utf16() {
        let h = Header {
            page_size: 65536,
            reserved_space: 32,
            page_count: 1,
            change_counter: 0,
            freelist_trunk: 0,
            freelist_count: 0,
            schema_cookie: 0,
            schema_format: 1,
            text_encoding: TextEncoding::Utf16Le,
        };
        let bytes = h.encode();
        // 65536 is written as the u16 value 1.
        assert_eq!(u16::from_be_bytes([bytes[16], bytes[17]]), 1);
        assert_eq!(Header::parse(&bytes).unwrap(), h);
    }

    /// The bytes `encode` produces are accepted by the pager: opening a two-page
    /// buffer whose first 100 bytes are `encode()` yields the same header.
    #[test]
    fn encoded_header_opens_via_pager() {
        let h = Header {
            page_size: 512,
            reserved_space: 0,
            page_count: 2,
            change_counter: 1,
            freelist_trunk: 0,
            freelist_count: 0,
            schema_cookie: 0,
            schema_format: 1,
            text_encoding: TextEncoding::Utf8,
        };
        let mut file = vec![0u8; 512 * 2];
        file[0..100].copy_from_slice(&h.encode());
        let (parsed, _pager) = crate::pager::Pager::open(&file).unwrap();
        assert_eq!(parsed, h);
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
