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
//! ## Overflow chains
//!
//! A row whose record is too big for one page keeps only its **first K bytes**
//! inline; the rest spills into a linked list of *overflow pages*. The leaf cell
//! then looks like `[payload-len varint] [rowid varint] [first K bytes] [u32-be
//! first-overflow-page]`, and each overflow page is `[u32-be next-page] [content
//! bytes]`, chained until the full payload is collected (a next-page of 0 ends
//! the chain).
//!
//! The inline split follows SQLite's table-leaf rule exactly. With usable size
//! `U` and payload length `P`:
//!
//! ```text
//!   X = U - 35                        (max bytes that stay inline)
//!   M = ((U - 12) * 32 / 255) - 23    (min bytes that stay inline)
//!   K = M + ((P - M) mod (U - 4))     (candidate inline length)
//!   inline = if K <= X { K } else { M }
//! ```
//!
//! and each overflow page carries `U - 4` content bytes (the first 4 are the
//! next-page pointer). [`read_leaf_cell`] reassembles inline + overflow into one
//! contiguous record, guarding the chain against cycles (a visited-page set) and
//! against amplification (the same file-size byte cap the leaf loop uses).

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
    // Usable bytes per page (page size minus the reserved tail), and the largest
    // record that fits inline on a leaf page, per the format's table-leaf
    // formula: usable_size - 35. A larger payload keeps only its first K bytes
    // inline and spills the rest into an overflow chain, which `read_leaf_cell`
    // reassembles.
    let usable = header.usable_size() as usize;
    let max_local = usable.saturating_sub(35);

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
                    let (rowid, record) =
                        read_leaf_cell(pager, page, cell_off, usable, max_local, file_bytes)?;
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
/// varint] [payload]`. Returns `(rowid, record bytes)`.
///
/// When the payload fits inline (`payload_len <= max_local`) the record is a
/// direct slice of the page. When it does not, only the first K bytes are inline
/// — followed by a 4-byte overflow-page pointer — and the remainder is collected
/// from the overflow chain by [`follow_overflow`]. `usable` is the page's usable
/// size; `file_bytes` is the running amplification cap (a record can never
/// exceed the file's own byte length).
fn read_leaf_cell(
    pager: &Pager<'_>,
    page: &[u8],
    cell_off: usize,
    usable: usize,
    max_local: usize,
    file_bytes: usize,
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
    let payload = after_len
        .get(n2..)
        .ok_or(SqliteError::Corrupt("truncated cell"))?;

    // Common case: the whole payload sits on this page.
    if payload_len <= max_local {
        let record = payload
            .get(..payload_len)
            .ok_or(SqliteError::Corrupt("record past page"))?;
        return Ok((rowid, record.to_vec()));
    }

    // Overflow. Reject a payload that could not physically fit in the file up
    // front — this bounds the reassembly target and rejects a hostile cell that
    // claims a gigantic length. (A valid record's bytes all live in the file, so
    // its length is at most the file's length.)
    if payload_len > file_bytes {
        return Err(SqliteError::Corrupt("payload exceeds file size"));
    }

    // How many bytes stay on this page, per the SQLite table-leaf rule.
    //   X = usable - 35 = max_local          (the inline ceiling)
    //   M = ((usable - 12) * 32 / 255) - 23   (the inline floor)
    //   K = M + ((P - M) mod (usable - 4))
    //   inline = if K <= X { K } else { M }
    let m = usable.saturating_sub(12).saturating_mul(32) / 255;
    let min_local = m.saturating_sub(23);
    let span = usable
        .checked_sub(4)
        .filter(|s| *s > 0)
        .ok_or(SqliteError::Corrupt("usable size too small"))?;
    let k = min_local + (payload_len - min_local) % span;
    let inline = if k <= max_local { k } else { min_local };

    // The inline bytes, then the 4-byte pointer to the first overflow page.
    let inline_bytes = payload
        .get(..inline)
        .ok_or(SqliteError::Corrupt("inline payload past page"))?;
    let first_overflow =
        be_u32(payload, inline).ok_or(SqliteError::Corrupt("missing overflow ptr"))?;

    let mut record = Vec::with_capacity(payload_len);
    record.extend_from_slice(inline_bytes);
    follow_overflow(
        pager,
        first_overflow,
        payload_len,
        usable,
        file_bytes,
        &mut record,
    )?;
    Ok((rowid, record))
}

/// Collect the tail of an overflow payload by walking the overflow-page chain,
/// appending onto `record` until it holds `payload_len` bytes.
///
/// Each overflow page is `[u32-be next-page] [content bytes]`; the content area
/// runs from offset 4 to `usable`, giving `usable - 4` bytes per page. A next-page
/// of 0 ends the chain. Guarded three ways so hostile input cannot hang or
/// exhaust memory: a `visited` set turns any cycle into `Corrupt`, every page
/// fetch is bounds-checked by the pager, and the total never exceeds `file_bytes`.
fn follow_overflow(
    pager: &Pager<'_>,
    first_page: u32,
    payload_len: usize,
    usable: usize,
    file_bytes: usize,
    record: &mut Vec<u8>,
) -> Result<(), SqliteError> {
    let mut next = first_page;
    let mut visited: std::collections::HashSet<u32> = std::collections::HashSet::new();

    while record.len() < payload_len {
        if next == 0 {
            return Err(SqliteError::Corrupt("overflow chain ended early"));
        }
        if !visited.insert(next) {
            return Err(SqliteError::Corrupt("overflow chain cycle"));
        }
        let page = pager.page(next)?;
        let next_ptr = be_u32(page, 0).ok_or(SqliteError::Corrupt("truncated overflow page"))?;
        let content = page
            .get(4..usable)
            .ok_or(SqliteError::Corrupt("overflow content past page"))?;

        // Take only as many bytes as the payload still needs from this page.
        let still_needed = payload_len - record.len();
        let take = still_needed.min(content.len());
        record.extend_from_slice(&content[..take]);

        // Belt-and-braces amplification guard (the visited set already bounds the
        // chain to the file's real pages, but this makes the cap explicit).
        if record.len() > file_bytes {
            return Err(SqliteError::Corrupt("overflow payload exceeds file size"));
        }
        next = next_ptr;
    }
    Ok(())
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

    /// The table-leaf inline split, mirrored from `read_leaf_cell` so tests can
    /// lay out an overflow row exactly the way the reader expects to find it.
    fn inline_len(usable: usize, payload_len: usize) -> usize {
        let max_local = usable - 35;
        let min_local = (usable - 12) * 32 / 255 - 23;
        let span = usable - 4;
        let k = min_local + (payload_len - min_local) % span;
        if k <= max_local {
            k
        } else {
            min_local
        }
    }

    /// Build a database whose **page 2** is a leaf table b-tree holding a single
    /// row `(rowid, payload)` too big to fit inline: the first `inline` bytes stay
    /// on page 2, the rest spills across overflow pages 3, 4, … each `[u32-be
    /// next-page][content]`. Rooting the leaf on page 2 (not page 1) avoids the
    /// 100-byte header quirk, so the whole leaf content area is available — which
    /// is what real SQLite does for any table other than `sqlite_schema`. Page 1
    /// carries only the database header. Returns the bytes; the leaf root is
    /// page 2. Reserved space is zero, so usable size == page size.
    fn one_overflow_row_db(ps: usize, rowid: i64, payload: &[u8]) -> Vec<u8> {
        let usable = ps;
        assert!(payload.len() > usable - 35, "payload must overflow");
        let inline = inline_len(usable, payload.len());
        let (head, tail) = payload.split_at(inline);

        // Chunk the overflow tail into (usable - 4)-byte content pieces.
        let content = usable - 4;
        let n_overflow = tail.len().div_ceil(content);
        let total_pages = 2 + n_overflow; // page 1 header, page 2 leaf, then overflow
        let first_overflow = 3u32;

        let mut data = vec![0u8; ps * total_pages];

        // --- Page 1: database header only.
        data[0..16].copy_from_slice(MAGIC);
        data[16..18].copy_from_slice(&(ps as u16).to_be_bytes());
        data[56..60].copy_from_slice(&1u32.to_be_bytes()); // UTF-8
        data[28..32].copy_from_slice(&(total_pages as u32).to_be_bytes());

        // --- Page 2: one-cell leaf b-tree (header at offset 0).
        let base = ps;
        data[base] = LEAF_TABLE;
        data[base + 3..base + 5].copy_from_slice(&1u16.to_be_bytes());

        // Cell: [payload-len varint][rowid varint][inline head][u32-be first
        // overflow page].
        let mut cell = Vec::new();
        varint::write(payload.len() as i64, &mut cell);
        varint::write(rowid, &mut cell);
        cell.extend_from_slice(head);
        cell.extend_from_slice(&first_overflow.to_be_bytes());
        let cell_rel = ps - cell.len(); // offset within page 2
        data[base + cell_rel..base + cell_rel + cell.len()].copy_from_slice(&cell);
        data[base + 8..base + 10].copy_from_slice(&(cell_rel as u16).to_be_bytes());
        data[base + 5..base + 7].copy_from_slice(&(cell_rel as u16).to_be_bytes());

        // --- Overflow pages 3..=total_pages.
        for (i, chunk) in tail.chunks(content).enumerate() {
            let page_no = first_overflow as usize + i;
            let ob = (page_no - 1) * ps;
            let next = if i + 1 < n_overflow {
                (page_no + 1) as u32
            } else {
                0 // last page ends the chain
            };
            data[ob..ob + 4].copy_from_slice(&next.to_be_bytes());
            data[ob + 4..ob + 4 + chunk.len()].copy_from_slice(chunk);
        }
        data
    }

    #[test]
    fn reassembles_a_row_that_spills_into_overflow() {
        // A 1500-byte payload on a 512-byte page cannot fit inline; walking must
        // stitch the inline head and the overflow tail back into the exact bytes.
        let payload: Vec<u8> = (0..1500).map(|i| (i % 251) as u8).collect();
        let db = one_overflow_row_db(512, 7, &payload);
        let (header, pager) = Pager::open(&db).unwrap();
        let rows = walk_table(&pager, &header, 2).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, 7);
        assert_eq!(rows[0].1, payload);
    }

    #[test]
    fn overflow_reassembly_spans_several_pages() {
        // A payload several page-widths long exercises a multi-hop chain.
        let payload: Vec<u8> = (0..5000).map(|i| (i * 7 % 256) as u8).collect();
        let db = one_overflow_row_db(512, 1, &payload);
        let (header, pager) = Pager::open(&db).unwrap();
        let rows = walk_table(&pager, &header, 2).unwrap();
        assert_eq!(rows[0].1, payload);
    }

    #[test]
    fn detects_an_overflow_chain_cycle() {
        // Point the first overflow page's next-pointer back at itself: the walk
        // must report a cycle, never spin forever.
        let payload: Vec<u8> = (0..1500).map(|i| i as u8).collect();
        let mut db = one_overflow_row_db(512, 1, &payload);
        // First overflow page is 3 (offset 2*512 = 1024); make it point at itself.
        db[1024..1028].copy_from_slice(&3u32.to_be_bytes());
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 2),
            Err(SqliteError::Corrupt("overflow chain cycle"))
        );
    }

    #[test]
    fn detects_an_overflow_chain_that_ends_too_soon() {
        // Truncate the chain (next = 0 on the first overflow page) before the
        // full payload has been collected.
        let payload: Vec<u8> = (0..1500).map(|i| i as u8).collect();
        let mut db = one_overflow_row_db(512, 1, &payload);
        db[1024..1028].copy_from_slice(&0u32.to_be_bytes()); // page 3 -> chain end
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 2),
            Err(SqliteError::Corrupt("overflow chain ended early"))
        );
    }

    #[test]
    fn rejects_overflow_payload_larger_than_the_file() {
        // A hostile cell claiming a gigantic payload_len must be refused before
        // any reassembly, not drive a huge allocation.
        let payload: Vec<u8> = (0..1500).map(|i| i as u8).collect();
        let mut db = one_overflow_row_db(512, 1, &payload);
        // The leaf cell is on page 2; its page-relative offset is in the cell
        // pointer array at page-2 offset 8 (absolute 512 + 8 = 520).
        let cell_rel = be_u16(&db, 520).unwrap() as usize;
        let cell_off = 512 + cell_rel;
        // Overwrite the payload-length varint with a 5-byte varint for a big
        // number (its length differs, shifting the rowid — the point is only that
        // the huge length trips the file-size guard before any reassembly).
        let mut huge = Vec::new();
        varint::write(1_000_000_000, &mut huge);
        db[cell_off..cell_off + huge.len()].copy_from_slice(&huge);
        let (header, pager) = Pager::open(&db).unwrap();
        assert_eq!(
            walk_table(&pager, &header, 2),
            Err(SqliteError::Corrupt("payload exceeds file size"))
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
