//! Writing a **table b-tree leaf page** — the byte-level inverse of the leaf
//! reader in [`crate::btree`].
//!
//! This is the first rung of the *write* path. [`crate::record::encode`] already
//! turns a row's values into its **record bytes** (the payload of one cell); this
//! module packs a set of `(rowid, record bytes)` cells into a single leaf page,
//! producing bytes that SQLite — and our own reader — accept verbatim.
//!
//! ## The leaf-page layout we emit
//!
//! A table-leaf page (type `0x0D`) is an 8-byte header, then a *cell-pointer
//! array* that grows **downward from the top**, then the cells themselves packed
//! **upward from the bottom**. The two grow toward each other; the gap between
//! them is free space.
//!
//! ```text
//!  offset  bytes  field
//!    0       1    page type = 0x0D (leaf table)
//!    1       2    first-freeblock offset — 0 (we never leave freeblocks)
//!    3       2    cell count N (big-endian u16)
//!    5       2    cell-content-area start — offset of the lowest cell
//!    7       1    fragmented free bytes — 0 (we never fragment)
//!    8     2·N    cell-pointer array: N big-endian u16 offsets, in rowid order
//!    …            free space
//!   top    …      cells, each `[payload-len varint][rowid varint][record bytes]`
//! ```
//!
//! Cells are laid out from the end of the page toward the middle: the first cell
//! (smallest rowid) sits highest, each subsequent cell just below it. The
//! *pointer array* lists them in the same rowid order, so a reader that trusts
//! the pointer array sees rows in key order without sorting. (Our own reader
//! sorts by rowid regardless, so physical order is not load-bearing for
//! round-tripping — but matching SQLite's convention keeps the bytes faithful.)
//!
//! ## Scope of this rung
//!
//! One capability only: **a single leaf page, no overflow, no interior pages, no
//! freeblock coalescing.** A record that would not fit inline (larger than the
//! `usable - 35` local limit the reader uses) or a set of cells that overflows
//! the page is rejected with [`SqliteError::Unsupported`] rather than split —
//! spilling into overflow chains and growing the tree are later rungs.

use crate::error::SqliteError;
use crate::header::{Header, TextEncoding};
use crate::varint;

/// The table-leaf page type byte (matches `btree::LEAF_TABLE`).
const LEAF_TABLE: u8 = 0x0D;

/// SQLite's local-payload limit for a table-leaf cell: a record longer than
/// `usable_size - 35` spills into an overflow chain. We do not emit overflow
/// yet, so any record above this limit is rejected. This mirrors the reader's
/// `max_local = usable.saturating_sub(35)` in [`crate::btree`].
const LEAF_PAYLOAD_OVERHEAD: usize = 35;

/// Encode one table b-tree **leaf page** from `(rowid, record-bytes)` cells.
///
/// `page_size` is the database's page size (a power of two, 512..=65536) and
/// `reserved_space` the per-page reserved tail (usually 0); together they set
/// the usable area, exactly as [`crate::header::Header::usable_size`] does on the
/// read side. The returned `Vec<u8>` is exactly `page_size` bytes.
///
/// The b-tree header is written at **offset 0**, so the result is a standalone
/// page suitable for any page number *other than 1* — page 1 carries the 100-byte
/// database header ahead of its b-tree header, which is a separate rung.
///
/// # Errors
/// - [`SqliteError::BadPageSize`] if `page_size` is not a power of two in
///   `512..=65536`, or `reserved_space` leaves no usable area.
/// - [`SqliteError::Unsupported`] if there are more than 65535 cells, if two
///   cells share a rowid, if any record is too large to store inline (would need
///   an overflow chain), or if the cells do not fit in one page.
pub fn encode_table_leaf_page(
    page_size: usize,
    reserved_space: u8,
    cells: &[(i64, Vec<u8>)],
) -> Result<Vec<u8>, SqliteError> {
    // --- Validate the geometry, mirroring the reader's header checks. ---------
    if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
        return Err(SqliteError::BadPageSize(page_size as u32));
    }
    let reserved = reserved_space as usize;
    if reserved >= page_size {
        return Err(SqliteError::BadPageSize(page_size as u32));
    }
    let usable = page_size - reserved;
    // Largest record that may live inline on a leaf page. `saturating_sub`
    // guards the (rejected-above) degenerate case where the reserved tail eats
    // the whole usable area.
    let max_local = usable.saturating_sub(LEAF_PAYLOAD_OVERHEAD);

    // Order cells by rowid and reject duplicates (see `order_cells`).
    let ordered = order_cells(cells)?;

    let mut page = vec![0u8; page_size];
    fill_table_leaf_page(&mut page, 0, page_size, max_local, &ordered)?;
    Ok(page)
}

/// Write a table-leaf b-tree into `page` with its 8-byte b-tree header at
/// `header_offset` — 0 for an ordinary page, or 100 on page 1 (whose b-tree
/// header follows the 100-byte database header). Cells are packed from the end
/// of the page downward with absolute offsets, and the cell-pointer array grows
/// from `header_offset + 8`. `ordered` must already be rowid-sorted and
/// duplicate-free; `max_local` is the inline-payload limit.
fn fill_table_leaf_page(
    page: &mut [u8],
    header_offset: usize,
    page_size: usize,
    max_local: usize,
    ordered: &[&(i64, Vec<u8>)],
) -> Result<(), SqliteError> {
    let n = ordered.len();

    // --- 8-byte page header. Freeblock (1..3) and fragmented-free (7) stay 0. --
    page[header_offset] = LEAF_TABLE;
    page[header_offset + 3..header_offset + 5].copy_from_slice(&(n as u16).to_be_bytes());

    // The cell-pointer array occupies [header+8, header+8+2·N); cell content must
    // not grow down into it. `content_floor` is the first byte content may not
    // cross.
    let ptr_array = header_offset + 8;
    let content_floor = ptr_array + n * 2;

    // --- Pack cells from the end of the page downward. ------------------------
    let mut content_top = page_size;
    for (i, (rowid, record)) in ordered.iter().enumerate() {
        // No overflow support: the record must fit inline.
        if record.len() > max_local {
            return Err(SqliteError::Unsupported(
                "record too large for one leaf page (overflow not yet supported)",
            ));
        }

        // One cell: [payload-length varint][rowid varint][record bytes].
        let mut cell = Vec::new();
        varint::write(record.len() as i64, &mut cell);
        varint::write(*rowid, &mut cell);
        cell.extend_from_slice(record);

        // Grow the content area downward; refuse to collide with the pointer
        // array (i.e. the page is full).
        content_top = content_top
            .checked_sub(cell.len())
            .filter(|top| *top >= content_floor)
            .ok_or(SqliteError::Unsupported(
                "leaf page overflow: cells do not fit in one page",
            ))?;
        page[content_top..content_top + cell.len()].copy_from_slice(&cell);

        // Record this cell's offset in the pointer array (rowid order).
        let entry = ptr_array + i * 2;
        page[entry..entry + 2].copy_from_slice(&(content_top as u16).to_be_bytes());
    }

    // --- Cell-content-area start (b-tree header offset 5..7). The format stores
    //     65536 as 0 (only an empty 64 KiB page); `as u16` wraps 65536 → 0. -----
    page[header_offset + 5..header_offset + 7].copy_from_slice(&(content_top as u16).to_be_bytes());
    Ok(())
}

/// Sort `(rowid, record)` cells by rowid and reject duplicate rowids — the
/// shared preamble for any table-leaf page. Returns borrowed refs in key order.
fn order_cells(cells: &[(i64, Vec<u8>)]) -> Result<Vec<&(i64, Vec<u8>)>, SqliteError> {
    if cells.len() > u16::MAX as usize {
        return Err(SqliteError::Unsupported(
            "more than 65535 cells on one leaf page",
        ));
    }
    let mut ordered: Vec<&(i64, Vec<u8>)> = cells.iter().collect();
    ordered.sort_by_key(|(rowid, _)| *rowid);
    for pair in ordered.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(SqliteError::Unsupported("duplicate rowid in leaf page"));
        }
    }
    Ok(ordered)
}

/// Emit a complete, re-readable single-table SQLite database file in one call.
///
/// This is the ergonomic capstone of the write path: it wires together
/// [`crate::header::Header::encode`], [`crate::schema::table_schema_row`],
/// [`crate::record::encode`], and the leaf-page writer to produce a two-page
/// file our own reader — and SQLite — accept:
///
/// - **Page 1** holds the 100-byte database header followed (at offset 100) by
///   the `sqlite_schema` b-tree: a single row describing table `table_name`,
///   rooted on page 2, with `create_sql` as its DDL.
/// - **Page 2** is the table's data leaf, holding `rows` (each `(rowid, columns)`
///   encoded with [`crate::record::encode`]).
///
/// `rows` is `(rowid, column values)` per row. The result is exactly
/// `2 * page_size` bytes and reads back through
/// [`crate::schema::read_table`]`(&db, table_name)`.
///
/// # Errors
/// [`SqliteError::BadPageSize`] for a `page_size` that is not a power of two in
/// `512..=65536`; [`SqliteError::Unsupported`] if the schema row or any data row
/// is too large to fit inline on its single page (overflow is not yet emitted),
/// or if there are duplicate rowids.
pub fn write_single_table_db(
    page_size: usize,
    table_name: &str,
    create_sql: &str,
    rows: &[(i64, Vec<crate::record::SqlValue>)],
) -> Result<Vec<u8>, SqliteError> {
    if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
        return Err(SqliteError::BadPageSize(page_size as u32));
    }

    // --- Page 2: the user table's data leaf. ---------------------------------
    let data_cells: Vec<(i64, Vec<u8>)> = rows
        .iter()
        .map(|(rowid, cols)| (*rowid, crate::record::encode(cols)))
        .collect();
    let data_leaf = encode_table_leaf_page(page_size, 0, &data_cells)?;

    // --- Page 1: DB header (offset 0..100) + sqlite_schema leaf (offset 100). -
    let header = Header {
        page_size: page_size as u32,
        reserved_space: 0,
        page_count: 2,
        change_counter: 1,
        freelist_trunk: 0,
        freelist_count: 0,
        schema_cookie: 1,
        schema_format: 1,
        text_encoding: TextEncoding::Utf8,
    };
    let mut page1 = vec![0u8; page_size];
    page1[0..100].copy_from_slice(&header.encode());

    // The schema b-tree carries one cell (rowid 1): the record for the table,
    // rooted on page 2. Its leaf header sits at offset 100 (after the DB header).
    let schema_record =
        crate::record::encode(&crate::schema::table_schema_row(table_name, 2, create_sql));
    let schema_cells = vec![(1i64, schema_record)];
    let ordered = order_cells(&schema_cells)?;
    // Inline limit for page 1's leaf: usable area minus the 100-byte header and
    // the 35-byte cell overhead the reader assumes.
    let max_local = (page_size - 100).saturating_sub(LEAF_PAYLOAD_OVERHEAD);
    fill_table_leaf_page(&mut page1, 100, page_size, max_local, &ordered)?;

    let mut file = page1;
    file.extend_from_slice(&data_leaf);
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btree::walk_table;
    use crate::header::MAGIC;
    use crate::pager::Pager;
    use crate::record::{self, SqlValue};

    /// The one-call assembler produces a database that the real reader resolves
    /// by table name: `write_single_table_db` → `schema::read_table`.
    #[test]
    fn write_single_table_db_round_trips_by_name() {
        let rows = vec![
            (1i64, vec![SqlValue::Int(10), SqlValue::Text("a".into())]),
            (2, vec![SqlValue::Int(20), SqlValue::Text("b".into())]),
            (3, vec![SqlValue::Null, SqlValue::Text("c".into())]),
        ];
        let db = write_single_table_db(512, "widgets", "CREATE TABLE widgets(n, label)", &rows)
            .unwrap();
        assert_eq!(db.len(), 512 * 2);
        assert_eq!(&db[0..16], MAGIC);

        // Reads back by name through the real schema/b-tree reader.
        let read = crate::schema::read_table(&db, "widgets").unwrap();
        assert_eq!(read, rows);

        // The schema resolves the table's root page and DDL.
        let schema = crate::schema::read_schema(&db).unwrap();
        assert_eq!(schema.len(), 1);
        assert_eq!(schema[0].name, "widgets");
        assert_eq!(schema[0].root_page, Some(2));
        assert_eq!(schema[0].sql.as_deref(), Some("CREATE TABLE widgets(n, label)"));
    }

    /// An empty table still yields a valid, readable database (no rows).
    #[test]
    fn write_single_table_db_handles_empty_table() {
        let db = write_single_table_db(512, "t", "CREATE TABLE t(x)", &[]).unwrap();
        assert_eq!(crate::schema::read_table(&db, "t").unwrap(), vec![]);
    }

    /// A bad page size is rejected up front.
    #[test]
    fn write_single_table_db_rejects_bad_page_size() {
        let err = write_single_table_db(1000, "t", "CREATE TABLE t(x)", &[]).unwrap_err();
        assert!(matches!(err, SqliteError::BadPageSize(_)));
    }

    /// Wrap an encoded leaf page as **page 2** of a minimal two-page database, so
    /// the real reader ([`Pager::open`] + [`walk_table`]) can parse it. Page 1
    /// holds only the 100-byte database header (enough for `Pager::open`); page 2
    /// is the leaf page under test. `walk_table(root = 2)` reads page 2 with a
    /// zero header offset — exactly what the encoder emits.
    fn db_with_leaf_as_page2(page_size: usize, leaf: &[u8]) -> Vec<u8> {
        let mut file = vec![0u8; page_size * 2];
        file[0..16].copy_from_slice(MAGIC);
        file[16..18].copy_from_slice(&(page_size as u16).to_be_bytes());
        file[20] = 0; // reserved space
        file[28..32].copy_from_slice(&2u32.to_be_bytes()); // 2 pages
        file[56..60].copy_from_slice(&1u32.to_be_bytes()); // UTF-8
        file[page_size..page_size * 2].copy_from_slice(leaf);
        file
    }

    /// Encode a page, then read it back through the real leaf reader and assert
    /// the rows come out in rowid order — the round-trip gate for this rung.
    #[test]
    fn round_trips_raw_cells_through_the_reader() {
        // Inserted out of rowid order; the encoder sorts and the reader confirms.
        let cells = vec![
            (2i64, vec![0xAA, 0xBB]),
            (1, vec![0x01]),
            (3, vec![0xCC, 0xDD, 0xEE]),
        ];
        let leaf = encode_table_leaf_page(512, 0, &cells).unwrap();
        assert_eq!(leaf.len(), 512);

        let file = db_with_leaf_as_page2(512, &leaf);
        let (header, pager) = Pager::open(&file).unwrap();
        let rows = walk_table(&pager, &header, 2).unwrap();
        assert_eq!(
            rows,
            vec![
                (1, vec![0x01]),
                (2, vec![0xAA, 0xBB]),
                (3, vec![0xCC, 0xDD, 0xEE]),
            ]
        );
    }

    /// End-to-end across both writer rungs: values → `record::encode` → leaf page
    /// → `walk_table` → `record::decode` → values. Proves the record encoder and
    /// the page writer compose into bytes the reader fully understands.
    #[test]
    fn round_trips_encoded_records_end_to_end() {
        let rows = [
            (10i64, vec![SqlValue::Int(42), SqlValue::Text("hello".into())]),
            (5, vec![SqlValue::Null, SqlValue::Real(2.5)]),
            (7, vec![SqlValue::Blob(vec![0xDE, 0xAD])]),
        ];
        let cells: Vec<(i64, Vec<u8>)> = rows
            .iter()
            .map(|(rowid, vals)| (*rowid, record::encode(vals)))
            .collect();

        let leaf = encode_table_leaf_page(4096, 0, &cells).unwrap();
        let file = db_with_leaf_as_page2(4096, &leaf);
        let (header, pager) = Pager::open(&file).unwrap();
        let read = walk_table(&pager, &header, 2).unwrap();

        let decoded: Vec<(i64, Vec<SqlValue>)> = read
            .iter()
            .map(|(rowid, bytes)| (*rowid, record::decode(bytes).unwrap()))
            .collect();

        // Expected is the input sorted by rowid.
        assert_eq!(
            decoded,
            vec![
                (5, vec![SqlValue::Null, SqlValue::Real(2.5)]),
                (7, vec![SqlValue::Blob(vec![0xDE, 0xAD])]),
                (10, vec![SqlValue::Int(42), SqlValue::Text("hello".into())]),
            ]
        );
    }

    /// An empty page encodes with a zero cell count and content-area start at the
    /// page size, and reads back as no rows.
    #[test]
    fn empty_page_round_trips_to_no_rows() {
        let leaf = encode_table_leaf_page(512, 0, &[]).unwrap();
        assert_eq!(leaf[0], LEAF_TABLE);
        assert_eq!(u16::from_be_bytes([leaf[3], leaf[4]]), 0); // cell count
        assert_eq!(u16::from_be_bytes([leaf[5], leaf[6]]), 512); // content start

        let file = db_with_leaf_as_page2(512, &leaf);
        let (header, pager) = Pager::open(&file).unwrap();
        assert_eq!(walk_table(&pager, &header, 2).unwrap(), vec![]);
    }

    /// A record larger than the inline limit is rejected (overflow unsupported)
    /// rather than silently corrupting the page.
    #[test]
    fn rejects_oversized_record() {
        let big = vec![0u8; 512]; // far above 512 - 35 max_local
        let err = encode_table_leaf_page(512, 0, &[(1, big)]).unwrap_err();
        assert!(matches!(err, SqliteError::Unsupported(_)));
    }

    /// Too many small cells to fit in one page is a clean overflow error.
    #[test]
    fn rejects_page_overflow() {
        // Each cell is ~4 bytes; a 512-byte page cannot hold hundreds of them.
        let cells: Vec<(i64, Vec<u8>)> = (0..200).map(|i| (i, vec![0xFF])).collect();
        let err = encode_table_leaf_page(512, 0, &cells).unwrap_err();
        assert!(matches!(err, SqliteError::Unsupported(_)));
    }

    /// Duplicate rowids cannot form a valid table b-tree and are rejected.
    #[test]
    fn rejects_duplicate_rowid() {
        let err =
            encode_table_leaf_page(512, 0, &[(1, vec![0x01]), (1, vec![0x02])]).unwrap_err();
        assert!(matches!(err, SqliteError::Unsupported(_)));
    }

    /// A non-power-of-two page size is rejected up front.
    #[test]
    fn rejects_bad_page_size() {
        let err = encode_table_leaf_page(1000, 0, &[]).unwrap_err();
        assert!(matches!(err, SqliteError::BadPageSize(_)));
    }
}
