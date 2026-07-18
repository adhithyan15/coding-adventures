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

/// The interior table-b-tree page type byte (matches `btree::INTERIOR_TABLE`).
/// An interior page holds no rows — only child pointers plus divider rowids.
const INTERIOR_TABLE: u8 = 0x05;

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
    // Build each cell's bytes: [payload-length varint][rowid varint][record].
    // This entry point has no overflow support (it cannot allocate overflow
    // pages), so a record that would not fit inline is rejected — the
    // whole-database writer handles spilling instead (see `build_leaf_cell`).
    let mut cells = Vec::with_capacity(ordered.len());
    for (rowid, record) in ordered {
        if record.len() > max_local {
            return Err(SqliteError::Unsupported(
                "record too large for one leaf page (overflow not yet supported)",
            ));
        }
        let mut cell = Vec::new();
        varint::write(record.len() as i64, &mut cell);
        varint::write(*rowid, &mut cell);
        cell.extend_from_slice(record);
        cells.push(cell);
    }
    pack_leaf_cells(page, header_offset, page_size, &cells)
}

/// Pack already-built table-leaf **cell bytes** (each a complete
/// `[payload-len varint][rowid varint][inline payload][…]`) into `page`, with
/// the 8-byte b-tree header at `header_offset`. Cells are laid out from the end
/// of the page downward with absolute offsets; the cell-pointer array grows from
/// `header_offset + 8` in the given order (which the caller must have sorted by
/// rowid). Shared by the inline-only [`fill_table_leaf_page`] and the
/// overflow-aware whole-database writer, so both produce byte-identical page
/// framing.
fn pack_leaf_cells(
    page: &mut [u8],
    header_offset: usize,
    page_size: usize,
    cells: &[Vec<u8>],
) -> Result<(), SqliteError> {
    let n = cells.len();

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
    for (i, cell) in cells.iter().enumerate() {
        // Grow the content area downward; refuse to collide with the pointer
        // array (i.e. the page is full).
        content_top = content_top
            .checked_sub(cell.len())
            .filter(|top| *top >= content_floor)
            .ok_or(SqliteError::Unsupported(
                "leaf page overflow: cells do not fit in one page",
            ))?;
        page[content_top..content_top + cell.len()].copy_from_slice(cell);

        // Record this cell's offset in the pointer array (rowid order).
        let entry = ptr_array + i * 2;
        page[entry..entry + 2].copy_from_slice(&(content_top as u16).to_be_bytes());
    }

    // --- Cell-content-area start (b-tree header offset 5..7). The format stores
    //     65536 as 0 (only an empty 64 KiB page); `as u16` wraps 65536 → 0. -----
    page[header_offset + 5..header_offset + 7].copy_from_slice(&(content_top as u16).to_be_bytes());
    Ok(())
}

/// Number of a record's payload bytes that stay **inline** on a table-leaf cell,
/// the exact inverse of the reader's `split_and_reassemble`. For a payload that
/// fits (`<= max_local`) that is its whole length; otherwise SQLite's surplus
/// formula keeps
///
/// ```text
///   M = ((U - 12) * 32 / 255) - 23         (inline floor)
///   K = M + ((P - M) mod (U - 4))          (candidate inline length)
///   inline = if K <= X { K } else { M }    (X = max_local)
/// ```
///
/// bytes inline and spills the rest into an overflow chain. `usable` is `U`.
fn table_leaf_inline_len(payload_len: usize, usable: usize, max_local: usize) -> usize {
    if payload_len <= max_local {
        return payload_len;
    }
    let min_local = (usable.saturating_sub(12).saturating_mul(32) / 255).saturating_sub(23);
    // `usable >= 512` (validated by callers), so `span > 0`.
    let span = usable - 4;
    let k = min_local + (payload_len - min_local) % span;
    if k <= max_local {
        k
    } else {
        min_local
    }
}

/// Build one table-leaf cell, spilling into an overflow chain when the record is
/// too large to sit inline. Returns the cell bytes plus any overflow pages the
/// record produced (empty when it fits inline).
///
/// The cell is always `[payload-len varint = full record length P][rowid varint]
/// [first `inline` payload bytes]`, and when the record overflows a trailing
/// 4-byte big-endian pointer to `first_overflow_page` follows the inline bytes —
/// exactly what the reader's `read_leaf_cell` expects. Overflow pages are
/// numbered `first_overflow_page, +1, …`, each `[u32-be next-page][content]`
/// with `next = 0` on the last, matching `follow_overflow`.
fn build_leaf_cell(
    rowid: i64,
    record: &[u8],
    page_size: usize,
    usable: usize,
    max_local: usize,
    first_overflow_page: u32,
) -> (Vec<u8>, Vec<Vec<u8>>) {
    let payload_len = record.len();
    let inline = table_leaf_inline_len(payload_len, usable, max_local);

    let mut cell = Vec::new();
    varint::write(payload_len as i64, &mut cell);
    varint::write(rowid, &mut cell);
    cell.extend_from_slice(&record[..inline]);

    if inline == payload_len {
        return (cell, Vec::new());
    }

    // Spill the tail across a chain of overflow pages, then point the cell at
    // the first page in that chain.
    cell.extend_from_slice(&first_overflow_page.to_be_bytes());
    let overflow = encode_overflow_pages(&record[inline..], page_size, usable, first_overflow_page);
    (cell, overflow)
}

/// Encode the overflow chain carrying `tail` (the payload bytes that did not fit
/// inline). Each page is `page_size` bytes: a 4-byte big-endian next-page pointer
/// (`0` on the last page) followed by up to `usable - 4` content bytes, the rest
/// zero-padded. Pages are numbered `first_page, first_page + 1, …`.
fn encode_overflow_pages(
    tail: &[u8],
    page_size: usize,
    usable: usize,
    first_page: u32,
) -> Vec<Vec<u8>> {
    let content_per_page = usable - 4;
    let mut pages = Vec::new();
    let mut chunks = tail.chunks(content_per_page).peekable();
    let mut page_no = first_page;
    while let Some(chunk) = chunks.next() {
        let mut page = vec![0u8; page_size];
        let next = if chunks.peek().is_some() {
            page_no + 1
        } else {
            0
        };
        page[0..4].copy_from_slice(&next.to_be_bytes());
        page[4..4 + chunk.len()].copy_from_slice(chunk);
        pages.push(page);
        page_no += 1;
    }
    pages
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
    // A single-table database is just the one-table case of the general writer.
    // Delegating keeps the two byte-for-byte identical (the sole table roots on
    // page 2, exactly as before).
    write_multi_table_db(page_size, &[(table_name, create_sql, rows)])
}

/// One table's rows, as `(name, create_sql, rows)` — the unit
/// [`write_multi_table_db`] writes. `rows` is `(rowid, column values)` per row.
pub type TableSpec<'a> = (&'a str, &'a str, &'a [(i64, Vec<crate::record::SqlValue>)]);

/// Emit a complete, re-readable SQLite database holding **several** tables in one
/// call — the multi-table generalisation of [`write_single_table_db`].
///
/// ## Page layout
///
/// - **Page 1** is the 100-byte DB header followed (at offset 100) by the
///   `sqlite_schema` leaf: one row per table, in the given order, each naming
///   the table's root page and DDL.
/// - **Pages 2…** hold each table's data, table by table in order: a data leaf
///   followed immediately by that table's overflow pages (for rows too large to
///   sit inline), then the next table's leaf, and so on. A table's root page is
///   therefore wherever its leaf lands after the previous tables' pages.
///
/// The result reads back through [`crate::schema::read_table`]`(&db, name)` for
/// each table and is accepted by real SQLite (it passes `PRAGMA
/// integrity_check`). Reserved space is 0, so usable size equals `page_size`.
///
/// # Errors
/// [`SqliteError::BadPageSize`] for a `page_size` outside the power-of-two
/// `512..=65536` range; [`SqliteError::Unsupported`] if a duplicate rowid
/// appears within a table, if the combined `sqlite_schema` rows do not fit on
/// page 1's single leaf (page-1 schema overflow is a later rung), or if the
/// database would exceed the 2³²-page limit.
pub fn write_multi_table_db(page_size: usize, tables: &[TableSpec]) -> Result<Vec<u8>, SqliteError> {
    if !(512..=65536).contains(&page_size) || !page_size.is_power_of_two() {
        return Err(SqliteError::BadPageSize(page_size as u32));
    }
    // `reserved_space` is 0, so usable == page_size.
    let usable = page_size;
    let data_max_local = usable.saturating_sub(LEAF_PAYLOAD_OVERHEAD);

    // --- Encode every table's pages, assigning root pages sequentially from 2.
    // Each table contributes one data leaf followed by its overflow pages; the
    // next table roots right after them. `schema_rows` records (name, root, sql)
    // for the page-1 schema leaf we build once all roots are known.
    let mut next_page: u32 = 2;
    let mut table_pages: Vec<Vec<u8>> = Vec::new();
    let mut schema_rows: Vec<(i64, Vec<u8>)> = Vec::with_capacity(tables.len());
    for (i, (name, create_sql, rows)) in tables.iter().enumerate() {
        let root_page = next_page;
        // A table is a whole b-tree now: a single leaf when it fits, otherwise an
        // interior root over several leaves. Every page it produces is allocated
        // contiguously from `root_page`, so the next table roots right after.
        let pages = encode_table_btree(page_size, usable, data_max_local, root_page, rows)?;
        next_page = u32::try_from(root_page as usize + pages.len())
            .map_err(|_| SqliteError::Unsupported("database exceeds the 2^32-page limit"))?;
        schema_rows.push((
            (i + 1) as i64,
            crate::record::encode(&crate::schema::table_schema_row(
                name,
                root_page,
                create_sql,
            )),
        ));
        table_pages.extend(pages);
    }

    // Total pages: page 1 (schema) + every table/overflow page.
    let total_pages = u32::try_from(1 + table_pages.len())
        .map_err(|_| SqliteError::Unsupported("database exceeds the 2^32-page limit"))?;

    // --- Page 1: DB header (offset 0..100) + sqlite_schema leaf (offset 100). -
    let header = Header {
        page_size: page_size as u32,
        reserved_space: 0,
        page_count: total_pages,
        change_counter: 1,
        freelist_trunk: 0,
        freelist_count: 0,
        schema_cookie: 1,
        schema_format: 1,
        text_encoding: TextEncoding::Utf8,
    };
    let mut page1 = vec![0u8; page_size];
    page1[0..100].copy_from_slice(&header.encode());

    // The schema b-tree leaf sits at offset 100 (after the DB header). Its inline
    // limit is the usable area minus that 100-byte prefix and the 35-byte cell
    // overhead; schema rows that would overflow it are rejected (a later rung).
    let ordered_schema = order_cells(&schema_rows)?;
    let schema_max_local = (page_size - 100).saturating_sub(LEAF_PAYLOAD_OVERHEAD);
    fill_table_leaf_page(&mut page1, 100, page_size, schema_max_local, &ordered_schema)?;

    // --- Assemble: page 1, then every table's pages in allocation order. ------
    let mut file = page1;
    for page in &table_pages {
        file.extend_from_slice(page);
    }
    Ok(file)
}

/// Encode one table's data leaf (rooted at `root_page`) plus its overflow pages,
/// which are numbered from `root_page + 1`. Returns `(leaf_bytes,
/// overflow_pages)` — the leaf is exactly `page_size` bytes and each overflow
/// page likewise. Shared by every table in [`write_multi_table_db`].
fn encode_table_btree(
    page_size: usize,
    usable: usize,
    data_max_local: usize,
    root_page: u32,
    rows: &[(i64, Vec<crate::record::SqlValue>)],
) -> Result<Vec<Vec<u8>>, SqliteError> {
    let records: Vec<(i64, Vec<u8>)> = rows
        .iter()
        .map(|(rowid, cols)| (*rowid, crate::record::encode(cols)))
        .collect();
    let ordered = order_cells(&records)?;

    // Partition the rowid-ordered cells into leaf pages. A cell's footprint is
    // its bytes plus a 2-byte cell-pointer entry; a leaf's 8-byte header leaves
    // `usable - 8` for the pointer array and cells. The overflow *page number*
    // does not affect a cell's length (the inline pointer is always 4 bytes), so
    // we can size cells with a placeholder before assigning page numbers.
    let leaf_content = usable - 8;
    let mut leaves: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_size = 0usize;
    for (idx, (rowid, record)) in ordered.iter().enumerate() {
        let (cell, _spilled) =
            build_leaf_cell(*rowid, record, page_size, usable, data_max_local, 0);
        let footprint = cell.len() + 2;
        if !current.is_empty() && current_size + footprint > leaf_content {
            leaves.push(std::mem::take(&mut current));
            current_size = 0;
        }
        current.push(idx);
        current_size += footprint;
    }
    // An empty table still has one (empty) leaf.
    leaves.push(current);

    // A table that fits on a single leaf keeps the flat root-is-leaf layout, so
    // its bytes stay identical to the pre-tree-growth writer.
    if leaves.len() == 1 {
        let (leaf, overflow) = encode_single_leaf(
            page_size,
            usable,
            data_max_local,
            root_page,
            &ordered,
            &leaves[0],
        )?;
        let mut pages = vec![leaf];
        pages.extend(overflow);
        return Ok(pages);
    }

    // Multiple leaves: the table grows into a b-tree whose ROOT is an interior
    // page at `root_page`. `pages[0]` is reserved for that root; every other page
    // (leaves, their overflow chains, and any *intermediate* interior levels) is
    // allocated contiguously from `root_page + 1` in the order it is pushed, so
    // the invariant `pages[i]` ⇔ page number `root_page + i` always holds.
    //
    // We build the tree BOTTOM-UP, one interior level at a time:
    //   level 0 = the data leaves,
    //   level 1 = interior pages whose children are leaves,
    //   level 2 = interior pages whose children are level-1 interiors, …
    // until a level collapses to a single node — that node is the root and is
    // written into `pages[0]`. One interior level (the common case) reproduces
    // the previous single-root layout exactly.
    let mut pages: Vec<Vec<u8>> = vec![Vec::new()];
    let mut next_page = root_page + 1;

    // Level 0: encode each leaf (plus its overflow pages) and record
    // `(page, largest-rowid-in-subtree)` — the raw material for divider cells.
    let mut level: Vec<(u32, i64)> = Vec::with_capacity(leaves.len());
    for leaf_indices in &leaves {
        let leaf_page = next_page;
        let (leaf, overflow) =
            encode_single_leaf(page_size, usable, data_max_local, leaf_page, &ordered, leaf_indices)?;
        // Advance past the leaf itself (+1) and its overflow pages, checked so a
        // table crossing the 2^32-page limit fails cleanly instead of wrapping —
        // consistent with the interior loop's `checked_add`.
        next_page = u32::try_from(leaf_page as usize + 1 + overflow.len())
            .map_err(|_| SqliteError::Unsupported("database exceeds the 2^32-page limit"))?;
        let max_rowid = leaf_indices
            .iter()
            .map(|&i| ordered[i].0)
            .max()
            .unwrap_or(0);
        pages.push(leaf);
        pages.extend(overflow);
        level.push((leaf_page, max_rowid));
    }

    // Build interior levels until one root remains. `group_interior_children`
    // packs the current level's nodes into as few interior pages as fit, each
    // group becoming one parent node. A parent's key (for the level above) is the
    // largest rowid in its whole subtree — i.e. its last child's key, since the
    // nodes stay rowid-ordered throughout.
    loop {
        let groups = group_interior_children(usable, &level);
        // Root level: a single group is the tree root — write it into `pages[0]`
        // rather than allocating a fresh page, so the root lands on `root_page`.
        if groups.len() == 1 {
            pages[0] = pack_interior_from_children(page_size, &groups[0])?;
            break;
        }
        // Progress guard: a level with >1 node must collapse to strictly fewer
        // parents, or the loop can't terminate. This holds for every real page
        // size (≥512 bytes leaves room for dozens of dividers per interior
        // page), so it can only fail if usable space were pathologically small —
        // fail loudly instead of looping forever.
        if groups.len() >= level.len() {
            return Err(SqliteError::Unsupported(
                "interior page too small to reduce a b-tree level",
            ));
        }

        // Intermediate level: allocate one interior page per group.
        let mut parents: Vec<(u32, i64)> = Vec::with_capacity(groups.len());
        for group in &groups {
            let interior_page = next_page;
            next_page = next_page
                .checked_add(1)
                .ok_or(SqliteError::Unsupported("database exceeds the 2^32-page limit"))?;
            let parent_key = group.last().expect("group is never empty").1;
            pages.push(pack_interior_from_children(page_size, group)?);
            parents.push((interior_page, parent_key));
        }
        level = parents;
    }
    Ok(pages)
}

/// Partition a level's child nodes (each `(page, key)`, in rowid order) into the
/// groups that will each become one interior page. A child contributes a
/// divider cell unless it is the right-most child of its group (that pointer
/// lives in the page header, not a cell). We size conservatively — charging
/// *every* child a divider-cell footprint, including the right-most — so the
/// group is guaranteed to fit `pack_interior_page`'s exact packing, at the cost
/// of one unused cell's slack per page (negligible, and it keeps the split
/// decision independent of which child ends up last).
///
/// Each group holds at least one child, so this always makes progress: the next
/// level has strictly fewer nodes whenever the current level has more than one.
fn group_interior_children(usable: usize, children: &[(u32, i64)]) -> Vec<Vec<(u32, i64)>> {
    // Interior header is 12 bytes; the cell-pointer array + cells share the rest.
    // A divider cell is `[u32 child][varint key]` (≤ 13 bytes) plus a 2-byte
    // pointer. Bound the content region by `usable` (≤ page_size) to stay clear
    // of any reserved tail region.
    let capacity = usable.saturating_sub(12);
    let mut groups: Vec<Vec<(u32, i64)>> = Vec::new();
    let mut current: Vec<(u32, i64)> = Vec::new();
    let mut used = 0usize;
    for &(child, key) in children {
        // Divider cell length for this child: 4-byte child pointer + key varint.
        let mut cell = Vec::with_capacity(13);
        cell.extend_from_slice(&child.to_be_bytes());
        varint::write(key, &mut cell);
        let footprint = cell.len() + 2; // + cell-pointer array entry
        if !current.is_empty() && used + footprint > capacity {
            groups.push(std::mem::take(&mut current));
            used = 0;
        }
        current.push((child, key));
        used += footprint;
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

/// Pack one interior page from an ordered group of child nodes: all but the last
/// child become divider cells (`key` = largest rowid in that child's subtree),
/// and the last child is the right-most-child pointer.
fn pack_interior_from_children(
    page_size: usize,
    group: &[(u32, i64)],
) -> Result<Vec<u8>, SqliteError> {
    let (rightmost_page, _) = *group.last().expect("interior group is never empty");
    let dividers = &group[..group.len() - 1];
    pack_interior_page(page_size, dividers, rightmost_page)
}

/// Build one data leaf (rooted at `leaf_page`) from the `ordered` cells selected
/// by `indices`, spilling oversized records into overflow pages numbered from
/// `leaf_page + 1`. Returns `(leaf_bytes, overflow_pages)`.
fn encode_single_leaf(
    page_size: usize,
    usable: usize,
    data_max_local: usize,
    leaf_page: u32,
    ordered: &[&(i64, Vec<u8>)],
    indices: &[usize],
) -> Result<(Vec<u8>, Vec<Vec<u8>>), SqliteError> {
    let mut next_overflow_page = leaf_page + 1;
    let mut overflow_pages: Vec<Vec<u8>> = Vec::new();
    let mut leaf_cells: Vec<Vec<u8>> = Vec::with_capacity(indices.len());
    for &i in indices {
        let (rowid, record) = ordered[i];
        let (cell, spilled) = build_leaf_cell(
            *rowid,
            record,
            page_size,
            usable,
            data_max_local,
            next_overflow_page,
        );
        next_overflow_page += spilled.len() as u32;
        overflow_pages.extend(spilled);
        leaf_cells.push(cell);
    }

    let mut leaf = vec![0u8; page_size];
    pack_leaf_cells(&mut leaf, 0, page_size, &leaf_cells)?;
    Ok((leaf, overflow_pages))
}

/// Pack an **interior table b-tree page** (type `0x05`) from its divider cells
/// and right-most child. Each divider is `(left-child page, key)` where `key` is
/// the largest rowid reachable through that child; the cell bytes are
/// `[u32-be left-child][varint key]`, mirroring the reader's `INTERIOR_TABLE`
/// walk. The 12-byte header carries the cell count and the right-most child at
/// offset 8; the cell-pointer array grows from offset 12 and cells pack upward
/// from the bottom, exactly like a leaf.
///
/// # Errors
/// [`SqliteError::Unsupported`] if the dividers exceed 65535 cells, or if they do
/// not fit on one interior page. Callers building multi-level trees pre-split
/// their children with [`group_interior_children`] so each group fits, so the
/// size error is an internal invariant guard rather than a reachable limit.
fn pack_interior_page(
    page_size: usize,
    dividers: &[(u32, i64)],
    rightmost: u32,
) -> Result<Vec<u8>, SqliteError> {
    let n = dividers.len();
    if n > u16::MAX as usize {
        return Err(SqliteError::Unsupported("more than 65535 interior cells"));
    }
    let mut page = vec![0u8; page_size];
    page[0] = INTERIOR_TABLE;
    page[3..5].copy_from_slice(&(n as u16).to_be_bytes());
    page[8..12].copy_from_slice(&rightmost.to_be_bytes());

    let ptr_array = 12;
    let content_floor = ptr_array + n * 2;
    let mut content_top = page_size;
    for (i, (child, key)) in dividers.iter().enumerate() {
        let mut cell = Vec::with_capacity(13);
        cell.extend_from_slice(&child.to_be_bytes());
        varint::write(*key, &mut cell);
        content_top = content_top.checked_sub(cell.len()).filter(|top| *top >= content_floor).ok_or(
            SqliteError::Unsupported(
                "too many child leaves for one interior page (multi-level tree not yet supported)",
            ),
        )?;
        page[content_top..content_top + cell.len()].copy_from_slice(&cell);
        let entry = ptr_array + i * 2;
        page[entry..entry + 2].copy_from_slice(&(content_top as u16).to_be_bytes());
    }
    page[5..7].copy_from_slice(&(content_top as u16).to_be_bytes());
    Ok(page)
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

    /// A record larger than the inline limit spills into an overflow chain, and
    /// the whole database still round-trips through the real reader — including
    /// the overflow-reassembly path. The big value spans several overflow pages
    /// (512-byte page → 508 content bytes each), interleaved with inline rows to
    /// prove page-number allocation stays correct.
    #[test]
    fn write_single_table_db_spills_large_record_into_overflow() {
        let big = "x".repeat(4000); // ≫ max_local (512 − 35 = 477)
        let rows = vec![
            (1i64, vec![SqlValue::Int(1), SqlValue::Text("small".into())]),
            (2, vec![SqlValue::Int(2), SqlValue::Text(big.clone())]),
            (3, vec![SqlValue::Int(3), SqlValue::Text("also small".into())]),
        ];
        let db = write_single_table_db(512, "docs", "CREATE TABLE docs(n, body)", &rows).unwrap();

        // Page 1 + data leaf (page 2) + one or more overflow pages. The file is a
        // whole number of pages and the header's page count must equal the actual
        // page count (SQLite validates this).
        assert_eq!(db.len() % 512, 0);
        let pages = db.len() / 512;
        assert!(pages > 2, "overflow pages must have been allocated, got {pages}");
        let header = crate::header::Header::parse(&db).unwrap();
        assert_eq!(header.page_count as usize, pages);

        // The reader reassembles the overflow row in full alongside the inline
        // rows, byte-for-byte.
        let read = crate::schema::read_table(&db, "docs").unwrap();
        assert_eq!(read, rows);
    }

    /// Several tables in one database: each roots on its own leaf, and the
    /// reader resolves every table by name. One table carries an overflow row to
    /// prove root-page allocation stays correct across a table that consumes
    /// extra (overflow) pages before the next table's leaf.
    #[test]
    fn write_multi_table_db_round_trips_each_table() {
        let big = "y".repeat(3000); // forces table `docs` past one leaf via overflow
        let widgets = vec![
            (1i64, vec![SqlValue::Int(10), SqlValue::Text("a".into())]),
            (2, vec![SqlValue::Int(20), SqlValue::Text("b".into())]),
        ];
        let docs = vec![
            (1i64, vec![SqlValue::Text("small".into())]),
            (2, vec![SqlValue::Text(big.clone())]),
        ];
        let gadgets = vec![(1i64, vec![SqlValue::Int(99)])];
        let tables: &[TableSpec] = &[
            ("widgets", "CREATE TABLE widgets(n, label)", &widgets),
            ("docs", "CREATE TABLE docs(body)", &docs),
            ("gadgets", "CREATE TABLE gadgets(k)", &gadgets),
        ];
        let db = write_multi_table_db(512, tables).unwrap();

        // Header page count agrees with the file length.
        let pages = db.len() / 512;
        let header = crate::header::Header::parse(&db).unwrap();
        assert_eq!(header.page_count as usize, pages);

        // Every table reads back by name, overflow row reassembled.
        assert_eq!(crate::schema::read_table(&db, "widgets").unwrap(), widgets);
        assert_eq!(crate::schema::read_table(&db, "docs").unwrap(), docs);
        assert_eq!(crate::schema::read_table(&db, "gadgets").unwrap(), gadgets);

        // The schema lists all three, with distinct ascending-ish root pages.
        let schema = crate::schema::read_schema(&db).unwrap();
        let names: Vec<&str> = schema.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["widgets", "docs", "gadgets"]);
        assert_eq!(schema[0].root_page, Some(2));
        // `docs` roots after widgets' single leaf (page 3); `gadgets` after
        // docs' leaf + its overflow pages.
        assert_eq!(schema[1].root_page, Some(3));
        assert!(schema[2].root_page.unwrap() > 3);
    }

    /// A table with more rows than fit on one leaf grows a b-tree: several data
    /// leaves under an interior root page. The reader walks the whole tree and
    /// returns every row in rowid order.
    #[test]
    fn write_table_grows_a_btree_across_multiple_leaves() {
        // ~300 small rows on a 512-byte page force several leaves + an interior
        // root (a 512-byte leaf holds only a few dozen small cells).
        let rows: Vec<(i64, Vec<SqlValue>)> = (1..=300)
            .map(|n| (n, vec![SqlValue::Int(n * 10), SqlValue::Text(format!("r{n}"))]))
            .collect();
        let db = write_single_table_db(512, "big", "CREATE TABLE big(n, label)", &rows).unwrap();

        let pages = db.len() / 512;
        let header = crate::header::Header::parse(&db).unwrap();
        assert_eq!(header.page_count as usize, pages);
        assert!(pages > 3, "expected multiple leaves + interior, got {pages} pages");

        // The table's root page is now an *interior* table page (type 0x05).
        let schema = crate::schema::read_schema(&db).unwrap();
        let root = schema[0].root_page.unwrap() as usize;
        let root_off = (root - 1) * 512;
        assert_eq!(db[root_off], 0x05, "root should be an interior table page");

        // Every row reads back in order, through the interior→leaf descent.
        let read = crate::schema::read_table(&db, "big").unwrap();
        assert_eq!(read, rows);
    }

    /// Enough rows that even the *interior* level overflows one page, forcing a
    /// **multi-level** tree: root interior → intermediate interiors → leaves. The
    /// reader's stack-based descent walks all levels and returns every row in
    /// order.
    #[test]
    fn write_table_grows_a_multi_level_btree() {
        // 3000 small rows on a 512-byte page produce ~90 leaves; a 512-byte
        // interior page holds only a few dozen dividers, so the leaves can't be
        // covered by a single interior page — a second interior level is needed.
        let rows: Vec<(i64, Vec<SqlValue>)> = (1..=3000)
            .map(|n| (n, vec![SqlValue::Int(n), SqlValue::Text(format!("r{n}"))]))
            .collect();
        let db = write_single_table_db(512, "big", "CREATE TABLE big(n, label)", &rows).unwrap();

        let pages = db.len() / 512;
        let header = crate::header::Header::parse(&db).unwrap();
        assert_eq!(header.page_count as usize, pages);

        // The root is an interior page (0x05) whose FIRST child is ALSO an
        // interior page — i.e. the tree is at least three levels deep.
        let schema = crate::schema::read_schema(&db).unwrap();
        let root = schema[0].root_page.unwrap() as usize;
        let root_off = (root - 1) * 512;
        assert_eq!(db[root_off], 0x05, "root should be an interior table page");
        // First divider cell's left-child pointer lives at the cell offset named
        // by the first cell-pointer array slot (interior header is 12 bytes).
        let first_ptr = u16::from_be_bytes([db[root_off + 12], db[root_off + 13]]) as usize;
        let first_child = u32::from_be_bytes([
            db[root_off + first_ptr],
            db[root_off + first_ptr + 1],
            db[root_off + first_ptr + 2],
            db[root_off + first_ptr + 3],
        ]) as usize;
        let child_off = (first_child - 1) * 512;
        assert_eq!(
            db[child_off], 0x05,
            "root's child should also be interior (≥3-level tree)"
        );

        // Every row reads back in rowid order through the full multi-level descent.
        let read = crate::schema::read_table(&db, "big").unwrap();
        assert_eq!(read, rows);
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
