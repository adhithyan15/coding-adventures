# Changelog

## [0.5.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — phase 4b: full recursive leaf and interior splits.
  - **Non-root leaf split**: `BTree.insert` now splits any full non-root leaf
    page into two halves, rewrites the existing page with the left half,
    allocates a right sibling, and calls `_push_separator_up` to propagate
    the separator key up the ancestor path. Trees can grow to arbitrary depth.
  - **`_push_separator_up`**: recursively inserts a new separator cell into a
    parent interior page.  If the parent is also full it is split via
    `_split_interior_page` (non-root) or `_split_root_interior` (root), and
    the process repeats with the grandparent.
  - **`_split_interior_page`**: splits a non-root interior page by removing
    the median cell, rewriting the existing page with the left half (rightmost
    child = median.left_child), and allocating a right sibling for the right
    half (rightmost child = original rightmost_child).  Returns
    `(median_sep_rowid, right_pgno)` for the caller to push further up.
  - **`_split_root_interior`**: splits the root interior page when it is full.
    Allocates two new interior children for the left and right halves and
    rewrites the root with a single separator cell.  The root page number never
    changes.
  - **`_find_leaf_with_path`**: extended traversal that records the full
    ancestor path `(pgno, hdr_off, chosen_idx)` from root to leaf's parent.
    `_find_leaf_page` is now a thin wrapper that discards the path.
  - **`_write_interior_page`**: helper that builds an interior page from scratch
    given a sorted cell list and a rightmost-child pointer.
  - **`_interior_cells_fit`** (module-level): checks whether a list of interior
    cells fits within the usable space of an interior page at a given
    `header_offset`, used by `_push_separator_up` to decide whether to split.
  - **`_split_leaf`**: splits a non-root leaf page in-place (rewrites existing
    page with left half, allocates right sibling with right half).
  - `PageFullError` is retained in the public API but is no longer raised by
    `BTree.insert` during normal operation.  Recursive splits handle all
    leaf-overflow cases transparently.

### Changed

- `insert` now uses `_find_leaf_with_path` instead of `_find_leaf_page` so
  that the ancestor path is available when a non-root leaf is full.
- Module docstring updated to document the interior split algorithm, the new
  helper methods, and the v1 limitation that orphaned overflow pages from
  splits are not reclaimed until phase 5 (freelist).

## [0.4.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — phase 4a: interior page traversal + root-leaf split.
  - **Interior page support**: `_read_hdr` now returns `"rightmost_child"` for
    interior pages (type `0x05`). New helpers `_write_interior_hdr`,
    `_read_interior_ptrs`, `_read_interior_cell`, `_interior_cell_encode` expose
    the full interior-page format for callers and tests.
  - **Root-leaf split**: `BTree.insert` automatically promotes the root page from
    a leaf to an interior page when the leaf fills up, distributing cells across
    two freshly allocated child pages. The root page number never changes.
  - **Multi-level traversal**: `find`, `scan`, `delete`, `update`, and
    `cell_count` all traverse interior pages to reach the correct leaf. `scan`
    performs a full left-to-right DFS over all leaves, yielding rows in ascending
    rowid order.
  - **Cycle and depth guards** in all traversal paths: `_find_leaf_page` uses a
    depth counter (limit `_MAX_BTREE_DEPTH = 20`); `_scan_page` uses a visited
    set; both raise `CorruptDatabaseError` on corrupt trees.
  - **Child-pointer validation** in `_find_leaf_page` and `_scan_page`: any
    child pointer of 0 or beyond the pager's current page count raises
    `CorruptDatabaseError` before dereferencing.
  - `PageFullError` is now only raised for *non-root* leaf overflows; root-leaf
    overflow is resolved silently by the split. Phase 4b will extend splitting to
    non-root leaves.
  - `free_space()` updated to account for the wider 12-byte interior header when
    the root is an interior page.
  - `header_offset=100` (page-1 support) correctly preserved through the
    root-split rewrite (the 100-byte SQLite database header prefix is not zeroed).

## [0.3.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — table B-tree leaf pages (phase 3).
  - `BTree.create(pager)` allocates and initialises a fresh empty leaf page.
  - `BTree.open(pager, root_page)` attaches to an existing page.
  - `insert(rowid, payload)` — sorted insert; raises `DuplicateRowidError` on
    collision, `PageFullError` when the leaf is full (splits land in phase 4).
  - `find(rowid)` — binary search over the sorted cell-pointer array; returns
    raw record bytes or `None`.
  - `scan()` — iterator over `(rowid, payload)` in ascending rowid order.
  - `delete(rowid)` — removes the cell and compacts the page in-place.
  - `update(rowid, payload)` — delete + re-insert; handles payload size changes.
  - Overflow chains: records larger than `max_local` (4 061 bytes for 4 096-byte
    pages) spill to linked overflow pages per the SQLite formula; reads and
    writes follow the chain transparently.
  - `header_offset=100` support for page 1 (database header occupies bytes 0–99).
  - `BTreeError`, `PageFullError`, `DuplicateRowidError` exported from package root.

## [0.2.0] - 2026-04-19

### Added

- `storage_sqlite.varint` — SQLite's 1..9 byte big-endian varint codec.
  `encode(value)`, `decode(data, offset)`, `encode_signed`, `decode_signed`,
  `size(value)`. Produces the shortest form for every value (required for
  byte-compat round-trip). Full u64 range supported; signed helpers span
  the i64 range via two's complement.
- `storage_sqlite.record` — record codec mapping row values ↔ bytes.
  Supports the nine value-bearing serial types (NULL, int8/16/24/32/48/64,
  float64 BE, constant-0, constant-1), plus BLOB (even serial types ≥ 12)
  and TEXT (odd serial types ≥ 13). Encoder picks the *smallest* serial
  type for every integer — matching sqlite3's byte-compat behaviour, so a
  record of `[None, 7, "hi"]` encodes to the exact same bytes sqlite3
  writes. Decoder rejects reserved serial types (10, 11), truncated
  payloads, and inconsistent header lengths.
- `Value` type alias exported at the package root for record values.

## [0.1.0] - 2026-04-19

### Added

- Initial release — phase 1 of the SQLite byte-compatible file backend.
- `storage_sqlite.header.Header` — dataclass for the 100-byte SQLite
  database header. Field-by-field layout per the v3 spec; `from_bytes`
  parses, `to_bytes` serialises, `new_database` constructs a fresh header
  with the conventional defaults (page size 4096, UTF-8, schema format 4).
  Validates magic string, page size (power-of-two, 512..65536), and text
  encoding on parse.
- `storage_sqlite.pager.Pager` — page-at-a-time I/O against a file, with:
  - 1-based page numbers (page 0 is never read or written).
  - Fixed page size of 4096 in v1.
  - Small LRU page cache (default 32 pages).
  - `allocate()` to claim a fresh page number at the file tail.
  - Rollback journal at `<db>-journal`: on first write to a page within a
    transaction, the original page contents are copied to the journal.
    `commit()` fsyncs the journal, applies staged writes to the main file,
    fsyncs the main file, then deletes the journal. `rollback()` discards
    the staged writes and the journal.
  - Context-manager protocol (`with Pager.open(path) as pager:`).
  - Crash-recovery on `open`: if a hot journal is present, its contents are
    replayed back into the main file before the pager becomes usable.
