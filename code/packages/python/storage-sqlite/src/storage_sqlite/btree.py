"""
Table B-tree pages — phase 4b: full recursive leaf and interior splits.

Architecture summary
--------------------

SQLite organises every table as a **table B-tree**: a balanced tree of
4 096-byte pages where each leaf holds a sorted array of (rowid, record)
pairs and each interior node routes a search to the right child page.

Phase 4b adds full recursive splitting so the tree can grow to arbitrary
depth with any number of rows:

* **Non-root leaf split**: when a leaf that is not the root fills up,
  ``BTree.insert`` splits it into two halves, rewrites the existing page
  with the left half, allocates a right sibling, and pushes the separator
  key up to the parent interior page.
* **Interior page split**: if the parent interior page is also full, it
  is split recursively. The median cell is removed and becomes the new
  separator pushed further up the tree.
* **Root interior split**: if the root itself is an interior page and is
  full, two new interior child pages are allocated for the left and right
  halves and the root is rewritten with a single separator cell, keeping
  the root page number constant.

Phase 4a added (preserved here):

* Reading and traversing **interior pages** (type ``0x05``) so that
  :meth:`BTree.find`, :meth:`BTree.scan`, :meth:`BTree.delete`, and
  :meth:`BTree.cell_count` all work correctly on multi-level trees.
* A **root-leaf split**: when the root page is a leaf and it is full,
  :meth:`BTree.insert` allocates two new leaf pages, divides the cells
  between them, and promotes the root to an interior page containing a
  single separator cell.

Page memory layout
------------------

Every B-tree page has a small header, a cell pointer array that grows
from the header toward the middle, a free gap in the middle, and a cell
content area that grows from the end of the page toward the middle::

    page_offset = 0 (non-page-1) or 100 (page 1 — database header sits first)
    ┌──────────────────────────────────────────────────────┐
    │ B-tree page header (8 bytes leaf / 12 bytes interior) │
    ├──────────────────────────────────────────────────────┤
    │ Cell pointer array  ──────────────► grows up          │
    ├──────────────────────────────────────────────────────┤
    │                 free space                           │
    ├──────────────────────────────────────────────────────┤
    │ Cell content area   ◄─────────────── grows down       │
    └──────────────────────────────────────────────────────┘
    page byte 4095 (last usable byte)

**Leaf page header** (8 bytes, all multi-byte fields big-endian):

::

    offset  size  meaning
       0     1    page type:  0x0D = leaf table
       1     2    first freeblock offset (0 = none)
       3     2    number of cells (u16)
       5     2    cell content area start (0 means 65 536; 4096 for a fresh page)
       7     1    fragmented free bytes

**Interior page header** (12 bytes — same first 8 as leaf, plus):

::

       8     4    rightmost child page number (u32)

**Cell pointer array** starts at ``header_offset + 8`` (leaf) or
``header_offset + 12`` (interior). Each entry is a u16 big-endian offset
pointing to a cell in the content area. For leaf pages, entries are
**sorted ascending by rowid**; for interior pages the same invariant holds
on the separator keys stored in the cells.

Leaf table cell wire format
---------------------------

::

    [total-payload-size  varint]   ← whole record length, incl. overflow
    [rowid               varint]   ← signed (encodes negative rowids)
    [local payload bytes       ]   ← first L bytes of the record
    [overflow page pointer u32 ]   ← only present when total > max_local

Interior table cell wire format
--------------------------------

::

    [left-child page number  u32]  ← big-endian 4-byte page number
    [separator rowid         varint] ← largest rowid stored in left subtree

The right-most child page number lives in the page header (offset 8),
not in any cell. When searching for rowid R, the traversal rule is:

    for each interior cell (left_child, sep_rowid) in sorted order:
        if R <= sep_rowid:  go to left_child
    else:  go to rightmost_child

This ensures every rowid R ≤ last separator goes left; everything larger
goes to the rightmost child.

Overflow pages
--------------

When a record is too large to fit in one page::

    overflow page layout:
        0..3   next overflow page number u32  (0 = last page in chain)
        4..4095 payload continuation bytes

The first ``L`` bytes are inline. The remainder fills overflow pages,
chained by the u32 at their start.

Interior split algorithm
------------------------

Given a full interior page with cells ``[C0, C1, ..., Cn-1]`` and
``rightmost_child = R``::

    mid = n // 2
    left_page  : cells [C0, ..., C(mid-1)], rightmost_child = Cmid.left_child
    median key : Cmid.sep_rowid  (pushed up to parent)
    right_page : cells [C(mid+1), ..., C(n-1)], rightmost_child = R

The median cell is consumed by the split — it disappears from both child
pages and resurfaces as a separator in the parent.

v1 limitations
--------------

* No rebalancing or merging on delete — leaf pages can become sparse.
* No compacting of freeblocks within a page (unused space from deletes is
  not reclaimed until the leaf is rewritten by a subsequent operation).
* Overflow chains are re-encoded on split.  Overflow pages orphaned during
  a split are returned to the freelist if a :class:`~storage_sqlite.freelist.Freelist`
  was injected; otherwise they are zeroed and remain allocated (they will
  be reclaimed by a future phase-6 DROP TABLE or VACUUM).
* Page size is pinned at 4 096; *reserved_per_page* is always 0.
* The ``header_offset`` parameter accommodates page 1 (database header
  at bytes 0–99, B-tree header at byte 100). Callers that want a plain
  B-tree root page pass the default of 0.
"""

from __future__ import annotations

import struct
from collections.abc import Iterator
from typing import TYPE_CHECKING

from storage_sqlite.errors import CorruptDatabaseError, StorageError
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.varint import decode as varint_decode
from storage_sqlite.varint import decode_signed as varint_decode_signed
from storage_sqlite.varint import encode as varint_encode
from storage_sqlite.varint import encode_signed as varint_encode_signed

if TYPE_CHECKING:
    from storage_sqlite.freelist import Freelist

# ── Page type constants ────────────────────────────────────────────────────────

PAGE_TYPE_LEAF_TABLE: int = 0x0D
"""SQLite page type byte for a leaf table B-tree page."""

PAGE_TYPE_INTERIOR_TABLE: int = 0x05
"""SQLite page type byte for an interior table B-tree page."""

# ── Page header sizes ─────────────────────────────────────────────────────────

_LEAF_HDR: int = 8
"""Bytes consumed by the B-tree page header on a leaf page."""

_INTERIOR_HDR: int = 12
"""Bytes consumed by the B-tree page header on an interior page (adds the
4-byte rightmost-child pointer)."""

_CELL_PTR: int = 2
"""Bytes per entry in the cell pointer array."""

# ── Overflow thresholds (for reference — 4 096-byte default) ─────────────────

# SQLite formulas (§2.3.4 of the file-format spec):
#   usableSize  = pageSize - reservedSize  (reservedSize = 0 in v1)
#   maxLocal    = usableSize - 35
#   minLocal    = floor((usableSize - 12) * 32 / 255) - 23
#
# For 4 096-byte pages:
#   maxLocal = 4061
#   minLocal = floor(4084 * 32 / 255) - 23 = 512 - 23 = 489
#
# These module-level constants document the 4 096 case.  The helper
# functions below accept an explicit ``page_size`` parameter so that
# non-default page sizes work correctly.

_USABLE: int = PAGE_SIZE  # 4 096
_MAX_LOCAL: int = _USABLE - 35  # 4 061
_MIN_LOCAL: int = ((_USABLE - 12) * 32) // 255 - 23  # 489
_OVERFLOW_USABLE: int = _USABLE - 4  # bytes per overflow page (minus 4-byte next ptr)

# ── Traversal safety limit ────────────────────────────────────────────────────

_MAX_BTREE_DEPTH: int = 20
"""Maximum B-tree depth before we assume corruption (infinite loop guard)."""


# ── Errors ────────────────────────────────────────────────────────────────────


class BTreeError(StorageError):
    """Base for all B-tree layer errors."""


class PageFullError(BTreeError):
    """Retained in the public API for external callers.

    After phase 4b, normal :meth:`BTree.insert` calls never raise this
    exception — recursive splits handle all leaf-overflow cases. The
    class is kept so that custom callers who build their own backends can
    still signal page-full conditions if needed.
    """


class DuplicateRowidError(BTreeError):
    """A cell with the same rowid already exists in the B-tree."""


# ── Internal helpers ──────────────────────────────────────────────────────────


def _local_payload_size(total: int, page_size: int = PAGE_SIZE) -> int:
    """Compute how many bytes of ``total`` stay on the leaf page.

    Returns ``total`` unchanged when no overflow is needed. When overflow
    is needed, returns the local portion per the SQLite formula — always
    ≤ ``max_local`` and always ≥ ``min_local`` (both derived from
    *page_size*).

    The formula is designed so that adjacent overflow pages are used
    efficiently: the local portion is the largest value of the form
    ``minLocal + k * (pageSize - 4)`` that is ≤ maxLocal, given the
    total payload. In practice this pushes as much as possible inline
    while keeping overflow pages close to full.
    """
    max_local = page_size - 35
    min_local = ((page_size - 12) * 32) // 255 - 23
    overflow_usable = page_size - 4
    if total <= max_local:
        return total
    local = min_local + (total - min_local) % overflow_usable
    return min_local if local > max_local else local


def _cell_rowid_only(page_data: bytes, ptr: int) -> int:
    """Decode only the rowid from a leaf table cell at byte offset *ptr*.

    Skips the payload-size varint, then reads the rowid varint. Used for
    binary-search comparisons where we don't need the payload.
    """
    offset = ptr
    _, n = varint_decode(page_data, offset)  # skip total-payload size
    offset += n
    rowid, _ = varint_decode_signed(page_data, offset)
    return rowid


def _cell_size_on_page(rowid: int, total_payload: int, page_size: int = PAGE_SIZE) -> int:
    """Return the number of bytes a cell occupies on the leaf page.

    This is the varint for total-payload, plus the varint for rowid, plus
    the local payload portion, plus 4 if there is an overflow pointer.
    """
    local = _local_payload_size(total_payload, page_size)
    overflow = total_payload > local
    return (
        len(varint_encode(total_payload))
        + len(varint_encode_signed(rowid))
        + local
        + (4 if overflow else 0)
    )


def _read_hdr(page_data: bytes | bytearray, hdr_off: int) -> dict[str, int]:
    """Parse the B-tree page header at *hdr_off*.

    Reads the common 8-byte prefix (leaf or interior). For interior pages
    (type ``0x05``) also reads the 4-byte rightmost-child pointer that
    follows immediately, returning it as ``"rightmost_child"`` in the dict.
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
    if page_type == PAGE_TYPE_INTERIOR_TABLE:
        (rightmost_child,) = struct.unpack_from(">I", page_data, hdr_off + 8)
        result["rightmost_child"] = rightmost_child
    return result


def _write_hdr(
    buf: bytearray,
    hdr_off: int,
    *,
    ncells: int,
    content_start: int,
    freeblock: int = 0,
    fragmented: int = 0,
) -> None:
    """Write the 8-byte leaf page header into *buf* at *hdr_off*."""
    buf[hdr_off] = PAGE_TYPE_LEAF_TABLE
    struct.pack_into(
        ">HHHB",
        buf,
        hdr_off + 1,
        freeblock,
        ncells,
        content_start if content_start != 65536 else 0,
        fragmented,
    )


def _write_interior_hdr(
    buf: bytearray,
    hdr_off: int,
    *,
    ncells: int,
    content_start: int,
    rightmost_child: int,
    freeblock: int = 0,
    fragmented: int = 0,
) -> None:
    """Write the 12-byte interior page header into *buf* at *hdr_off*.

    Layout (all big-endian)::

        hdr_off + 0  : u8  page type (0x05)
        hdr_off + 1  : u16 freeblock
        hdr_off + 3  : u16 ncells
        hdr_off + 5  : u16 content_start
        hdr_off + 7  : u8  fragmented
        hdr_off + 8  : u32 rightmost_child

    The struct format ``">HHHBI"`` is 2+2+2+1+4 = 11 bytes placed at
    offset hdr_off+1, giving a total 12-byte header.
    """
    buf[hdr_off] = PAGE_TYPE_INTERIOR_TABLE
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


def _ptr_array_base(hdr_off: int) -> int:
    """Return the byte offset of the first cell pointer entry on a *leaf* page.

    For interior pages use ``hdr_off + _INTERIOR_HDR`` directly (the extra
    4-byte rightmost-child field shifts the array start by 4 bytes).
    """
    return hdr_off + _LEAF_HDR


def _read_ptrs(
    page_data: bytes | bytearray, hdr_off: int, ncells: int, page_size: int = PAGE_SIZE
) -> list[int]:
    """Return the cell pointer array as a list of absolute page offsets (leaf).

    Guards against corrupt page data in two ways:

    1. **ncells cap** — a corrupt ``ncells`` of 65 535 would cause 131 070
       bytes of ``struct.unpack_from`` reads on a 4 096-byte page, reading
       far past the buffer and raising an obscure error deep inside
       ``struct``.  We reject any ``ncells`` that cannot physically fit.

    2. **pointer range** — each u16 pointer must land *inside* the cell
       content area, i.e. strictly after the pointer array itself and
       strictly before *page_size*.  A pointer aimed at the page header,
       the pointer array, or beyond the page boundary would silently
       mis-decode cell data or produce out-of-bounds reads.

    Raises :class:`~storage_sqlite.errors.CorruptDatabaseError` if either
    check fails.
    """
    base = _ptr_array_base(hdr_off)

    # Maximum number of 2-byte pointers that can possibly fit on the page.
    max_possible: int = (page_size - hdr_off - _LEAF_HDR) // _CELL_PTR
    if ncells > max_possible:
        raise CorruptDatabaseError(
            f"ncells={ncells} exceeds maximum possible {max_possible} "
            f"for page with header_offset={hdr_off}"
        )

    # The pointer array occupies bytes [base, base + ncells * _CELL_PTR).
    # Every pointer must point into the cell content area that comes after.
    ptr_array_end: int = base + ncells * _CELL_PTR

    ptrs: list[int] = []
    for i in range(ncells):
        (ptr,) = struct.unpack_from(">H", page_data, base + i * _CELL_PTR)
        if ptr < ptr_array_end or ptr >= page_size:
            raise CorruptDatabaseError(
                f"cell pointer[{i}]={ptr} is outside valid cell-content range "
                f"[{ptr_array_end}, {page_size})"
            )
        ptrs.append(ptr)
    return ptrs


def _read_interior_ptrs(
    page_data: bytes | bytearray, hdr_off: int, ncells: int, page_size: int = PAGE_SIZE
) -> list[int]:
    """Return the cell pointer array for an *interior* page.

    Interior pages have a 12-byte header (vs 8 on leaf pages), so the
    pointer array base is ``hdr_off + _INTERIOR_HDR``. The same validity
    checks as :func:`_read_ptrs` apply.
    """
    base = hdr_off + _INTERIOR_HDR

    max_possible: int = (page_size - hdr_off - _INTERIOR_HDR) // _CELL_PTR
    if ncells > max_possible:
        raise CorruptDatabaseError(
            f"interior page ncells={ncells} exceeds maximum possible "
            f"{max_possible} for header_offset={hdr_off}"
        )

    ptr_array_end: int = base + ncells * _CELL_PTR
    ptrs: list[int] = []
    for i in range(ncells):
        (ptr,) = struct.unpack_from(">H", page_data, base + i * _CELL_PTR)
        if ptr < ptr_array_end or ptr >= page_size:
            raise CorruptDatabaseError(
                f"interior cell pointer[{i}]={ptr} is outside valid range "
                f"[{ptr_array_end}, {page_size})"
            )
        ptrs.append(ptr)
    return ptrs


def _read_interior_cell(page_data: bytes | bytearray, ptr: int) -> tuple[int, int]:
    """Decode an interior table cell at byte offset *ptr*.

    Returns ``(left_child_pgno, separator_rowid)``.

    Wire format::

        [left_child  u32 BE]  ← 4 bytes
        [sep_rowid   varint]  ← 1-9 bytes (signed)
    """
    (left_child,) = struct.unpack_from(">I", page_data, ptr)
    sep_rowid, _ = varint_decode_signed(page_data, ptr + 4)
    return left_child, sep_rowid


def _interior_cell_encode(left_child: int, sep_rowid: int) -> bytes:
    """Encode an interior table cell.

    Returns the raw bytes: ``[left_child u32 BE][sep_rowid varint]``.
    """
    return struct.pack(">I", left_child) + varint_encode_signed(sep_rowid)


def _interior_cells_fit(
    hdr_off: int, cells: list[tuple[int, int]], page_size: int = PAGE_SIZE
) -> bool:
    """Return ``True`` if *cells* fit on an interior page with header at *hdr_off*.

    The usable space on an interior page is
    ``page_size - hdr_off - _INTERIOR_HDR`` bytes.  Each cell consumes
    2 bytes in the pointer array plus its raw cell bytes.

    This is called by :meth:`BTree._push_separator_up` to decide whether
    adding one new separator cell to a parent interior page requires
    splitting that page.

    Example — check whether 510 cells fit on a fresh interior root::

        cells_510 = [(i, i * 100) for i in range(1, 511)]
        assert _interior_cells_fit(0, cells_510)   # probably True
        cells_511 = cells_510 + [(511, 51100)]
        assert not _interior_cells_fit(0, cells_511)  # at capacity, False
    """
    avail = page_size - hdr_off - _INTERIOR_HDR
    needed = sum(
        _CELL_PTR + len(_interior_cell_encode(lc, sep))
        for lc, sep in cells
    )
    return needed <= avail


def _write_ptrs(buf: bytearray, hdr_off: int, ptrs: list[int]) -> None:
    """Write the cell pointer array from *ptrs* into *buf* (leaf pages)."""
    base = _ptr_array_base(hdr_off)
    for i, ptr in enumerate(ptrs):
        struct.pack_into(">H", buf, base + i * _CELL_PTR, ptr)


def _init_leaf_page(buf: bytearray, hdr_off: int = 0, page_size: int = PAGE_SIZE) -> None:
    """Write a fresh empty leaf table page header into *buf*."""
    _write_hdr(buf, hdr_off, ncells=0, content_start=page_size)


# ── BTree ─────────────────────────────────────────────────────────────────────


class BTree:
    """Table B-tree with full recursive splits (phase 4b).

    Supports an arbitrary number of rows at any depth. Inserts that fill a
    leaf page trigger an automatic split:

    * **Root-leaf split** (depth 1 → 2): the root leaf is promoted to an
      interior page and two fresh leaves receive the divided cells.
    * **Non-root leaf split** (depth ≥ 2): the leaf is split in two, and
      the separator key is pushed up to the parent interior page.
    * **Interior page split** (recursive): if the parent interior page is
      also full, it is split too, propagating a new separator further up.
    * **Root interior split**: if the root interior page is full, two new
      interior children are allocated and the root is rewritten with one
      cell.

    In all cases the :attr:`root_page` number is constant.

    All reads and writes go through the injected :class:`~storage_sqlite.pager.Pager`.
    The B-tree itself is stateless between calls.

    Parameters
    ----------
    pager:
        The page I/O layer this tree lives in.
    root_page:
        1-based page number of the B-tree root.
    header_offset:
        Byte offset within the root page where the B-tree page header
        starts. 0 for ordinary pages; 100 for page 1 (which has the
        100-byte SQLite database header in front of it). Note: only the
        root page is affected; all other pages in the tree always have
        their B-tree header at offset 0.
    freelist:
        Optional :class:`~storage_sqlite.freelist.Freelist` to use for
        page allocation and reclamation.  When provided, overflow pages
        freed by :meth:`delete` (and :meth:`update`) are returned to the
        freelist for reuse, and new pages are taken from the freelist
        before extending the file.  When ``None`` (default), freed overflow
        pages are zeroed in place and new pages always extend the file —
        the same behaviour as phases 1–4.
    """

    __slots__ = ("_freelist", "_header_offset", "_pager", "_root_page")

    def __init__(
        self,
        pager: Pager,
        root_page: int,
        *,
        header_offset: int = 0,
        freelist: Freelist | None = None,
    ) -> None:
        self._pager: Pager = pager
        self._root_page: int = root_page
        self._header_offset: int = header_offset
        self._freelist: Freelist | None = freelist

    # ── Construction ──────────────────────────────────────────────────────────

    @classmethod
    def create(
        cls,
        pager: Pager,
        *,
        header_offset: int = 0,
        freelist: Freelist | None = None,
    ) -> BTree:
        """Allocate a fresh leaf page and initialise it as an empty B-tree.

        Returns a :class:`BTree` rooted at the newly allocated page. The
        caller must :meth:`~storage_sqlite.pager.Pager.commit` the pager
        when ready to persist.

        If *freelist* is supplied, the root page is taken from the freelist
        rather than extending the file.
        """
        pgno = freelist.allocate() if freelist is not None else pager.allocate()
        buf = bytearray(pager.read(pgno))
        _init_leaf_page(buf, header_offset, pager.page_size)
        pager.write(pgno, bytes(buf))
        return cls(pager, pgno, header_offset=header_offset, freelist=freelist)

    @classmethod
    def open(
        cls,
        pager: Pager,
        root_page: int,
        *,
        header_offset: int = 0,
        freelist: Freelist | None = None,
    ) -> BTree:
        """Open an existing B-tree rooted at *root_page*.

        Pass the same *freelist* that was used when the B-tree was created
        so that subsequent inserts and deletes reuse the same freelist.
        """
        return cls(pager, root_page, header_offset=header_offset, freelist=freelist)

    # ── Properties ────────────────────────────────────────────────────────────

    @property
    def root_page(self) -> int:
        """1-based page number of the B-tree root."""
        return self._root_page

    # ── Internal page allocation / freeing helpers ────────────────────────────

    def _allocate_page(self) -> int:
        """Return a fresh page number for this B-tree.

        Tries the freelist first.  Falls back to :meth:`Pager.allocate` if
        no freelist was injected or the freelist is empty.
        """
        if self._freelist is not None:
            return self._freelist.allocate()
        return self._pager.allocate()

    def _free_page(self, pgno: int) -> None:
        """Release *pgno* back to the freelist if available.

        If no freelist was injected, the page is zeroed in the pager's
        dirty-page table so it does not contain stale data.  The page
        remains allocated in that case (no reuse until phase 6 / VACUUM).
        """
        if self._freelist is not None:
            self._freelist.free(pgno)
        else:
            self._pager.write(pgno, b"\x00" * self._pager.page_size)

    # ── Public operations ─────────────────────────────────────────────────────

    def insert(self, rowid: int, payload: bytes) -> None:
        """Insert *(rowid, payload)* into the B-tree.

        *payload* is the raw record bytes produced by
        :func:`~storage_sqlite.record.encode`. The B-tree stores and
        returns it opaquely.

        If the total payload exceeds the per-page inline threshold
        (:data:`_MAX_LOCAL` = 4 061 bytes), the tail is spilled to one or
        more overflow pages allocated from the pager, and only the local
        portion stays on the leaf.

        When a leaf page is full, it is split automatically:

        * Root leaf → root-leaf split (root promoted to interior, two new
          leaf children). The root page number never changes.
        * Non-root leaf → leaf split, separator pushed up.  If the parent
          interior page is also full, the split propagates recursively up
          the ancestor path, potentially splitting the root interior page.

        Raises
        ------
        DuplicateRowidError
            A cell with the same *rowid* already exists.
        """
        # Find the leaf page where this rowid belongs, recording the path
        # of interior ancestors from root to the leaf's parent.
        path, leaf_pgno, leaf_hdr_off = self._find_leaf_with_path(rowid)

        ps = self._pager.page_size
        page_data = bytearray(self._pager.read(leaf_pgno))
        hdr = _read_hdr(page_data, leaf_hdr_off)
        ncells = hdr["ncells"]
        content_start = hdr["content_start"]
        ptrs = _read_ptrs(page_data, leaf_hdr_off, ncells, ps)

        # Validate content_start before using it as a write cursor.  A
        # corrupt header could give a value that overlaps the pointer array
        # (too low) or points beyond the page (too high).  Either would
        # silently produce a corrupt page.
        ptr_array_end_now: int = _ptr_array_base(leaf_hdr_off) + ncells * _CELL_PTR
        if content_start > ps or content_start < ptr_array_end_now:
            raise CorruptDatabaseError(
                f"content_start={content_start} is out of valid range "
                f"[{ptr_array_end_now}, {ps}] on page {leaf_pgno}"
            )

        # Duplicate check + sorted insert position (raises DuplicateRowidError).
        insert_idx = self._bisect(page_data, ptrs, rowid)

        # Calculate how much on-page space the new cell needs *without*
        # writing the overflow chain yet, so we can test free space first.
        total = len(payload)
        cell_size = _cell_size_on_page(rowid, total, ps)

        ptr_array_end: int = _ptr_array_base(leaf_hdr_off) + (ncells + 1) * _CELL_PTR
        new_content_start = content_start - cell_size

        if new_content_start < ptr_array_end:
            # The leaf is full.  Gather all existing cells plus the new one
            # in sorted order and dispatch to the appropriate split path.
            survivors = [self._read_cell(page_data, ptr) for ptr in ptrs]
            all_cells = (
                survivors[:insert_idx]
                + [(rowid, payload)]
                + survivors[insert_idx:]
            )
            if not path:
                # Leaf IS the root → root-leaf split (phase 4a).
                self._split_root_leaf(all_cells)
            else:
                # Non-root leaf → split, then push separator up.
                sep, right_pgno = self._split_leaf(leaf_pgno, all_cells)
                self._push_separator_up(path, leaf_pgno, right_pgno, sep)
            return

        # Leaf has room: now write the overflow chain (if needed).
        local = _local_payload_size(total, ps)
        overflow_pgno = 0
        if total > local:
            overflow_pgno = self._write_overflow(payload, local)

        # Assemble the cell bytes.
        cell = (
            varint_encode(total)
            + varint_encode_signed(rowid)
            + payload[:local]
            + (struct.pack(">I", overflow_pgno) if total > local else b"")
        )

        # Write cell bytes at the bottom of the cell content area.
        page_data[new_content_start : new_content_start + cell_size] = cell

        # Shift pointer entries right of insert_idx to make room for the new one.
        base = _ptr_array_base(leaf_hdr_off)
        for i in range(ncells, insert_idx, -1):
            src = base + (i - 1) * _CELL_PTR
            dst = base + i * _CELL_PTR
            page_data[dst : dst + _CELL_PTR] = page_data[src : src + _CELL_PTR]

        # Write the new pointer.
        struct.pack_into(">H", page_data, base + insert_idx * _CELL_PTR, new_content_start)

        # Update the page header.
        _write_hdr(
            page_data,
            leaf_hdr_off,
            ncells=ncells + 1,
            content_start=new_content_start,
            freeblock=hdr["freeblock"],
            fragmented=hdr["fragmented"],
        )
        self._pager.write(leaf_pgno, bytes(page_data))

    def find(self, rowid: int) -> bytes | None:
        """Return the payload for *rowid*, or ``None`` if not found.

        Traverses interior pages as needed; the search always terminates
        at the unique leaf page that could contain *rowid*.
        """
        leaf_pgno, leaf_hdr_off = self._find_leaf_page(rowid)
        page_data = self._pager.read(leaf_pgno)
        hdr = _read_hdr(page_data, leaf_hdr_off)
        ptrs = _read_ptrs(page_data, leaf_hdr_off, hdr["ncells"], self._pager.page_size)

        lo, hi = 0, len(ptrs)
        while lo < hi:
            mid = (lo + hi) // 2
            mid_rowid = _cell_rowid_only(page_data, ptrs[mid])
            if mid_rowid < rowid:
                lo = mid + 1
            elif mid_rowid > rowid:
                hi = mid
            else:
                _, payload = self._read_cell(page_data, ptrs[mid])
                return payload
        return None

    def scan(self) -> Iterator[tuple[int, bytes]]:
        """Yield *(rowid, payload)* pairs in ascending rowid order.

        Performs a full left-to-right depth-first traversal of the B-tree,
        visiting every leaf in ascending key order. Follows overflow chains
        transparently.
        """
        yield from self._scan_page(self._root_page, self._header_offset, set())

    def delete(self, rowid: int) -> bool:
        """Remove the cell for *rowid*.

        Returns ``True`` if the cell was found and removed, ``False`` if
        *rowid* does not exist. Overflow pages for the deleted cell are
        freed (zeroed). The containing leaf page is compacted in-place
        after deletion.

        Works correctly on multi-level trees: traverses interior pages to
        locate the right leaf, modifies only that leaf.
        """
        leaf_pgno, leaf_hdr_off = self._find_leaf_page(rowid)
        page_data = bytearray(self._pager.read(leaf_pgno))
        hdr = _read_hdr(page_data, leaf_hdr_off)
        ptrs = _read_ptrs(page_data, leaf_hdr_off, hdr["ncells"], self._pager.page_size)

        # Binary search for the target rowid.
        lo, hi = 0, len(ptrs)
        found_idx = -1
        while lo < hi:
            mid = (lo + hi) // 2
            mid_rowid = _cell_rowid_only(page_data, ptrs[mid])
            if mid_rowid < rowid:
                lo = mid + 1
            elif mid_rowid > rowid:
                hi = mid
            else:
                found_idx = mid
                break
        if found_idx == -1:
            return False

        # Collect all surviving (rowid, payload) pairs.
        survivors: list[tuple[int, bytes]] = []
        for i, ptr in enumerate(ptrs):
            if i == found_idx:
                self._free_overflow(page_data, ptr)
            else:
                survivors.append(self._read_cell(page_data, ptr))

        # Rebuild the leaf page from scratch with surviving cells.
        self._write_cells_to_leaf(leaf_pgno, leaf_hdr_off, survivors)
        return True

    def update(self, rowid: int, payload: bytes) -> bool:
        """Replace the payload for *rowid*.

        Returns ``True`` if found and updated, ``False`` if not found.
        Implemented as a delete + insert, which handles the case where the
        new payload changes overflow requirements.
        """
        if not self.delete(rowid):
            return False
        self.insert(rowid, payload)
        return True

    # ── Path-tracking traversal ───────────────────────────────────────────────

    def _find_leaf_with_path(
        self, rowid: int
    ) -> tuple[list[tuple[int, int, int]], int, int]:
        """Traverse root → leaf, recording the ancestor path for split propagation.

        Returns ``(path, leaf_pgno, leaf_hdr_off)`` where *path* is a list
        of ``(pgno, hdr_off, chosen_idx)`` tuples for each interior page
        visited, ordered from root to the leaf's immediate parent.

        ``chosen_idx`` is the index of the interior cell whose
        ``left_child`` pointer was followed to reach the next level, or
        ``-1`` when the ``rightmost_child`` was followed.  This bookkeeping
        is needed by :meth:`_push_separator_up` to locate where the split
        child lives in the parent's cell array.

        Safety guards are identical to :meth:`_find_leaf_page`.
        """
        path: list[tuple[int, int, int]] = []
        pgno = self._root_page
        hdr_off = self._header_offset
        depth = 0

        while True:
            if depth > _MAX_BTREE_DEPTH:
                raise CorruptDatabaseError(
                    f"B-tree depth exceeded {_MAX_BTREE_DEPTH}: "
                    f"corrupt or circular interior page chain"
                )

            page_data = self._pager.read(pgno)
            hdr = _read_hdr(page_data, hdr_off)

            if hdr["page_type"] == PAGE_TYPE_LEAF_TABLE:
                return path, pgno, hdr_off

            if hdr["page_type"] != PAGE_TYPE_INTERIOR_TABLE:
                raise CorruptDatabaseError(
                    f"page {pgno} has unexpected type "
                    f"0x{hdr['page_type']:02x} (expected 0x05 or 0x0D)"
                )

            # Walk the interior cells to find which child to descend into,
            # recording the index so _push_separator_up can find it later.
            ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
            chosen_child = hdr["rightmost_child"]
            chosen_idx = -1  # -1 means rightmost_child was chosen
            for i, ptr in enumerate(ptrs):
                left_child, sep_rowid = _read_interior_cell(page_data, ptr)
                if rowid <= sep_rowid:
                    chosen_child = left_child
                    chosen_idx = i
                    break

            if chosen_child == 0 or chosen_child > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"interior page {pgno} has invalid child pointer "
                    f"{chosen_child} (pager has {self._pager.size_pages} pages)"
                )

            path.append((pgno, hdr_off, chosen_idx))
            pgno = chosen_child
            hdr_off = 0  # only the root page may have a non-zero header offset
            depth += 1

    def _find_leaf_page(self, rowid: int) -> tuple[int, int]:
        """Return ``(leaf_pgno, hdr_off)`` for the leaf that would contain *rowid*.

        Thin wrapper around :meth:`_find_leaf_with_path` used by
        :meth:`find`, :meth:`delete`, and :meth:`update`, which only need
        the leaf location and not the full ancestor path.
        """
        _, leaf_pgno, leaf_hdr_off = self._find_leaf_with_path(rowid)
        return leaf_pgno, leaf_hdr_off

    # ── Scan ──────────────────────────────────────────────────────────────────

    def _scan_page(
        self,
        pgno: int,
        hdr_off: int,
        visited: set[int],
    ) -> Iterator[tuple[int, bytes]]:
        """Recursively yield *(rowid, payload)* from the subtree at *pgno*.

        *visited* tracks every page touched so far; a page appearing twice
        indicates a cycle in the tree structure, which raises
        :class:`~storage_sqlite.errors.CorruptDatabaseError`.

        Traversal order: for each interior cell left-to-right, recurse into
        the *left_child* first (which holds all smaller rowids), then after
        all cells recurse into *rightmost_child*. This gives ascending order.
        """
        if pgno in visited:
            raise CorruptDatabaseError(
                f"cycle detected in B-tree structure: page {pgno} visited twice"
            )
        visited.add(pgno)

        page_data = self._pager.read(pgno)
        hdr = _read_hdr(page_data, hdr_off)

        ps = self._pager.page_size
        if hdr["page_type"] == PAGE_TYPE_LEAF_TABLE:
            for ptr in _read_ptrs(page_data, hdr_off, hdr["ncells"], ps):
                yield self._read_cell(page_data, ptr)

        elif hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE:
            ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], ps)
            for ptr in ptrs:
                left_child, _ = _read_interior_cell(page_data, ptr)
                if left_child == 0 or left_child > self._pager.size_pages:
                    raise CorruptDatabaseError(
                        f"interior page {pgno} has invalid left child pointer "
                        f"{left_child} (pager has {self._pager.size_pages} pages)"
                    )
                yield from self._scan_page(left_child, 0, visited)
            rightmost = hdr["rightmost_child"]
            if rightmost == 0 or rightmost > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"interior page {pgno} has invalid rightmost child pointer "
                    f"{rightmost} (pager has {self._pager.size_pages} pages)"
                )
            yield from self._scan_page(rightmost, 0, visited)

        else:
            raise CorruptDatabaseError(
                f"page {pgno} has unexpected type 0x{hdr['page_type']:02x}"
            )

    def _count_cells(self, pgno: int, hdr_off: int) -> int:
        """Return total cell count in the subtree rooted at *pgno*.

        For leaf pages this is simply ``ncells`` from the header. For
        interior pages this is the sum of all children's cell counts.
        """
        page_data = self._pager.read(pgno)
        hdr = _read_hdr(page_data, hdr_off)

        if hdr["page_type"] == PAGE_TYPE_LEAF_TABLE:
            return hdr["ncells"]

        if hdr["page_type"] != PAGE_TYPE_INTERIOR_TABLE:
            raise CorruptDatabaseError(
                f"page {pgno} has unexpected type 0x{hdr['page_type']:02x}"
            )

        ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
        total = 0
        for ptr in ptrs:
            left_child, _ = _read_interior_cell(page_data, ptr)
            total += self._count_cells(left_child, 0)
        total += self._count_cells(hdr["rightmost_child"], 0)
        return total

    # ── Split helpers ─────────────────────────────────────────────────────────

    def _split_root_leaf(self, all_cells: list[tuple[int, bytes]]) -> None:
        """Promote the root-leaf to an interior page by splitting *all_cells*.

        *all_cells* is the complete sorted list of ``(rowid, payload)``
        pairs that the new tree must contain — the caller merges the new
        cell into the existing ones before calling here.

        Steps:

        1. Divide *all_cells* at ``mid = len(all_cells) // 2``.
           Left half  → new ``left_pgno``  (fresh leaf page).
           Right half → new ``right_pgno`` (fresh leaf page).
        2. The separator key is the *maximum rowid in the left half*
           (``all_cells[mid-1][0]``).  This matches SQLite's convention:
           the interior cell key is the largest key in the left subtree.
        3. Rewrite the root page as an interior page with one cell:
           ``[left_pgno u32][separator varint]`` and
           ``rightmost_child = right_pgno``.

        The root page number never changes, so existing callers that hold
        :attr:`root_page` see a seamless promotion.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        right_cells = all_cells[mid:]
        separator_rowid = left_cells[-1][0]  # max rowid in left subtree

        # Allocate two fresh leaf pages.
        left_pgno = self._allocate_page()
        right_pgno = self._allocate_page()

        # Write cell halves to the new leaf pages (hdr_off=0 for non-root).
        self._write_cells_to_leaf(left_pgno, 0, left_cells)
        self._write_cells_to_leaf(right_pgno, 0, right_cells)

        # Rewrite the root as an interior page.
        ps = self._pager.page_size
        hdr_off = self._header_offset
        root_buf = bytearray(ps)

        # Preserve any bytes before hdr_off (e.g. the 100-byte SQLite
        # database header on page 1; those bytes must not be zeroed out).
        if hdr_off > 0:
            orig = self._pager.read(self._root_page)
            root_buf[:hdr_off] = orig[:hdr_off]

        # Single interior cell: [left_pgno u32][separator_rowid varint].
        cell = _interior_cell_encode(left_pgno, separator_rowid)
        cell_off = ps - len(cell)

        _write_interior_hdr(
            root_buf,
            hdr_off,
            ncells=1,
            content_start=cell_off,
            rightmost_child=right_pgno,
        )
        root_buf[cell_off : cell_off + len(cell)] = cell
        # Write the one entry in the cell pointer array.
        struct.pack_into(">H", root_buf, hdr_off + _INTERIOR_HDR, cell_off)

        self._pager.write(self._root_page, bytes(root_buf))

    def _split_leaf(
        self,
        leaf_pgno: int,
        all_cells: list[tuple[int, bytes]],
    ) -> tuple[int, int]:
        """Split a non-root leaf page into two halves.

        *leaf_pgno* is overwritten with the left half. A new right sibling
        is allocated for the right half.

        Non-root leaves always have ``hdr_off = 0`` — the 100-byte
        database header only appears on the root page (page 1).

        Split point: ``mid = len(all_cells) // 2``.

        Returns ``(separator_rowid, right_pgno)`` where *separator_rowid*
        is the maximum rowid in the left half — the key that propagates
        upward into the parent interior page.

        .. note::
            If *all_cells* contains payloads that originally lived on
            overflow pages, ``_write_cells_to_leaf`` re-encodes them into
            new overflow chains.  The old overflow pages become unreachable
            until phase 5 (freelist) recycles them.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        right_cells = all_cells[mid:]
        separator_rowid = left_cells[-1][0]

        right_pgno = self._allocate_page()
        # Reuse leaf_pgno for the left half; hdr_off=0 (non-root leaf).
        self._write_cells_to_leaf(leaf_pgno, 0, left_cells)
        self._write_cells_to_leaf(right_pgno, 0, right_cells)
        return separator_rowid, right_pgno

    def _write_interior_page(
        self,
        pgno: int,
        hdr_off: int,
        cells: list[tuple[int, int]],
        rightmost_child: int,
    ) -> None:
        """Write an interior page with *cells* and *rightmost_child*.

        Cells are written downward from ``PAGE_SIZE`` and pointers upward
        from ``hdr_off + _INTERIOR_HDR``. Any prefix bytes before *hdr_off*
        (the 100-byte SQLite database header on page 1) are preserved.

        Parameters
        ----------
        pgno:
            Target page number.
        hdr_off:
            Byte offset of the B-tree header within the page (0 or 100).
        cells:
            Sorted list of ``(left_child, sep_rowid)`` pairs.
        rightmost_child:
            Page number of the rightmost (largest-key) child.
        """
        ps = self._pager.page_size
        buf = bytearray(ps)
        if hdr_off > 0:
            orig = self._pager.read(pgno)
            buf[:hdr_off] = orig[:hdr_off]

        content_offset = ps
        ptrs: list[int] = []
        for left_child, sep_rowid in cells:
            cell = _interior_cell_encode(left_child, sep_rowid)
            content_offset -= len(cell)
            buf[content_offset : content_offset + len(cell)] = cell
            ptrs.append(content_offset)

        _write_interior_hdr(
            buf,
            hdr_off,
            ncells=len(cells),
            content_start=content_offset,
            rightmost_child=rightmost_child,
        )
        for i, ptr in enumerate(ptrs):
            struct.pack_into(">H", buf, hdr_off + _INTERIOR_HDR + i * _CELL_PTR, ptr)

        self._pager.write(pgno, bytes(buf))

    def _split_interior_page(
        self,
        pgno: int,
        hdr_off: int,
        all_cells: list[tuple[int, int]],
        rightmost_child: int,
    ) -> tuple[int, int]:
        """Split a non-root interior page.

        The median cell is promoted out of the page (it is not kept in
        either half). *pgno* is rewritten with the left half; a new right
        sibling is allocated for the right half.

        Split convention (n = len(all_cells), mid = n // 2)::

            left_page  : cells [0, mid),   rightmost_child = all_cells[mid].left_child
            median     : all_cells[mid]    → sep_rowid pushed up to parent
            right_page : cells [mid+1, n), rightmost_child = rightmost_child

        Returns ``(median_sep_rowid, right_pgno)``.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        median_left_child, median_sep = all_cells[mid]
        right_cells = all_cells[mid + 1 :]

        right_pgno = self._allocate_page()
        # Left half: reuse existing page.  Its rightmost_child is the left
        # child of the median cell (which is consumed by the promotion).
        self._write_interior_page(pgno, hdr_off, left_cells, median_left_child)
        # Right half: new page.  Its rightmost_child is the original.
        self._write_interior_page(right_pgno, 0, right_cells, rightmost_child)
        return median_sep, right_pgno

    def _split_root_interior(
        self,
        all_cells: list[tuple[int, int]],
        rightmost_child: int,
    ) -> None:
        """Split the root interior page into two interior children.

        When the root is an interior page and full, there is no parent to
        push a separator into. Instead:

        1. Pick the median cell ``all_cells[mid]``.
        2. Allocate ``left_pgno`` and ``right_pgno``.
        3. Write left half → ``left_pgno``,  rightmost = median.left_child.
        4. Write right half → ``right_pgno``, rightmost = *rightmost_child*.
        5. Rewrite the root with a single cell ``(left_pgno, median_sep)``
           and ``rightmost_child = right_pgno``.

        The root page number never changes.
        """
        mid = len(all_cells) // 2
        left_cells = all_cells[:mid]
        median_left_child, median_sep = all_cells[mid]
        right_cells = all_cells[mid + 1 :]

        left_pgno = self._allocate_page()
        right_pgno = self._allocate_page()

        self._write_interior_page(left_pgno, 0, left_cells, median_left_child)
        self._write_interior_page(right_pgno, 0, right_cells, rightmost_child)

        # Rewrite the root with a single separator cell.
        ps = self._pager.page_size
        hdr_off = self._header_offset
        root_buf = bytearray(ps)
        if hdr_off > 0:
            orig = self._pager.read(self._root_page)
            root_buf[:hdr_off] = orig[:hdr_off]

        cell = _interior_cell_encode(left_pgno, median_sep)
        cell_off = ps - len(cell)
        _write_interior_hdr(
            root_buf,
            hdr_off,
            ncells=1,
            content_start=cell_off,
            rightmost_child=right_pgno,
        )
        root_buf[cell_off : cell_off + len(cell)] = cell
        struct.pack_into(">H", root_buf, hdr_off + _INTERIOR_HDR, cell_off)

        self._pager.write(self._root_page, bytes(root_buf))

    def _push_separator_up(
        self,
        ancestors: list[tuple[int, int, int]],
        left_pgno: int,
        right_pgno: int,
        sep_rowid: int,
    ) -> None:
        """Insert a new separator into the nearest parent interior page.

        Called after any leaf or interior split.  *ancestors* is the path
        from the root down to the parent of the just-split page, with the
        direct parent at ``ancestors[-1]``.

        The parent currently holds a reference to *left_pgno* as one of
        its child pointers.  After the split, *left_pgno* now holds the
        smaller half and *right_pgno* holds the larger half.  We insert a
        new interior cell ``(left_pgno, sep_rowid)`` into the parent so
        that the tree routing is correct.

        Two placement cases arise depending on how the parent was reached:

        **Case A — chosen_idx ≥ 0** (parent cell ``i`` had ``left_child =
        left_pgno``):  insert new cell ``(left_pgno, sep_rowid)`` at index
        ``i``; replace cell ``i``'s left_child with ``right_pgno``::

            Before: ..., (left_pgno, old_sep), ...
            After:  ..., (left_pgno, sep_rowid), (right_pgno, old_sep), ...

        **Case B — chosen_idx = -1** (parent's ``rightmost_child =
        left_pgno``): append new cell ``(left_pgno, sep_rowid)`` and set
        ``rightmost_child = right_pgno``::

            Before: ..., (last_lc, last_sep),  rightmost = left_pgno
            After:  ..., (last_lc, last_sep), (left_pgno, sep_rowid),
                         rightmost = right_pgno

        If the resulting cell list fits in the parent, it is written
        directly.  If the parent is also full, it is split via
        :meth:`_split_interior_page` (or :meth:`_split_root_interior` if
        it is the root) and the process recurses with the grandparent.
        """
        parent_pgno, parent_hdr_off, chosen_idx = ancestors[-1]
        remaining = ancestors[:-1]

        parent_data = self._pager.read(parent_pgno)
        parent_hdr = _read_hdr(parent_data, parent_hdr_off)
        ncells = parent_hdr["ncells"]
        old_ptrs = _read_interior_ptrs(parent_data, parent_hdr_off, ncells, self._pager.page_size)
        old_cells = [_read_interior_cell(parent_data, ptr) for ptr in old_ptrs]
        old_rightmost = parent_hdr["rightmost_child"]

        # Build the updated cell list.
        if chosen_idx >= 0:
            # Case A: descended into old_cells[chosen_idx].left_child.
            # Replace that cell's left_child with right_pgno, and insert
            # (left_pgno, sep_rowid) before it.
            new_cells: list[tuple[int, int]] = []
            for i, (lc, sep) in enumerate(old_cells):
                if i == chosen_idx:
                    new_cells.append((left_pgno, sep_rowid))
                    new_cells.append((right_pgno, sep))
                else:
                    new_cells.append((lc, sep))
            new_rightmost = old_rightmost
        else:
            # Case B: descended into rightmost_child = left_pgno.
            # Append (left_pgno, sep_rowid) and update rightmost_child.
            new_cells = list(old_cells) + [(left_pgno, sep_rowid)]
            new_rightmost = right_pgno

        # Write the parent if the new cell list fits.
        if _interior_cells_fit(parent_hdr_off, new_cells, self._pager.page_size):
            self._write_interior_page(parent_pgno, parent_hdr_off, new_cells, new_rightmost)
            return

        # Parent is full — split it and recurse upward.
        if not remaining:
            # Parent IS the root interior page.
            self._split_root_interior(new_cells, new_rightmost)
        else:
            # Non-root interior page: split and push the median up.
            up_sep, new_right_pgno = self._split_interior_page(
                parent_pgno, parent_hdr_off, new_cells, new_rightmost
            )
            self._push_separator_up(remaining, parent_pgno, new_right_pgno, up_sep)

    # ── Cell I/O ──────────────────────────────────────────────────────────────

    def _read_cell(self, page_data: bytes | bytearray, ptr: int) -> tuple[int, bytes]:
        """Decode a full (rowid, payload) pair from *ptr*, following any
        overflow chain.

        Two safety guards protect against corrupt data:

        1. **Page-number validation** — each overflow pointer is checked
           against the pager's current logical size before dereferencing.
        2. **Cycle detection** — a ``visited`` set tracks every overflow
           page; a page appearing twice raises
           :class:`~storage_sqlite.errors.CorruptDatabaseError`.
        """
        offset = ptr
        total, n = varint_decode(page_data, offset)
        offset += n
        rowid, n = varint_decode_signed(page_data, offset)
        offset += n

        ps = self._pager.page_size
        local = _local_payload_size(total, ps)
        local_bytes = bytes(page_data[offset : offset + local])
        offset += local

        if total <= local:
            return rowid, local_bytes

        (overflow_pgno,) = struct.unpack_from(">I", page_data, offset)
        remaining = total - local
        overflow_usable = ps - 4
        chunks: list[bytes] = [local_bytes]
        visited: set[int] = set()
        while overflow_pgno != 0 and remaining > 0:
            if overflow_pgno < 1 or overflow_pgno > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"overflow page pointer {overflow_pgno} is out of range "
                    f"[1, {self._pager.size_pages}]"
                )
            if overflow_pgno in visited:
                raise CorruptDatabaseError(
                    f"circular overflow chain detected: page {overflow_pgno} "
                    f"was already visited"
                )
            visited.add(overflow_pgno)
            ov_data = self._pager.read(overflow_pgno)
            (next_pgno,) = struct.unpack_from(">I", ov_data, 0)
            chunk_size = min(remaining, overflow_usable)
            chunks.append(bytes(ov_data[4 : 4 + chunk_size]))
            remaining -= chunk_size
            overflow_pgno = next_pgno
        return rowid, b"".join(chunks)

    def _write_overflow(self, payload: bytes, local: int) -> int:
        """Spill ``payload[local:]`` into overflow pages.

        Returns the page number of the first overflow page (to embed in
        the leaf cell as the overflow pointer).
        """
        remaining = payload[local:]
        first_pgno = 0
        prev_pgno = 0
        prev_buf: bytearray | None = None

        overflow_usable = self._pager.page_size - 4
        ps = self._pager.page_size
        while remaining:
            pgno = self._allocate_page()
            buf = bytearray(ps)
            chunk = remaining[:overflow_usable]
            buf[4 : 4 + len(chunk)] = chunk
            struct.pack_into(">I", buf, 0, 0)  # next pointer = 0 (last)
            remaining = remaining[len(chunk):]

            if prev_buf is not None:
                struct.pack_into(">I", prev_buf, 0, pgno)
                self._pager.write(prev_pgno, bytes(prev_buf))
            else:
                first_pgno = pgno

            prev_pgno = pgno
            prev_buf = buf

        if prev_buf is not None:
            self._pager.write(prev_pgno, bytes(prev_buf))

        return first_pgno

    def _free_overflow(self, page_data: bytes | bytearray, ptr: int) -> None:
        """Free all overflow pages referenced by the cell at *ptr*.

        Each overflow page in the chain is passed to :meth:`_free_page`,
        which either returns it to the injected freelist (phase 5+) or
        zeroes it in place (backwards-compatible behaviour from phases 1–4).

        Same guards as :meth:`_read_cell`: page numbers are validated and
        circular chains are detected before any page is freed.
        """
        offset = ptr
        total, n = varint_decode(page_data, offset)
        offset += n
        _, n = varint_decode_signed(page_data, offset)  # skip rowid
        offset += n
        local = _local_payload_size(total, self._pager.page_size)
        if total <= local:
            return

        offset += local
        (overflow_pgno,) = struct.unpack_from(">I", page_data, offset)
        visited: set[int] = set()
        while overflow_pgno != 0:
            if overflow_pgno < 1 or overflow_pgno > self._pager.size_pages:
                raise CorruptDatabaseError(
                    f"overflow page pointer {overflow_pgno} is out of range "
                    f"[1, {self._pager.size_pages}]"
                )
            if overflow_pgno in visited:
                raise CorruptDatabaseError(
                    f"circular overflow chain detected: page {overflow_pgno} "
                    f"was already visited"
                )
            visited.add(overflow_pgno)
            ov_data = self._pager.read(overflow_pgno)
            (next_pgno,) = struct.unpack_from(">I", ov_data, 0)
            self._free_page(overflow_pgno)
            overflow_pgno = next_pgno

    def _write_cells_to_leaf(
        self,
        pgno: int,
        hdr_off: int,
        cells: list[tuple[int, bytes]],
    ) -> None:
        """Write a sorted list of (rowid, payload) cells to a leaf page.

        Used by :meth:`delete` (page compaction after removing a cell) and
        by :meth:`_split_root_leaf` and :meth:`_split_leaf` (writing
        split child pages).

        The byte layout is built from scratch: cells are written downward
        from ``PAGE_SIZE`` and pointers are written upward from
        ``hdr_off + _LEAF_HDR``.

        Important: any bytes before *hdr_off* (e.g. the 100-byte SQLite
        database header on page 1) are preserved by reading the existing
        page first.
        """
        ps = self._pager.page_size
        buf = bytearray(ps)
        # Preserve the database header or any other prefix bytes.
        if hdr_off > 0:
            orig = self._pager.read(pgno)
            buf[:hdr_off] = orig[:hdr_off]

        content_offset = ps  # grows down
        new_ptrs: list[int] = []

        for rowid, payload in cells:
            total = len(payload)
            local = _local_payload_size(total, ps)
            overflow_pgno = 0
            if total > local:
                overflow_pgno = self._write_overflow(payload, local)
            cell = (
                varint_encode(total)
                + varint_encode_signed(rowid)
                + payload[:local]
                + (struct.pack(">I", overflow_pgno) if total > local else b"")
            )
            cell_len = len(cell)
            # Defensive underflow guard: content area must not collide with
            # the pointer array.  Under normal operation this cannot happen
            # (these cells were already on the page), but an explicit check
            # catches logic bugs early rather than silently producing corrupt
            # data.
            ptr_array_end_check = (
                _ptr_array_base(hdr_off) + (len(new_ptrs) + 1) * _CELL_PTR
            )
            content_offset -= cell_len
            if content_offset < ptr_array_end_check:
                raise BTreeError(
                    f"internal error: content_offset={content_offset} underran "
                    f"pointer array end={ptr_array_end_check} while writing "
                    f"leaf page {pgno}"
                )
            buf[content_offset : content_offset + cell_len] = cell
            new_ptrs.append(content_offset)

        _write_hdr(buf, hdr_off, ncells=len(cells), content_start=content_offset)
        _write_ptrs(buf, hdr_off, new_ptrs)
        self._pager.write(pgno, bytes(buf))

    def _bisect(
        self, page_data: bytes | bytearray, ptrs: list[int], rowid: int
    ) -> int:
        """Return the index at which *rowid* should be inserted.

        Raises :class:`DuplicateRowidError` if *rowid* already exists.
        """
        lo, hi = 0, len(ptrs)
        while lo < hi:
            mid = (lo + hi) // 2
            mid_rowid = _cell_rowid_only(page_data, ptrs[mid])
            if mid_rowid < rowid:
                lo = mid + 1
            elif mid_rowid > rowid:
                hi = mid
            else:
                raise DuplicateRowidError(
                    f"rowid {rowid} already exists in page {self._root_page}"
                )
        return lo

    # ── Diagnostics ───────────────────────────────────────────────────────────

    def cell_count(self) -> int:
        """Return the total number of cells across all leaf pages.

        Traverses the entire B-tree, summing ``ncells`` from each leaf.
        For a single-page (leaf-root) tree this is equivalent to reading
        the root header directly.
        """
        return self._count_cells(self._root_page, self._header_offset)

    def free_space(self) -> int:
        """Return approximate free bytes on the root page.

        Accounts for the pointer array and the cell content area of the
        root page only; does not account for fragmented gaps (freeblocks)
        or free space on non-root pages.

        After a root split the root is an interior page; this method
        returns the free space on that interior page, not the combined
        free space across all leaves.
        """
        page_data = self._pager.read(self._root_page)
        hdr = _read_hdr(page_data, self._header_offset)
        if hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE:
            ptr_end = (
                self._header_offset + _INTERIOR_HDR + hdr["ncells"] * _CELL_PTR
            )
        else:
            ptr_end = _ptr_array_base(self._header_offset) + hdr["ncells"] * _CELL_PTR
        return hdr["content_start"] - ptr_end

    def free_all(self, freelist: Freelist) -> None:
        """Free every page in this B-tree, including all overflow chains.

        Traverses the entire tree (all interior pages, all leaf pages, all
        overflow chains) and returns each page to *freelist*. Page 1 is
        **never** freed — it is the database header page and always lives
        outside the freelist.

        Called by :meth:`~storage_sqlite.schema.Schema.drop_table` to
        reclaim all storage occupied by a dropped table.

        After this call the B-tree's pages must not be read or written
        again; the caller is responsible for removing any references to this
        tree (e.g. by deleting the ``sqlite_schema`` row).

        Algorithm:

        * **Leaf page** — free all overflow chains, then free the page itself.
        * **Interior page** — recursively free every child subtree first
          (left children + rightmost child), then free the interior page.

        A ``visited`` set prevents double-frees on corrupt databases that
        have cycles in their child-pointer chains.
        """
        old_freelist = self._freelist
        self._freelist = freelist
        try:
            self._free_subtree(self._root_page, self._header_offset, set())
        finally:
            self._freelist = old_freelist

    def _free_subtree(self, pgno: int, hdr_off: int, visited: set[int]) -> None:
        """Recursively free all pages reachable from *pgno*.

        Temporarily uses ``self._freelist`` (set by :meth:`free_all`) for
        all :meth:`_free_page` and :meth:`_free_overflow` calls.

        *hdr_off* is 100 for the root page of the ``sqlite_schema`` tree
        (page 1) and 0 for every other page.
        """
        if pgno == 0 or pgno in visited:
            return
        visited.add(pgno)

        page_data = self._pager.read(pgno)
        hdr = _read_hdr(page_data, hdr_off)

        if hdr["page_type"] == PAGE_TYPE_LEAF_TABLE:
            ptrs = _read_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
            for ptr in ptrs:
                self._free_overflow(page_data, ptr)
            # Page 1 is the database header — it is never freed.
            if pgno != 1:
                self._free_page(pgno)

        elif hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE:
            ptrs = _read_interior_ptrs(page_data, hdr_off, hdr["ncells"], self._pager.page_size)
            for ptr in ptrs:
                left_child, _ = _read_interior_cell(page_data, ptr)
                self._free_subtree(left_child, 0, visited)
            self._free_subtree(hdr["rightmost_child"], 0, visited)
            if pgno != 1:
                self._free_page(pgno)
