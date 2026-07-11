//! # The pager — pages out of a byte slice
//!
//! SQLite files are an array of fixed-size **pages**, numbered from 1. The
//! pager's one job is to hand back page *N*'s bytes. A real SQLite pager also
//! manages a cache and a rollback journal for writing; this reader needs
//! neither — it is read-only and the whole database is already in memory (the
//! Anki importer hands us the deserialized bytes). So our pager is a thin,
//! zero-copy view: it *borrows* the `&[u8]` and returns sub-slices of it. No
//! allocation, no copying, no I/O.
//!
//! ## Page numbering
//!
//! Pages are **1-based**. Page 1 begins at file offset 0 and, uniquely, carries
//! the 100-byte database header in its first 100 bytes — so a b-tree that roots
//! on page 1 (the `sqlite_schema` table) starts its cell content at offset 100,
//! not 0. The pager itself does not special-case this: `page(1)` returns the
//! whole page including the header bytes, and the b-tree layer (Phase E3) is
//! responsible for skipping the header when it walks page 1. Keeping that
//! knowledge out of the pager keeps the pager trivially correct.
//!
//! ```text
//!   file: ┌── page 1 ──┬── page 2 ──┬── page 3 ──┬ … ┐
//!         │header+data │   data     │   data     │   │
//!         0         psize        2·psize      3·psize
//! ```

use crate::error::SqliteError;
use crate::header::Header;

/// A read-only, zero-copy view over a SQLite database's bytes.
pub struct Pager<'a> {
    data: &'a [u8],
    page_size: usize,
}

impl<'a> Pager<'a> {
    /// Parse the header and build a pager over `data` in one step. Returns both
    /// so the caller has the header's metadata (page count, encoding, …) and the
    /// page accessor together.
    pub fn open(data: &'a [u8]) -> Result<(Header, Pager<'a>), SqliteError> {
        let header = Header::parse(data)?;
        let pager = Pager {
            data,
            page_size: header.page_size as usize,
        };
        Ok((header, pager))
    }

    /// Build a pager directly from an already-parsed page size. Useful when the
    /// header has been read separately.
    pub fn with_page_size(data: &'a [u8], page_size: usize) -> Pager<'a> {
        Pager { data, page_size }
    }

    /// Borrow page `page_no` (1-based) as a `page_size`-byte slice.
    ///
    /// Returns `BadPageNumber` for page 0 or any page whose bytes fall outside
    /// the file — so a b-tree cell that points at a bogus page (corrupt or
    /// hostile input) yields a clean error rather than an out-of-bounds read.
    pub fn page(&self, page_no: u32) -> Result<&'a [u8], SqliteError> {
        if page_no == 0 {
            return Err(SqliteError::BadPageNumber(0));
        }
        // Offset of this page: (page_no - 1) * page_size. Use checked math so a
        // huge page number cannot overflow `usize` and wrap to a valid slice.
        let index = (page_no - 1) as usize;
        let start = index
            .checked_mul(self.page_size)
            .ok_or(SqliteError::BadPageNumber(page_no))?;
        let end = start
            .checked_add(self.page_size)
            .ok_or(SqliteError::BadPageNumber(page_no))?;
        self.data
            .get(start..end)
            .ok_or(SqliteError::BadPageNumber(page_no))
    }

    /// The page size in bytes.
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// How many whole pages the file actually holds (from its length). This can
    /// be cross-checked against the header's `page_count`.
    // Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
    #[allow(clippy::manual_checked_ops)]
    pub fn page_count(&self) -> usize {
        if self.page_size == 0 {
            0
        } else {
            self.data.len() / self.page_size
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::MAGIC;

    /// A three-page database of `page_size` bytes each, with a valid header on
    /// page 1 and a recognisable byte stamped at the start of each page.
    fn three_page_db(page_size: usize) -> Vec<u8> {
        let mut data = vec![0u8; page_size * 3];
        data[0..16].copy_from_slice(MAGIC);
        data[16..18].copy_from_slice(&(page_size as u16).to_be_bytes());
        data[56..60].copy_from_slice(&1u32.to_be_bytes()); // UTF-8
        data[28..32].copy_from_slice(&3u32.to_be_bytes()); // 3 pages
                                                           // Stamp each page's first content byte (after the header on page 1).
        data[100] = 0xA1;
        data[page_size] = 0xB2;
        data[page_size * 2] = 0xC3;
        data
    }

    #[test]
    fn open_returns_header_and_pager() {
        let db = three_page_db(512);
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(header.page_size, 512);
        assert_eq!(header.page_count, 3);
        assert_eq!(pager.page_size(), 512);
        assert_eq!(pager.page_count(), 3);
    }

    #[test]
    fn page_one_includes_the_header_bytes() {
        let db = three_page_db(512);
        let (_h, pager) = Pager::open(&db).unwrap();
        let page1 = pager.page(1).unwrap();
        assert_eq!(page1.len(), 512);
        assert_eq!(&page1[0..16], MAGIC); // header is part of page 1
        assert_eq!(page1[100], 0xA1); // content begins after the header
    }

    #[test]
    fn later_pages_slice_at_the_right_offset() {
        let db = three_page_db(512);
        let (_h, pager) = Pager::open(&db).unwrap();
        assert_eq!(pager.page(2).unwrap()[0], 0xB2);
        assert_eq!(pager.page(3).unwrap()[0], 0xC3);
    }

    #[test]
    fn page_zero_and_out_of_range_pages_error() {
        let db = three_page_db(512);
        let (_h, pager) = Pager::open(&db).unwrap();
        assert_eq!(pager.page(0), Err(SqliteError::BadPageNumber(0)));
        assert_eq!(pager.page(4), Err(SqliteError::BadPageNumber(4)));
        // A page number large enough to overflow the offset multiplication must
        // also error cleanly, not wrap.
        assert_eq!(
            pager.page(u32::MAX),
            Err(SqliteError::BadPageNumber(u32::MAX))
        );
    }
}
