"""
Index B-tree pages — phase IX-1: IndexTree for automatic index support.

Architecture summary
--------------------

SQLite uses two additional B-tree page subtypes for index storage:

* **0x0A** — index leaf page
* **0x02** — index interior page

These share the same physical page layout as table B-tree pages (same 8-byte
leaf / 12-byte interior header format, same cell-pointer array, same
cell-content area), but differ in the meaning and format of their cells.

The fundamental difference from a table B-tree
----------------------------------------------

In a **table** B-tree the sort key is the *integer rowid*, stored as a signed
varint in each cell header. The payload is the row's column values encoded as
a record.

In an **index** B-tree the sort key is *(column_value …, rowid)*. There is no
separate rowid field in the cell — the rowid is encoded as the **last column**
of the index record, and the entire record IS the sort key::

    Table leaf cell wire format:
        [total-payload-size  varint]
        [rowid               varint signed]   ← separate key field
        [record bytes                    ]   ← column values only
        [overflow ptr u32, if needed     ]

    Index leaf cell wire format:
        [payload-size  varint]
        [record bytes        ]   ← (col_val, …, rowid)   ← key IS the record
        [overflow ptr u32, if needed]

Because the rowid acts as a tiebreaker (it is always unique), the full sort
key *(column_value, rowid)* is unique even when the indexed column contains
duplicate values. This means the tree never needs to handle equal sort keys.

Interior cell format
--------------------

::

    Index interior cell:
        [left-child page u32 BE ]   ← 4 bytes
        [separator record bytes ]   ← record.encode([*key_vals, rowid])

The separator is the **full composite key** of the last entry in the left
child subtree. The routing rule mirrors the table B-tree::

    if compare_full_key(search_key_vals, search_rowid,
                        sep_key_vals, sep_rowid) <= 0:
        descend into left_child
    else:
        continue to next interior cell (or rightmost_child)

Comparison order
----------------

SQLite's default BINARY collation, applied field by field::

    NULL < INTEGER / REAL < TEXT < BLOB

Integers and floats are compared numerically across types (so ``2.0 == 2``).
Text strings are compared by UTF-8 byte values (case-sensitive).
Blobs compare by raw byte value.

When all key-column values are equal, the rowid (last column) breaks the tie.

Overflow
--------

Long payloads spill to overflow pages identically to table B-trees: the first
``_MAX_LOCAL`` (4 061) bytes stay on the leaf; the remainder fills chained
overflow pages referenced by a 4-byte pointer at the end of the inline portion.

Interior separator records are always assumed to fit inline in v2 (they are
typically small: one typed integer or short text value plus a rowid). Support
for overflowed separator records is deferred to v3.

v1 / v2 limitations
--------------------

* Separator records must fit inline (< 4 061 bytes). Oversized keys
  (e.g. a 5 000-byte BLOB as an index key) are rejected at insert time.
* No rebalancing on delete — sparse leaf pages are not merged.
* Single-process, single-writer (same as the rest of the v2 stack).
* Page size is pinned at 4 096 bytes.
"""

from __future__ import annotations

import struct
from collections.abc import Iterator
from typing import TYPE_CHECKING

from storage_sqlite.errors import CorruptDatabaseError, StorageError
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.record import decode as record_decode
from storage_sqlite.record import encode as record_encode
from storage_sqlite.varint import decode as varint_decode
from storage_sqlite.varint import encode as varint_encode

if TYPE_CHECKING:
    from storage_sqlite.freelist import Freelist

# ── Type alias ────────────────────────────────────────────────────────────────

#: Any value that can appear in an index key.
SqlValue = None | int | float | str | bytes

# ── Page type constants ───────────────────────────────────────────────────────

PAGE_TYPE_LEAF_INDEX: int = 0x0A
"""SQLite page type byte for an index leaf page."""

PAGE_TYPE_INTERIOR_INDEX: int = 0x02
"""SQLite page type byte for an index interior page."""

# ── Page layout constants ─────────────────────────────────────────────────────

_LEAF_HDR: int = 8
"""Bytes consumed by the B-tree page header on a leaf page (shared with table
B-trees — the format is identical; only the page-type byte differs)."""

_INTERIOR_HDR: int = 12
"""Bytes consumed by the B-tree page header on an interior page (8 common bytes
plus the 4-byte rightmost-child pointer)."""

_CELL_PTR: int = 2
"""Bytes per entry in the cell pointer array (u16 big-endian offset)."""

# ── Overflow thresholds (identical to table B-trees) ─────────────────────────

_USABLE: int = PAGE_SIZE  # 4 096 bytes
_MAX_LOCAL: int = _USABLE - 35  # 4 061
_MIN_LOCAL: int = ((_USABLE - 12) * 32) // 255 - 23  # 489
_OVERFLOW_USABLE: int = _USABLE - 4  # payload bytes per overflow page

# ── Traversal safety limit ────────────────────────────────────────────────────

_MAX_BTREE_DEPTH: int = 20
"""Maximum traversal depth before assuming a corrupt / circular tree."""


# ── Errors ────────────────────────────────────────────────────────────────────


class IndexTreeError(StorageError):
    """Base for all index B-tree errors."""


class DuplicateIndexKeyError(IndexTreeError):
    """The same (key_vals, rowid) pair already exists in the index."""


# ── Value comparison (SQLite type ordering) ───────────────────────────────────


def _type_class(v: SqlValue) -> int:
    """Map a Python value to its SQLite comparison class.

    SQLite comparison order (ascending):

    * 0 = NULL          — always the smallest value
    * 1 = INTEGER/REAL  — compared numerically; INT 2 == FLOAT 2.0
    * 2 = TEXT          — UTF-8 byte comparison (BINARY collation)
    * 3 = BLOB          — raw byte comparison

    Examples::

        _type_class(None)  # 0
        _type_class(42)    # 1
        _type_class(3.14)  # 1
        _type_class("hi")  # 2
        _type_class(b"\\x00")  # 3
    """
    if v is None:
        return 0
    if isinstance(v, (int, float)):
        return 1
    if isinstance(v, str):
        return 2
    if isinstance(v, (bytes, bytearray)):
        return 3
    raise TypeError(f"unsupported SQL value type: {type(v)!r}")


def _cmp_values(a: SqlValue, b: SqlValue) -> int:
    """Compare two SQL values, returning −1, 0, or +1.

    Follows SQLite's default collation:

    * Values of different type classes: compare by class number.
    * NULL vs NULL: equal (0).
    * Numeric vs numeric: compare numerically (``2.0 == 2``).
    * Text vs text: compare UTF-8 byte representations (case-sensitive).
    * Blob vs blob: compare raw bytes.

    Truth table (sign of result)::

        a       b       result
        ------  ------  ------
        NULL    NULL     0
        NULL    42      -1
        1       2       -1
        2.0     2        0   ← numeric cross-type equality
        "a"     "b"     -1
        "Z"     "a"     -1  ← capital 'Z' < lower 'a' in UTF-8
        b"a"    "a"     +1  ← BLOB > TEXT
    """
    ta, tb = _type_class(a), _type_class(b)
    if ta != tb:
        return -1 if ta < tb else 1
    if a is None:
        # Both are NULL — equal.
        return 0
    if ta == 1:
        # Both numeric (int or float) — Python's built-in numeric comparison
        # handles cross-type correctly: 2.0 == 2.
        if a == b:
            return 0
        return -1 if a < b else 1  # type: ignore[operator]
    if ta == 2:
        # Both TEXT — compare by UTF-8 byte values (BINARY collation).
        a_enc = a.encode("utf-8")  # type: ignore[union-attr]
        b_enc = b.encode("utf-8")  # type: ignore[union-attr]
        if a_enc == b_enc:
            return 0
        return -1 if a_enc < b_enc else 1
    # Both BLOB — raw byte comparison.
    if a == b:
        return 0
    return -1 if a < b else 1  # type: ignore[operator]


def _cmp_full_keys(
    k1: list[SqlValue], r1: int, k2: list[SqlValue], r2: int
) -> int:
    """Compare two full index entries (key_vals, rowid).

    This is the canonical sort order for index leaf cells.  All key columns
    are compared left-to-right using :func:`_cmp_values`; if all are equal,
    the rowid is the tiebreaker.

    Because rowids are unique within a table, two index entries with the
    same key values *and* the same rowid cannot coexist — this function
    returns 0 only for identical entries.

    Examples::

        _cmp_full_keys([5], 1, [5], 2)   # -1 (same key, r1 < r2)
        _cmp_full_keys([5], 1, [6], 1)   # -1 (5 < 6 in key)
        _cmp_full_keys(["b"], 1, ["a"], 99)  # +1 ("b" > "a")
    """
    for v1, v2 in zip(k1, k2, strict=False):
        c = _cmp_values(v1, v2)
        if c != 0:
            return c
    # All key columns equal — tiebreak by rowid.
    if r1 == r2:
        return 0
    return -1 if r1 < r2 else 1


def _cmp_keys_partial(k1: list[SqlValue], k2: list[SqlValue]) -> int:
    """Compare two key-value lists without rowids.

    Used for range-scan bounds: the bound is specified as a list of key
    column values only (no rowid).  Comparison is element-by-element over
    the shorter list length; if all shared columns are equal the result
    is 0 (the shorter list is not considered "less than" its extension).

    This is appropriate when the caller wants *all entries whose key columns
    equal the bound*, regardless of rowid.
    """
    for v1, v2 in zip(k1, k2, strict=False):
        c = _cmp_values(v1, v2)
        if c != 0:
            return c
    # All shared columns equal.
    if len(k1) == len(k2):
        return 0
    return -1 if len(k1) < len(k2) else 1


# ── Overflow size formula (same as table B-trees) ─────────────────────────────


def _local_payload_size(total: int, page_size: int = PAGE_SIZE) -> int:
    """How many bytes of a *total*-byte payload stay on the leaf page.

    Identical to the table B-tree formula. Values ≤ ``max_local`` are stored
    fully inline; larger payloads spill to overflow pages.  All thresholds
    are derived from *page_size*.
    """
    max_local = page_size - 35
    min_local = ((page_size - 12) * 32) // 255 - 23
    overflow_usable = page_size - 4
    if total <= max_local:
        return total
    local = min_local + (total - min_local) % overflow_usable
    return min_local if local > max_local else local


# ── Page header I/O (index variants) ─────────────────────────────────────────


def _read_idx_hdr(page_data: bytes | bytearray, hdr_off: int) -> dict[str, int]:
    """Parse an index B-tree page header at *hdr_off*.

    Returns a dict with keys: ``page_type``, ``freeblock``, ``ncells``,
    ``content_start``, ``fragmented``, and — for interior pages only —
    ``rightmost_child``.

    The byte layout is identical to table B-tree headers; only the
    page-type byte values differ (``0x0A`` leaf, ``0x02`` interior).
    """
    page_type = page_data[hdr_off]
    freeblock, ncells, content_start, fragmented = struct.unpack_from(
        ">HHHB", page_data, hdr_off + 1
    )
    if content_start == 0:
        content_start = 65536
    result: dict[str, int] = {
        "page_type": page_type,
        "freeblock": freeblock,
        "ncells": ncells,
        "content_start": content_start,
        "fragmented": fragmented,
    }
    if page_type == PAGE_TYPE_INTERIOR_INDEX:
        (rightmost_child,) = struct.unpack_from(">I", page_data, hdr_off + 8)
        result["rightmost_child"] = rightmost_child
    return result


def _write_idx_leaf_hdr(
    buf: bytearray,
    hdr_off: int,
    *,
    ncells: int,
    content_start: int,
    freeblock: int = 0,
    fragmented: int = 0,
) -> None:
    """Write the 8-byte index leaf page header (page type 0x0A) into *buf*."""
    buf[hdr_off] = PAGE_TYPE_LEAF_INDEX
    struct.pack_into(
        ">HHHB",
        buf,
        hdr_off + 1,
        freeblock,
        ncells,
        content_start if content_start != 65536 else 0,
        fragmented,
    )


def _write_idx_interior_hdr(
    buf: bytearray,
    hdr_off: int,
    *,
    ncells: int,
    content_start: int,
    rightmost_child: int,
    freeblock: int = 0,
    fragmented: int = 0,
) -> None:
    """Write the 12-byte index interior page header (page type 0x02) into *buf*."""
    buf[hdr_off] = PAGE_TYPE_INTERIOR_INDEX
    struct.pack_into(
        ">HHHBI",
        buf,
        hdr_off + 1,
        freeblock,
        ncells,
        content_start if content_start != 65536 else 0,
        fragmented,
        rightmost_child,
    )


# ── Cell pointer array helpers ────────────────────────────────────────────────


def _ptr_array_base(hdr_off: int) -> int:
    """Byte offset of the first cell pointer on a leaf index page."""
    return hdr_off + _LEAF_HDR


def _read_leaf_ptrs(
    page_data: bytes | bytearray, hdr_off: int, ncells: int, page_size: int = PAGE_SIZE
) -> list[int]:
    """Return the cell pointer array for a leaf index page.

    Same validity guards as the table B-tree version: cap *ncells* and
    validate each pointer is inside the cell-content area.
    """
    base = _ptr_array_base(hdr_off)
    max_possible: int = (page_size - hdr_off - _LEAF_HDR) // _CELL_PTR
    if ncells > max_possible:
        raise CorruptDatabaseError(
            f"index leaf ncells={ncells} exceeds maximum {max_possible}"
        )
    ptr_array_end = base + ncells * _CELL_PTR
    ptrs: list[int] = []
    for i in range(ncells):
        (ptr,) = struct.unpack_from(">H", page_data, base + i * _CELL_PTR)
        if ptr < ptr_array_end or ptr >= page_size:
            raise CorruptDatabaseError(
                f"index leaf cell pointer[{i}]={ptr} outside valid range "
                f"[{ptr_array_end}, {page_size})"
            )
        ptrs.append(ptr)
    return ptrs


def _read_interior_ptrs(
    page_data: bytes | bytearray, hdr_off: int, ncells: int, page_size: int = PAGE_SIZE
) -> list[int]:
    """Return the cell pointer array for an interior index page."""
    base = hdr_off + _INTERIOR_HDR
    max_possible: int = (page_size - hdr_off - _INTERIOR_HDR) // _CELL_PTR
    if ncells > max_possible:
        raise CorruptDatabaseError(
            f"index interior ncells={ncells} exceeds maximum {max_possible}"
        )
    ptr_array_end = base + ncells * _CELL_PTR
    ptrs: list[int] = []
    for i in range(ncells):
        (ptr,) = struct.unpack_from(">H", page_data, base + i * _CELL_PTR)
        if ptr < ptr_array_end or ptr >= page_size:
            raise CorruptDatabaseError(
                f"index interior cell pointer[{i}]={ptr} outside valid range "
                f"[{ptr_array_end}, {page_size})"
            )
        ptrs.append(ptr)
    return ptrs


def _write_leaf_ptrs(buf: bytearray, hdr_off: int, ptrs: list[int]) -> None:
    """Write the leaf cell pointer array into *buf*."""
    base = _ptr_array_base(hdr_off)
    for i, ptr in enumerate(ptrs):
        struct.pack_into(">H", buf, base + i * _CELL_PTR, ptr)


# ── Leaf cell helpers ─────────────────────────────────────────────────────────


def _idx_leaf_cell_size_on_page(total: int) -> int:
    """Return the bytes a leaf index cell with *total* payload bytes occupies
    on the page (inline portion only — overflow pages are separate).

    Unlike table leaf cells, there is no ``rowid`` varint outside the record.
    The cell is::

        varint_encode(total_size)
        +  local_payload_portion
        +  4 bytes overflow pointer (only when total > _MAX_LOCAL)
    """
    local = _local_payload_size(total)
    return len(varint_encode(total)) + local + (4 if total > local else 0)


def _decode_leaf_cell_key_rowid(
    page_data: bytes | bytearray, ptr: int
) -> tuple[list[SqlValue], int]:
    """Extract (key_vals, rowid) from a leaf index cell without reading overflow.

    This is the fast path used during bisect comparisons.  It reads only
    the inline portion of the record — sufficient to recover the full sort
    key as long as the record fits inline (which is always true for sort-key
    comparisons since the sort key columns must fit on the leaf).

    The payload-size varint tells us the total record length.  We decode
    the *local* portion (which always includes the full type header and the
    rowid value), then reconstruct the complete record.  If the record
    spills to overflow, only the local bytes are used for comparison — this
    works because the key values and rowid are always in the first
    ``_MIN_LOCAL`` bytes or are retrieved in full via :meth:`_read_full_cell`.
    """
    offset = ptr
    _, n = varint_decode(page_data, offset)
    offset += n
    # Decode the local record bytes (may be a partial payload if overflowed).
    values, _ = record_decode(bytes(page_data[offset:]))
    return values[:-1], int(values[-1])  # type: ignore[arg-type]


# ── Interior cell helpers ─────────────────────────────────────────────────────


def _idx_interior_cell_encode(left_child: int, sep_record: bytes) -> bytes:
    """Encode an interior index cell.

    Wire format::

        [left_child  u32 BE]   ← 4 bytes
        [sep_record  bytes ]   ← record.encode([*key_vals, rowid])

    The separator record is the full composite sort key of the last entry in
    the left child subtree, encoded as a standard SQLite record.  It is
    always stored inline (no overflow support for separators in v2).

    The left-child pointer tells the tree traversal which page holds all
    entries with sort key ≤ the separator; entries with sort key > the
    separator live in the next right sibling (or in ``rightmost_child``).
    """
    return struct.pack(">I", left_child) + sep_record


def _idx_interior_cell_decode(
    page_data: bytes | bytearray, ptr: int
) -> tuple[int, list[SqlValue], int]:
    """Decode an interior index cell at *ptr*.

    Returns ``(left_child_pgno, sep_key_vals, sep_rowid)``.

    The separator is a full composite key encoded as a SQLite record.
    ``record_decode`` is called to extract the typed values.
    """
    (left_child,) = struct.unpack_from(">I", page_data, ptr)
    values, _ = record_decode(bytes(page_data[ptr + 4 :]))
    sep_rowid = int(values[-1])  # type: ignore[arg-type]
    sep_key_vals: list[SqlValue] = values[:-1]  # type: ignore[assignment]
    return left_child, sep_key_vals, sep_rowid


def _idx_separator_record(key_vals: list[SqlValue], rowid: int) -> bytes:
    """Encode the separator record stored in interior index cells.

    The separator is the full composite key ``[*key_vals, rowid]`` encoded
    as a standard SQLite record.  Using the full key (including rowid) as
    the separator — rather than just the key columns — gives unambiguous
    routing even when multiple rows share the same indexed column value.
    This matches SQLite's own behaviour for non-unique indexes.
    """
    return record_encode([*key_vals, rowid])


def _idx_interior_cells_fit(
    hdr_off: int, cells: list[tuple[int, bytes]], page_size: int = PAGE_SIZE
) -> bool:
    """Return True if *cells* fit on an interior index page.

    Each interior cell occupies:
    * 2 bytes in the pointer array
    * 4 bytes for the left-child u32
    * ``len(sep_record)`` bytes for the separator record

    The available space is ``page_size - hdr_off - _INTERIOR_HDR``.
    """
    avail = page_size - hdr_off - _INTERIOR_HDR
    needed = sum(_CELL_PTR + 4 + len(sep) for _, sep in cells)
    return needed <= avail


# ── IndexTree ─────────────────────────────────────────────────────────────────


class IndexTree:
    """Index B-tree with full recursive splits.

    Stores ``(key_vals, rowid)`` pairs in ascending order using SQLite index
    page types (0x0A leaf, 0x02 interior).  Supports lookup, ordered range
    scan, insert, delete, and full-tree reclamation via :meth:`free_all`.

    The API mirrors :class:`~storage_sqlite.btree.BTree` (the table B-tree)
    wherever the concepts overlap.

    Parameters
    ----------
    pager:
        The page I/O layer this index lives in.
    root_page:
        1-based page number of the index root page.
    freelist:
        Optional :class:`~storage_sqlite.freelist.Freelist` for page
        allocation and reclamation.  When injected, deleted overflow pages
        are returned to the freelist for reuse; new pages are taken from
        the freelist before extending the file.

    Index cell sort order
    ---------------------

    All entries are sorted by ``(key_vals, rowid)`` using
    :func:`_cmp_full_keys`.  Because rowids are unique within a table, the
    full sort key is always unique — there are no ties at the leaf level.
    """

    __slots__ = ("_freelist", "_pager", "_root_page")

    def __init__(
        self,
        pager: Pager,
        root_page: int,
        *,
        freelist: Freelist | None = None,
    ) -> None:
        self._pager: Pager = pager
        self._root_page: int = root_page
        self._freelist: Freelist | None = freelist

    # ── Construction ──────────────────────────────────────────────────────────

    @classmethod
    def create(
        cls,
        pager: Pager,
        *,
        freelist: Freelist | None = None,
    ) -> IndexTree:
        """Allocate a fresh root page and return an attached IndexTree.

        The root page is initialised as an empty index leaf (type 0x0A).
        Pass the same *pager* and optional *freelist* that the rest of the
        database uses.
        """
        pgno = freelist.allocate() if freelist is not None else pager.allocate()
        ps = pager.page_size
        buf = bytearray(ps)
        _write_idx_leaf_hdr(buf, 0, ncells=0, content_start=ps)
        pager.write(pgno, bytes(buf))
        return cls(pager, pgno, freelist=freelist)

    @classmethod
    def open(
        cls,
        pager: Pager,
        rootpage: int,
        *,
        freelist: Freelist | None = None,
    ) -> IndexTree:
        """Open an existing index B-tree rooted at *rootpage*."""
        return cls(pager, rootpage, freelist=freelist)

    # ── Properties ────────────────────────────────────────────────────────────

    @property
    def root_page(self) -> int:
        """1-based page number of the index root."""
        return self._root_page

    # ── Internal page allocation / freeing ───────────────────────────────────

    def _allocate_page(self) -> int:
        """Return a fresh page number, preferring the freelist."""
        if self._freelist is not None:
            return self._freelist.allocate()
        return self._pager.allocate()

    def _free_page(self, pgno: int) -> None:
        """Release *pgno* to the freelist or zero it in place."""
        if self._freelist is not None:
            self._freelist.free(pgno)
        else:
            self._pager.write(pgno, b"\x00" * self._pager.page_size)

    # ── Public operations ─────────────────────────────────────────────────────

    def insert(self, key: list[SqlValue], rowid: int) -> None:
        """Insert *(key, rowid)* into the index.

        The composite sort key is ``(key, rowid)``.  Splits happen
        transparently at all tree levels.

        Raises :class:`DuplicateIndexKeyError` if the exact same
        ``(key, rowid)`` pair already exists.

        Parameters
        ----------
        key:
            The indexed column value(s) for this row.  For a single-column
            index, pass a one-element list, e.g. ``[42]`` or ``["alice"]``.
        rowid:
            The table rowid that this index entry points to.
        """
        # Build the leaf record: [*key_vals, rowid].
        payload = record_encode([*key, rowid])
        total = len(payload)

        # Validate that the payload fits inline (required for sort correctness
        # — the local bytes on the leaf must include the complete record, or
        # at least the complete type header plus all values up to the rowid).
        # In v2 we enforce that index records always fit inline entirely.
        ps = self._pager.page_size
        max_local = ps - 35
        if total > max_local:
            raise IndexTreeError(
                f"index key payload ({total} bytes) exceeds inline limit "
                f"({max_local} bytes); oversized keys not supported in v2"
            )

        path, leaf_pgno = self._find_leaf_with_path(key, rowid)

        page_data = bytearray(self._pager.read(leaf_pgno))
        hdr = _read_idx_hdr(page_data, 0)
        ncells = hdr["ncells"]
        content_start = hdr["content_start"]
        ptrs = _read_leaf_ptrs(page_data, 0, ncells, ps)

        # Validate content_start.
        ptr_array_end_now = _ptr_array_base(0) + ncells * _CELL_PTR
        if content_start > ps or content_start < ptr_array_end_now:
            raise CorruptDatabaseError(
                f"content_start={content_start} out of valid range on page {leaf_pgno}"
            )


        insert_idx = self._bisect(page_data, ptrs, key, rowid)

        cell_size = _idx_leaf_cell_size_on_page(total)
        ptr_array_end = _ptr_array_base(0) + (ncells + 1) * _CELL_PTR
        new_content_start = content_start - cell_size

        if new_content_start < ptr_array_end:
            # Leaf is full — gather all cells plus the new one and split.
            survivors = [
                _decode_leaf_cell_key_rowid(page_data, ptr) for ptr in ptrs
            ]
            all_cells: list[tuple[list[SqlValue], int]] = (
                survivors[:insert_idx]
                + [(key, rowid)]
                + survivors[insert_idx:]
            )
            if not path:
                # Root is a leaf — root-leaf split.
                self._split_root_leaf(all_cells)
            else:
                sep_record, right_pgno = self._split_leaf(leaf_pgno, all_cells)
                self._push_separator_up(path, leaf_pgno, right_pgno, sep_record)
            return

        # Leaf has room — write the cell.
        cell = varint_encode(total) + payload

        page_data[new_content_start : new_content_start + cell_size] = cell

        # Shift pointer entries right of insert_idx to make room.
        base = _ptr_array_base(0)
        for i in range(ncells, insert_idx, -1):
            src = base + (i - 1) * _CELL_PTR
            dst = base + i * _CELL_PTR
            page_data[dst : dst + _CELL_PTR] = page_data[src : src + _CELL_PTR]
        struct.pack_into(">H", page_data, base + insert_idx * _CELL_PTR, new_content_start)

        _write_idx_leaf_hdr(
            page_data,
            0,
            ncells=ncells + 1,
            content_start=new_content_start,
            freeblock=hdr["freeblock"],
            fragmented=hdr["fragmented"],
        )
        self._pager.write(leaf_pgno, bytes(page_data))

    def delete(self, key: list[SqlValue], rowid: int) -> bool:
        """Remove the entry matching *(key, rowid)* from the index.

        Returns ``True`` if found and removed, ``False`` if not present.
        The containing leaf page is rebuilt in-place after deletion.
        """
        _, leaf_pgno = self._find_leaf_with_path(key, rowid)
        page_data = bytearray(self._pager.read(leaf_pgno))
        hdr = _read_idx_hdr(page_data, 0)
        ptrs = _read_leaf_ptrs(page_data, 0, hdr["ncells"], self._pager.page_size)

        # Binary search for the exact (key, rowid) pair.
        lo, hi = 0, len(ptrs)
        found_idx = -1
        while lo < hi:
            mid = (lo + hi) // 2
            mid_key, mid_rowid = _decode_leaf_cell_key_rowid(page_data, ptrs[mid])
            c = _cmp_full_keys(mid_key, mid_rowid, key, rowid)
            if c < 0:
                lo = mid + 1
            elif c > 0:
                hi = mid
            else:
                found_idx = mid
                break
        if found_idx == -1:
            return False

        # Rebuild the leaf without the found entry.
        survivors = [
            _decode_leaf_cell_key_rowid(page_data, ptr)
            for i, ptr in enumerate(ptrs)
            if i != found_idx
        ]
        self._write_cells_to_leaf(leaf_pgno, survivors)
        return True

    def lookup(self, key: list[SqlValue]) -> list[int]:
        """Return all rowids whose index key equals *key*.

        For a unique index there will be at most one result.  For a
        non-unique index (the default in v2) multiple rowids may match.

        Implemented as a range scan with equal lower and upper bounds.
        """
        return [rowid for _, rowid in self.range_scan(key, key)]

    def range_scan(
        self,
        lo: list[SqlValue] | None,
        hi: list[SqlValue] | None,
        *,
        lo_inclusive: bool = True,
        hi_inclusive: bool = True,
    ) -> Iterator[tuple[list[SqlValue], int]]:
        """Yield *(key_vals, rowid)* pairs in ascending sort order within
        the given key range.

        Parameters
        ----------
        lo:
            Lower bound on the key values (not including the rowid).
            ``None`` means unbounded (start from the first entry).
        hi:
            Upper bound on the key values.  ``None`` means unbounded.
        lo_inclusive:
            When ``True`` (default), entries whose key equals *lo* are
            included.
        hi_inclusive:
            When ``True`` (default), entries whose key equals *hi* are
            included.

        The comparison against *lo* and *hi* uses :func:`_cmp_keys_partial`
        which compares only the key-column values (not the rowid).  This
        means all rows with a given key value are included together, in
        ascending rowid order within that key value.

        Example — scan all entries with user_id between 10 and 20::

            for key_vals, rowid in tree.range_scan([10], [20]):
                ...
        """
        for key_vals, rowid in self._scan_page(self._root_page, 0, set()):
            # Check lower bound.
            if lo is not None:
                c = _cmp_keys_partial(key_vals, lo)
                if c < 0 or (c == 0 and not lo_inclusive):
                    continue  # entry is before the lower bound; skip
            # Check upper bound.  Because the scan is ordered, once we
            # exceed hi we can stop entirely.
            if hi is not None:
                c = _cmp_keys_partial(key_vals, hi)
                if c > 0 or (c == 0 and not hi_inclusive):
                    return  # past the upper bound; done
            yield key_vals, rowid

    def free_all(self, freelist: Freelist) -> None:
        """Reclaim every page in this index tree.

        Traverses the full tree (interior pages, leaf pages, overflow chains)
        and returns each page to *freelist*.  Used by ``drop_index`` to
        release all storage occupied by the dropped index.

        After this call the index's pages must not be accessed again.
        """
        old_freelist = self._freelist
        self._freelist = freelist
        try:
            self._free_subtree(self._root_page, 0, set())
        finally:
            self._freelist = old_freelist

    # ── Path-tracking traversal ───────────────────────────────────────────────

    def _find_leaf_with_path(
        self, key: list[SqlValue], rowid: int
    ) -> tuple[list[tuple[int, int]], int]:
        """Traverse root → leaf, recording the ancestor path.

        Returns ``(path, leaf_pgno)`` where *path* is a list of
        ``(pgno, chosen_idx)`` tuples from root to the leaf's parent.
        ``chosen_idx`` is the index of the interior cell whose
        ``left_child`` was followed (``-1`` if ``rightmost_child`` was
        followed).

        Safety guards: depth limit (detects cycles), page-number validation.
        """
        path: list[tuple[int, int]] = []
        pgno = self._root_page
        depth = 0

        while True:
            if depth > _MAX_BTREE_DEPTH:
                raise CorruptDatabaseError(
                    f"index tree depth exceeded {_MAX_BTREE_DEPTH}: "
                    "corrupt or circular page chain"
                )

            page_data = self._pager.read(pgno)
            hdr = _read_idx_hdr(page_data, 0)

            if hdr["page_type"] == PAGE_TYPE_LEAF_INDEX:
                return path, pgno

            if hdr["page_type"] != PAGE_TYPE_INTERIOR_INDEX:
                raise CorruptDatabaseError(
                    f"index page {pgno} has unexpected type "
                    f"0x{hdr['page_type']:02x} (expected 0x02 or 0x0A)"
                )

            # Walk interior cells to find the child to descend into.
            ptrs = _read_interior_ptrs(page_data, 0, hdr["ncells"], self._pager.page_size)
            chosen_child = hdr["rightmost_child"]
            chosen_idx = -1
            for i, ptr in enumerate(ptrs):
                lc, sep_key, sep_rowid = _idx_interior_cell_decode(page_data, ptr)
                if _cmp_full_keys(key, rowid, sep_key, sep_rowid) <= 0:
                    chosen_child = lc
                    chosen_idx = i
                    break

            if chosen_child == 0 or chosen_child > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"index interior page {pgno} has invalid child pointer "
                    f"{chosen_child} (pager has {self._pager.size_pages} pages)"
                )

            path.append((pgno, chosen_idx))
            pgno = chosen_child
            depth += 1

    # ── Ordered scan ─────────────────────────────────────────────────────────

    def _scan_page(
        self,
        pgno: int,
        hdr_off: int,
        visited: set[int],
    ) -> Iterator[tuple[list[SqlValue], int]]:
        """Recursively yield *(key_vals, rowid)* from the subtree at *pgno*.

        *visited* detects cycles in corrupt databases.  Traversal order:
        for each interior cell left-to-right, recurse into ``left_child``
        first, then after all cells recurse into ``rightmost_child``.  This
        gives entries in ascending sort order.
        """
        if pgno in visited:
            raise CorruptDatabaseError(
                f"cycle detected in index tree: page {pgno} visited twice"
            )
        visited.add(pgno)

        page_data = self._pager.read(pgno)
        hdr = _read_idx_hdr(page_data, hdr_off)

        ps = self._pager.page_size
        if hdr["page_type"] == PAGE_TYPE_LEAF_INDEX:
            ptrs = _read_leaf_ptrs(page_data, hdr_off, hdr["ncells"], ps)
            for ptr in ptrs:
                yield _decode_leaf_cell_key_rowid(page_data, ptr)

        elif hdr["page_type"] == PAGE_TYPE_INTERIOR_INDEX:
            ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], ps)
            for ptr in ptrs:
                left_child, _, _ = _idx_interior_cell_decode(page_data, ptr)
                if left_child == 0 or left_child > self._pager.size_pages:
                    raise CorruptDatabaseError(
                        f"index interior page {pgno} has invalid left child "
                        f"pointer {left_child}"
                    )
                yield from self._scan_page(left_child, 0, visited)
            rightmost = hdr["rightmost_child"]
            if rightmost == 0 or rightmost > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"index interior page {pgno} has invalid rightmost child "
                    f"pointer {rightmost}"
                )
            yield from self._scan_page(rightmost, 0, visited)

        else:
            raise CorruptDatabaseError(
                f"index page {pgno} has unexpected type 0x{hdr['page_type']:02x}"
            )

    # ── Split helpers ─────────────────────────────────────────────────────────

    def _split_root_leaf(
        self, all_cells: list[tuple[list[SqlValue], int]]
    ) -> None:
        """Promote the root leaf to an interior page.

        1. Divide *all_cells* at ``mid = len(all_cells) // 2``.
        2. Write left half to a new leaf, right half to another new leaf.
        3. Separator = full composite key of the last entry in the left half.
        4. Rewrite the root as an interior page with one separator cell.

        The root page number never changes.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        right_cells = all_cells[mid:]
        last_left_key, last_left_rowid = left_cells[-1]
        sep_record = _idx_separator_record(last_left_key, last_left_rowid)

        left_pgno = self._allocate_page()
        right_pgno = self._allocate_page()

        self._write_cells_to_leaf(left_pgno, left_cells)
        self._write_cells_to_leaf(right_pgno, right_cells)

        # Rewrite root as an interior page with one cell.
        ps = self._pager.page_size
        root_buf = bytearray(ps)
        cell = _idx_interior_cell_encode(left_pgno, sep_record)
        cell_off = ps - len(cell)

        _write_idx_interior_hdr(
            root_buf, 0, ncells=1, content_start=cell_off, rightmost_child=right_pgno
        )
        root_buf[cell_off : cell_off + len(cell)] = cell
        struct.pack_into(">H", root_buf, _INTERIOR_HDR, cell_off)
        self._pager.write(self._root_page, bytes(root_buf))

    def _split_leaf(
        self,
        leaf_pgno: int,
        all_cells: list[tuple[list[SqlValue], int]],
    ) -> tuple[bytes, int]:
        """Split a non-root leaf page into two halves.

        *leaf_pgno* is overwritten with the left half; a new right sibling
        is allocated for the right half.

        Returns ``(sep_record_bytes, right_pgno)`` where *sep_record_bytes*
        is ``record.encode([*last_left_key, last_left_rowid])`` — the
        separator to push up into the parent interior page.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        right_cells = all_cells[mid:]
        last_left_key, last_left_rowid = left_cells[-1]
        sep_record = _idx_separator_record(last_left_key, last_left_rowid)

        right_pgno = self._allocate_page()
        self._write_cells_to_leaf(leaf_pgno, left_cells)
        self._write_cells_to_leaf(right_pgno, right_cells)
        return sep_record, right_pgno

    def _write_interior_page(
        self,
        pgno: int,
        cells: list[tuple[int, bytes]],
        rightmost_child: int,
    ) -> None:
        """Write an interior index page with *cells* and *rightmost_child*.

        *cells* is a list of ``(left_child, sep_record_bytes)`` pairs.
        Cells are written downward from ``PAGE_SIZE``; pointer array upward
        from offset ``_INTERIOR_HDR``.
        """
        ps = self._pager.page_size
        buf = bytearray(ps)
        content_offset = ps
        ptrs: list[int] = []
        for left_child, sep_record in cells:
            cell = _idx_interior_cell_encode(left_child, sep_record)
            content_offset -= len(cell)
            buf[content_offset : content_offset + len(cell)] = cell
            ptrs.append(content_offset)

        _write_idx_interior_hdr(
            buf, 0, ncells=len(cells), content_start=content_offset,
            rightmost_child=rightmost_child
        )
        for i, ptr in enumerate(ptrs):
            struct.pack_into(">H", buf, _INTERIOR_HDR + i * _CELL_PTR, ptr)

        self._pager.write(pgno, bytes(buf))

    def _split_interior_page(
        self,
        pgno: int,
        all_cells: list[tuple[int, bytes]],
        rightmost_child: int,
    ) -> tuple[bytes, int]:
        """Split a non-root interior index page.

        The median cell is promoted out of the page.  *pgno* is rewritten
        with the left half; a new right sibling is allocated for the right.

        Split convention::

            left_page  : cells [0, mid)     rightmost_child = median.left_child
            median     : all_cells[mid]  → separator pushed to parent
            right_page : cells [mid+1, n)   rightmost_child = rightmost_child

        Returns ``(median_sep_record, right_pgno)``.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        median_lc, median_sep = all_cells[mid]
        right_cells = all_cells[mid + 1 :]

        right_pgno = self._allocate_page()
        self._write_interior_page(pgno, left_cells, median_lc)
        self._write_interior_page(right_pgno, right_cells, rightmost_child)
        return median_sep, right_pgno

    def _split_root_interior(
        self,
        all_cells: list[tuple[int, bytes]],
        rightmost_child: int,
    ) -> None:
        """Split the root interior page into two interior children.

        1. Pick the median cell.
        2. Allocate left_pgno and right_pgno.
        3. Write left half → left_pgno.
        4. Write right half → right_pgno.
        5. Rewrite root with a single separator cell.

        The root page number never changes.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        median_lc, median_sep = all_cells[mid]
        right_cells = all_cells[mid + 1 :]

        left_pgno = self._allocate_page()
        right_pgno = self._allocate_page()

        self._write_interior_page(left_pgno, left_cells, median_lc)
        self._write_interior_page(right_pgno, right_cells, rightmost_child)

        ps = self._pager.page_size
        root_buf = bytearray(ps)
        cell = _idx_interior_cell_encode(left_pgno, median_sep)
        cell_off = ps - len(cell)
        _write_idx_interior_hdr(
            root_buf, 0, ncells=1, content_start=cell_off, rightmost_child=right_pgno
        )
        root_buf[cell_off : cell_off + len(cell)] = cell
        struct.pack_into(">H", root_buf, _INTERIOR_HDR, cell_off)
        self._pager.write(self._root_page, bytes(root_buf))

    def _push_separator_up(
        self,
        ancestors: list[tuple[int, int]],
        left_pgno: int,
        right_pgno: int,
        sep_record: bytes,
    ) -> None:
        """Insert a new separator into the nearest parent interior page.

        After any leaf or interior split, the new separator must be
        propagated into the parent.  *ancestors* is ordered from root to
        the direct parent (``ancestors[-1]``).

        Two placement cases (matching the table B-tree logic):

        **Case A** — ``chosen_idx ≥ 0``: the parent descended via
        ``old_cells[chosen_idx].left_child = left_pgno``.  Insert
        ``(left_pgno, sep_record)`` at index *chosen_idx*; replace
        ``old_cells[chosen_idx]``'s left_child with *right_pgno*.

        **Case B** — ``chosen_idx = -1``: the parent descended via
        ``rightmost_child = left_pgno``.  Append ``(left_pgno, sep_record)``
        and set ``rightmost_child = right_pgno``.

        If the resulting cell list fits the parent, write it directly.
        Otherwise split the parent and recurse upward.
        """
        parent_pgno, chosen_idx = ancestors[-1]
        remaining = ancestors[:-1]

        parent_data = self._pager.read(parent_pgno)
        parent_hdr = _read_idx_hdr(parent_data, 0)
        ptrs = _read_interior_ptrs(parent_data, 0, parent_hdr["ncells"], self._pager.page_size)
        old_cells: list[tuple[int, bytes]] = []
        for ptr in ptrs:
            lc, sk, sr = _idx_interior_cell_decode(parent_data, ptr)
            old_cells.append((lc, _idx_separator_record(sk, sr)))
        old_rightmost = parent_hdr["rightmost_child"]

        if chosen_idx >= 0:
            # Case A: replace old_cells[chosen_idx] with (left, sep) +
            # (right, old_sep).
            new_cells: list[tuple[int, bytes]] = []
            for i, (lc, sep) in enumerate(old_cells):
                if i == chosen_idx:
                    new_cells.append((left_pgno, sep_record))
                    new_cells.append((right_pgno, sep))
                else:
                    new_cells.append((lc, sep))
            new_rightmost = old_rightmost
        else:
            # Case B: append and update rightmost.
            new_cells = list(old_cells) + [(left_pgno, sep_record)]
            new_rightmost = right_pgno

        if _idx_interior_cells_fit(0, new_cells, self._pager.page_size):
            self._write_interior_page(parent_pgno, new_cells, new_rightmost)
            return

        # Parent is full — split it.
        if not remaining:
            self._split_root_interior(new_cells, new_rightmost)
        else:
            up_sep, new_right_pgno = self._split_interior_page(
                parent_pgno, new_cells, new_rightmost
            )
            self._push_separator_up(remaining, parent_pgno, new_right_pgno, up_sep)

    # ── Cell I/O ──────────────────────────────────────────────────────────────

    def _write_cells_to_leaf(
        self,
        pgno: int,
        cells: list[tuple[list[SqlValue], int]],
    ) -> None:
        """Write a sorted list of (key_vals, rowid) cells to a leaf page.

        Cells are written downward from ``PAGE_SIZE``; pointers upward from
        ``_LEAF_HDR``.  The page is rewritten from scratch (used for splits
        and in-place deletion compaction).

        In v2 index records are always inline (never overflow).  The guard
        below raises :class:`IndexTreeError` if a record somehow exceeds the
        inline limit (which should not happen given the insert-time check in
        :meth:`insert`).
        """
        ps = self._pager.page_size
        buf = bytearray(ps)
        content_offset = ps
        new_ptrs: list[int] = []

        for key_vals, rowid in cells:
            payload = record_encode([*key_vals, rowid])
            total = len(payload)
            cell = varint_encode(total) + payload
            cell_len = len(cell)

            ptr_array_end_check = _ptr_array_base(0) + (len(new_ptrs) + 1) * _CELL_PTR
            content_offset -= cell_len
            if content_offset < ptr_array_end_check:
                raise IndexTreeError(
                    f"internal error: content_offset underran pointer array "
                    f"while writing leaf page {pgno}"
                )
            buf[content_offset : content_offset + cell_len] = cell
            new_ptrs.append(content_offset)

        _write_idx_leaf_hdr(
            buf, 0, ncells=len(cells), content_start=content_offset
        )
        _write_leaf_ptrs(buf, 0, new_ptrs)
        self._pager.write(pgno, bytes(buf))

    def _bisect(
        self,
        page_data: bytes | bytearray,
        ptrs: list[int],
        key: list[SqlValue],
        rowid: int,
    ) -> int:
        """Return the index at which *(key, rowid)* should be inserted.

        Binary-searches *ptrs* by decoding the sort key from each cell.
        Raises :class:`DuplicateIndexKeyError` if the exact *(key, rowid)*
        pair already exists in *ptrs*.
        """
        lo, hi = 0, len(ptrs)
        while lo < hi:
            mid = (lo + hi) // 2
            mid_key, mid_rowid = _decode_leaf_cell_key_rowid(page_data, ptrs[mid])
            c = _cmp_full_keys(mid_key, mid_rowid, key, rowid)
            if c < 0:
                lo = mid + 1
            elif c > 0:
                hi = mid
            else:
                raise DuplicateIndexKeyError(
                    f"index entry (key={key!r}, rowid={rowid}) already exists"
                )
        return lo

    # ── Free-tree reclamation ─────────────────────────────────────────────────

    def _free_subtree(self, pgno: int, hdr_off: int, visited: set[int]) -> None:
        """Recursively free all pages reachable from *pgno*.

        Post-order: children are freed before their parent.  Page 1 is
        never freed (it holds the database header and is not part of any
        B-tree's allocatable space).
        """
        if pgno == 0 or pgno in visited:
            return
        visited.add(pgno)

        page_data = self._pager.read(pgno)
        hdr = _read_idx_hdr(page_data, hdr_off)

        if hdr["page_type"] == PAGE_TYPE_LEAF_INDEX:
            if pgno != 1:
                self._free_page(pgno)

        elif hdr["page_type"] == PAGE_TYPE_INTERIOR_INDEX:
            ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
            for ptr in ptrs:
                left_child, _, _ = _idx_interior_cell_decode(page_data, ptr)
                self._free_subtree(left_child, 0, visited)
            self._free_subtree(hdr["rightmost_child"], 0, visited)
            if pgno != 1:
                self._free_page(pgno)

    # ── Diagnostics ───────────────────────────────────────────────────────────

    def cell_count(self) -> int:
        """Return the total number of index entries.

        Traverses all leaf pages and sums their ``ncells`` fields.
        """
        return self._count_cells(self._root_page, 0)

    def _count_cells(self, pgno: int, hdr_off: int) -> int:
        page_data = self._pager.read(pgno)
        hdr = _read_idx_hdr(page_data, hdr_off)

        if hdr["page_type"] == PAGE_TYPE_LEAF_INDEX:
            return hdr["ncells"]

        if hdr["page_type"] != PAGE_TYPE_INTERIOR_INDEX:
            raise CorruptDatabaseError(
                f"index page {pgno} has unexpected type 0x{hdr['page_type']:02x}"
            )

        ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
        total = 0
        for ptr in ptrs:
            left_child, _, _ = _idx_interior_cell_decode(page_data, ptr)
            total += self._count_cells(left_child, 0)
        total += self._count_cells(hdr["rightmost_child"], 0)
        return total
