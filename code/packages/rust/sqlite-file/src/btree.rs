//! # Walking a table b-tree
//!
//! A SQLite *table* stores its rows in a **b-tree** keyed by `rowid`. This
//! module walks that tree from its root page and yields every `(rowid, record
//! bytes)` pair, in rowid order — the raw material [`crate::record::decode`]
//! turns into typed columns.
//!
//! ## Two kinds of page
//!
//! Every b-tree page starts with a one-byte **type**:
//!
//! - **`0x0D` — leaf table page.** Holds the actual rows. Each cell is
//!   `[payload-length varint] [rowid varint] [record bytes…]`.
//! - **`0x05` — interior table page.** Holds no rows, only pointers: each cell
//!   is `[left-child page (u32-be)] [rowid varint]`, and the page header carries
//!   one extra **right-most child** pointer. Walking one means descending into
//!   every child.
//!
//! (Index b-trees use `0x0A`/`0x02`; this reader is table-only and rejects them.)
//!
//! ## The page-1 quirk, again
//!
//! Page 1 opens with the 100-byte database header, so a b-tree rooted on page 1
//! (the `sqlite_schema` table) puts its page header at offset **100**, not 0.
//! Every other page's header is at offset 0. Cell-pointer values, however, are
//! always offsets from the start of the page. `header_offset` below captures
//! exactly this.
//!
//! ## Page-header byte layout
//!
//! ```text
//! offset  size  field
//!    0     1    page type (0x0D leaf / 0x05 interior)
//!    1     2    first freeblock (unused here)
//!    3     2    number of cells on the page (u16-be)
//!    5     2    cell content area start (unused here)
//!    7     1    fragmented free bytes (unused here)
//!    8     4    right-most child pointer (u32-be) — INTERIOR pages only
//! ```
//!
//! The cell-pointer array (two bytes per cell, each a u16-be offset into the
//! page) begins right after the header: offset 8 on a leaf, 12 on an interior
//! page.
//!
//! ## Overflow — deferred
//!
//! A row whose record is too big for one page spills into an *overflow chain*.
//! That is Phase E3b; here, a cell that would overflow returns
//! `Unsupported("overflow chain")` rather than silently truncating the record.
//! The tables Engram reads whose rows all fit inline work today.

use crate::error::SqliteError;
use crate::header::Header;
use crate::pager::Pager;
use crate::varint;

const LEAF_TABLE: u8 = 0x0D;
const INTERIOR_TABLE: u8 = 0x05;

/// Read the big-endian `u16` at `off` (bounds-checked).
fn be_u16(page: &[u8], off: usize) -> Option<u16> {
    let hi = *page.get(off)?;
    let lo = *page.get(off + 1)?;
    Some(u16::from_be_bytes([hi, lo]))
}

/// Read the big-endian `u32` at `off` (bounds-checked).
fn be_u32(page: &[u8], off: usize) -> Option<u32> {
    let b0 = *page.get(off)?;
    let b1 = *page.get(off + 1)?;
    let b2 = *page.get(off + 2)?;
    let b3 = *page.get(off + 3)?;
    Some(u32::from_be_bytes([b0, b1, b2, b3]))
}

/// Walk the table b-tree rooted at `root_page` and return every row as
/// `(rowid, record bytes)`, sorted by rowid.
///
/// `root_page` is typically obtained from `sqlite_schema` (the `rootpage`
/// column); for `sqlite_schema` itself it is page 1. Bounds- and cycle-checked
/// throughout: a corrupt tree (bad page type, cell pointer past the page, or a
/// child-pointer cycle) yields an `Err`, never a panic, an out-of-bounds read,
/// or an infinite loop.
pub fn walk_table<'a>(
    pager: &Pager<'a>,
    header: &Header,
    root_page: u32,
) -> Result<Vec<(i64, Vec<u8>)>, SqliteError> {
    // The largest record that fits inline on a leaf page, per the format's
    // table-leaf formula: usable_size - 35. A larger payload spills to overflow
    // (Phase E3b) — for now we detect it and refuse rather than truncate.
    let max_local = header.usable_size().saturating_sub(35) as usize;

    // Anti-amplification budget. The cell-pointer array is attacker-controlled
    // and pointers need not be distinct: a hostile page could point all of its
    // (up to 65535) cells at one full-size cell, making us copy that record
    // thousands of times — gigabytes out of a single small page, an OOM DoS.
    // Every record byte a *well-formed* database emits is physically stored in
    // some page, and pages do not overlap, so the total record bytes can never
    // exceed the file's byte length. Cap the running total there: a valid file
    // is always under it, and the aliasing attack trips it after copying at most
    // one page's worth of bytes.
    let file_bytes = pager.page_count().saturating_mul(pager.page_size());
    let mut emitted_bytes: usize = 0;

    let mut rows: Vec<(i64, Vec<u8>)> = Vec::new();

    // Explicit stack instead of recursion: a deep or maliciously-nested tree
    // cannot overflow the call stack. A `visited` set bounds total work and
    // turns any child-pointer cycle into a clean `Corrupt` error.
    let mut stack: Vec<u32> = vec![root_page];
    let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();

    while let Some(page_no) = stack.pop() {
        if !visited.insert(page_no) {
            return Err(SqliteError::Corrupt("b-tree page cycle"));
        }
        let page = pager.page(page_no)?;

        // Page 1 carries the 100-byte database header before its b-tree header.
        let header_off = if page_no == 1 { 100 } else { 0 };

        let page_type = *page
            .get(header_off)
            .ok_or(SqliteError::Truncated("b-tree page"))?;
        let cell_count =
            be_u16(page, header_off + 3).ok_or(SqliteError::Truncated("b-tree page"))? as usize;

        match page_type {
            LEAF_TABLE => {
                let ptr_array = header_off + 8;
                for i in 0..cell_count {
                    let cell_off = cell_pointer(page, ptr_array, i)?;
                    let (rowid, record) = read_leaf_cell(page, cell_off, max_local)?;
                    emitted_bytes = emitted_bytes
                        .checked_add(record.len())
                        .filter(|total| *total <= file_bytes)
                        .ok_or(SqliteError::Corrupt("row data exceeds file size"))?;
                    rows.push((rowid, record));
                }
            }
            INTERIOR_TABLE => {
                // Each interior cell points at a left child; the page header
                // holds the extra right-most child.
                let ptr_array = header_off + 12;
                for i in 0..cell_count {
                    let cell_off = cell_pointer(page, ptr_array, i)?;
                    let child = be_u32(page, cell_off)
                        .ok_or(SqliteError::Corrupt("interior cell past page"))?;
                    stack.push(child);
                }
                let rightmost = be_u32(page, header_off + 8)
                    .ok_or(SqliteError::Truncated("interior page header"))?;
                stack.push(rightmost);
            }
            _ => return Err(SqliteError::Corrupt("unexpected b-tree page type")),
        }
    }

    // A table scan yields rows in rowid order; collecting across a DFS and
    // sorting by the unique rowid gives that same order deterministically.
    rows.sort_by_key(|(rowid, _)| *rowid);
    Ok(rows)
}

/// The `i`-th cell's byte offset within the page, read from the cell-pointer
/// array at `ptr_array`.
fn cell_pointer(page: &[u8], ptr_array: usize, i: usize) -> Result<usize, SqliteError> {
    let entry = ptr_array
        .checked_add(
            i.checked_mul(2)
                .ok_or(SqliteError::Corrupt("cell index overflow"))?,
        )
        .ok_or(SqliteError::Corrupt("cell index overflow"))?;
    let off = be_u16(page, entry).ok_or(SqliteError::Corrupt("cell pointer past page"))?;
    Ok(off as usize)
}

/// Decode one leaf-table cell at `cell_off`: `[payload-len varint] [rowid
/// varint] [record bytes]`. Returns `(rowid, record bytes)`.
fn read_leaf_cell(
    page: &[u8],
    cell_off: usize,
    max_local: usize,
) -> Result<(i64, Vec<u8>), SqliteError> {
    let cell = page
        .get(cell_off..)
        .ok_or(SqliteError::Corrupt("cell pointer past page"))?;

    let (payload_len, n1) = varint::read(cell).ok_or(SqliteError::Corrupt("bad payload length"))?;
    let payload_len =
        usize::try_from(payload_len).map_err(|_| SqliteError::Corrupt("bad payload length"))?;

    let after_len = cell
        .get(n1..)
        .ok_or(SqliteError::Corrupt("truncated cell"))?;
    let (rowid, n2) = varint::read(after_len).ok_or(SqliteError::Corrupt("bad rowid"))?;

    // A payload larger than the inline maximum spills into an overflow chain,
    // which this layer does not yet reassemble (Phase E3b).
    if payload_len > max_local {
        return Err(SqliteError::Unsupported("overflow chain"));
    }

    let record_start = n1 + n2;
    let record = after_len
        .get(n2..)
        .and_then(|s| s.get(..payload_len))
        .ok_or(SqliteError::Corrupt("record past page"))?;
    let _ = record_start; // documented offset; the slice above is what we return
    Ok((rowid, record.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::MAGIC;

    /// Build a one-page database (page size `ps`) whose page 1 is a *leaf* table
    /// b-tree holding `rows` as `(rowid, record-bytes)`. Cells are packed from
    /// the end of the page downward, exactly as SQLite lays them out.
    fn one_leaf_page_db(ps: usize, rows: &[(i64, Vec<u8>)]) -> Vec<u8> {
        let mut page = vec![0u8; ps];
        // DB header on page 1.
        page[0..16].copy_from_slice(MAGIC);
        page[16..18].copy_from_slice(&(ps as u16).to_be_bytes());
        page[56..60].copy_from_slice(&1u32.to_be_bytes()); // UTF-8
        page[28..32].copy_from_slice(&1u32.to_be_bytes()); // 1 page

        // B-tree header at offset 100 (page 1).
        let h = 100;
        page[h] = LEAF_TABLE;
        page[h + 3..h + 5].copy_from_slice(&(rows.len() as u16).to_be_bytes());

        // Write each cell from the end of the page down; record its offset.
        let mut content_top = ps;
        let ptr_array = h + 8;
        for (i, (rowid, record)) in rows.iter().enumerate() {
            let mut cell = Vec::new();
            varint::write(record.len() as i64, &mut cell);
            varint::write(*rowid, &mut cell);
            cell.extend_from_slice(record);
            content_top -= cell.len();
            page[content_top..content_top + cell.len()].copy_from_slice(&cell);
            let ptr = (content_top as u16).to_be_bytes();
            page[ptr_array + i * 2..ptr_array + i * 2 + 2].copy_from_slice(&ptr);
        }
        page[h + 5..h + 7].copy_from_slice(&(content_top as u16).to_be_bytes());
        page
    }

    #[test]
    fn walks_a_single_leaf_page_in_rowid_order() {
        // Rows inserted out of rowid order; walk must return them sorted.
        let db = one_leaf_page_db(
            512,
            &[
                (2, vec![0xAA, 0xBB]),
                (1, vec![0x01]),
                (3, vec![0xCC, 0xDD, 0xEE]),
            ],
        );
        let (header, pager) = Pager::open(&db).unwrap();
        let rows = walk_table(&pager, &header, 1).unwrap();
        assert_eq!(
            rows,
            vec![
                (1, vec![0x01]),
                (2, vec![0xAA, 0xBB]),
                (3, vec![0xCC, 0xDD, 0xEE]),
            ]
        );
    }

    #[test]
    fn empty_leaf_page_yields_no_rows() {
        let db = one_leaf_page_db(512, &[]);
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(walk_table(&pager, &header, 1).unwrap(), vec![]);
    }

    /// Build a two-level tree: page 1 is an *interior* page pointing at two leaf
    /// pages (2 and 3), each holding rows. Exercises interior descent + the
    /// right-most child pointer.
    fn interior_over_two_leaves_db() -> Vec<u8> {
        let ps = 512usize;
        let mut data = vec![0u8; ps * 3];

        // --- Page 1: interior table page with one cell (left child = 2) and
        //     right-most child = 3.
        data[0..16].copy_from_slice(MAGIC);
        data[16..18].copy_from_slice(&(ps as u16).to_be_bytes());
        data[56..60].copy_from_slice(&1u32.to_be_bytes());
        data[28..32].copy_from_slice(&3u32.to_be_bytes()); // 3 pages
        let h = 100;
        data[h] = INTERIOR_TABLE;
        data[h + 3..h + 5].copy_from_slice(&1u16.to_be_bytes()); // 1 cell
        data[h + 8..h + 12].copy_from_slice(&3u32.to_be_bytes()); // right child = page 3
                                                                  // One interior cell at the end of the page: [left child = 2][rowid = 2].
        let mut cell = Vec::new();
        cell.extend_from_slice(&2u32.to_be_bytes()); // left child = page 2
        varint::write(2, &mut cell); // divider rowid
        let cell_off = ps - cell.len();
        data[cell_off..cell_off + cell.len()].copy_from_slice(&cell);
        data[h + 12..h + 14].copy_from_slice(&(cell_off as u16).to_be_bytes());

        // --- Page 2: leaf with rowid 1.  --- Page 3: leaf with rowids 2, 3.
        for (page_no, rows) in [
            (2u32, vec![(1i64, vec![0x11])]),
            (3, vec![(2, vec![0x22]), (3, vec![0x33])]),
        ] {
            let base = (page_no as usize - 1) * ps;
            data[base] = LEAF_TABLE;
            data[base + 3..base + 5].copy_from_slice(&(rows.len() as u16).to_be_bytes());
            let mut top = base + ps;
            for (i, (rowid, record)) in rows.iter().enumerate() {
                let mut c = Vec::new();
                varint::write(record.len() as i64, &mut c);
                varint::write(*rowid, &mut c);
                c.extend_from_slice(record);
                top -= c.len();
                data[top..top + c.len()].copy_from_slice(&c);
                let ptr = ((top - base) as u16).to_be_bytes();
                data[base + 8 + i * 2..base + 8 + i * 2 + 2].copy_from_slice(&ptr);
            }
        }
        data
    }

    #[test]
    fn walks_interior_page_descending_into_children() {
        let db = interior_over_two_leaves_db();
        let (header, pager) = Pager::open(&db).unwrap();
        let rows = walk_table(&pager, &header, 1).unwrap();
        assert_eq!(
            rows,
            vec![(1, vec![0x11]), (2, vec![0x22]), (3, vec![0x33])]
        );
    }

    #[test]
    fn rejects_unknown_page_type() {
        let mut db = one_leaf_page_db(512, &[(1, vec![0x00])]);
        db[100] = 0x0A; // index leaf — unsupported
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 1),
            Err(SqliteError::Corrupt("unexpected b-tree page type"))
        );
    }

    #[test]
    fn detects_a_child_pointer_cycle() {
        // An interior page whose right-most child points back at itself (page 1).
        let ps = 512usize;
        let mut data = vec![0u8; ps];
        data[0..16].copy_from_slice(MAGIC);
        data[16..18].copy_from_slice(&(ps as u16).to_be_bytes());
        data[56..60].copy_from_slice(&1u32.to_be_bytes());
        data[28..32].copy_from_slice(&1u32.to_be_bytes());
        let h = 100;
        data[h] = INTERIOR_TABLE;
        data[h + 3..h + 5].copy_from_slice(&0u16.to_be_bytes()); // 0 cells
        data[h + 8..h + 12].copy_from_slice(&1u32.to_be_bytes()); // right child = page 1 (self!)
        let (header, pager) = Pager::open(&data).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 1),
            Err(SqliteError::Corrupt("b-tree page cycle"))
        );
    }

    #[test]
    fn rejects_aliased_cell_pointer_amplification() {
        // Anti-DoS: a hostile page pointing many cells at ONE full-size cell
        // must not copy that record thousands of times. Build a 512-byte page
        // (file = 512 bytes) claiming 20 cells that all alias one ~400-byte
        // record — replaying it would emit ~8 KB, well over the 512-byte file.
        let ps = 512usize;
        let mut page = vec![0u8; ps];
        page[0..16].copy_from_slice(MAGIC);
        page[16..18].copy_from_slice(&(ps as u16).to_be_bytes());
        page[56..60].copy_from_slice(&1u32.to_be_bytes());
        page[28..32].copy_from_slice(&1u32.to_be_bytes());
        let h = 100;
        page[h] = LEAF_TABLE;
        page[h + 3..h + 5].copy_from_slice(&20u16.to_be_bytes()); // 20 cells claimed

        // One real cell near the end: a ~400-byte record.
        let record = vec![0x5A; 400];
        let mut cell = Vec::new();
        varint::write(record.len() as i64, &mut cell);
        varint::write(1, &mut cell); // rowid
        cell.extend_from_slice(&record);
        let cell_off = ps - cell.len();
        page[cell_off..cell_off + cell.len()].copy_from_slice(&cell);
        // Point ALL 20 cell pointers at that same cell.
        for i in 0..20 {
            page[h + 8 + i * 2..h + 8 + i * 2 + 2]
                .copy_from_slice(&(cell_off as u16).to_be_bytes());
        }

        let (header, pager) = Pager::open(&page).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 1),
            Err(SqliteError::Corrupt("row data exceeds file size"))
        );
    }
}
