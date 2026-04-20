"""
Table B-tree pages — phase 3: leaf pages with overflow chains.

Architecture summary
--------------------

SQLite organises every table as a **table B-tree**: a balanced tree of
4 096-byte pages where each leaf holds a sorted array of (rowid, record)
pairs and each interior node routes a search to the right child page. v1
phase 3 implements the *leaf + overflow* portion; interior pages and page
splits land in phase 4.

Page memory layout
------------------

Every B-tree page has a small header, a cell pointer array that grows
from the header toward the middle, a free gap in the middle, and a cell
content area that grows from the end of the page toward the middle::

    page_offset = 0 (non-page-1) or 100 (page 1 — database header sits first)
    ┌──────────────────────────────────────────────────────┐
    │ B-tree page header (8 bytes for leaf, 12 for interior)│
    ├──────────────────────────────────────────────────────┤
    │ Cell pointer array  ──────────────► grows up          │
    ├──────────────────────────────────────────────────────┤
    │                 free space                           │
    ├──────────────────────────────────────────────────────┤
    │ Cell content area   ◄─────────────── grows down       │
    └──────────────────────────────────────────────────────┘
    page byte 4095 (last usable byte)

**Page header fields** (all multi-byte fields big-endian):

::

    offset  size  meaning
       0     1    page type:  0x0D = leaf table,  0x05 = interior table
       1     2    first freeblock offset (0 = none)
       3     2    number of cells (u16)
       5     2    cell content area start (0 means 65 536; 4096 for a fresh page)
       7     1    fragmented free bytes

Interior pages additionally have:

::

       8     4    rightmost child page number (u32)

**Cell pointer array** starts at ``header_offset + 8`` (leaf) or
``header_offset + 12`` (interior). Each entry is a u16 big-endian offset
pointing to a cell in the content area. Entries are **sorted ascending by
rowid** — the sort lives in the pointer array, not in the cell bytes
themselves (cells are written newest-first, i.e. highest-to-lowest offset
within the page).

Leaf table cell wire format
---------------------------

::

    [total-payload-size  varint]   ← whole record length, incl. overflow
    [rowid               varint]   ← signed (encodes negative rowids)
    [local payload bytes       ]   ← first L bytes of the record
    [overflow page pointer u32 ]   ← only present when total > max_local

The local portion ``L`` is computed by :func:`_local_payload_size` to
keep the on-page footprint in a sweet spot regardless of total record
size.

Overflow pages
--------------

When a record is too large to fit in one page::

    overflow page layout:
        0..3   next overflow page number u32  (0 = last page in chain)
        4..4095 payload continuation bytes

The first ``L`` bytes are inline. The remainder fills overflow pages,
chained by the u32 at their start.

v1 limitations (phase 3)
------------------------

- Only leaf pages are read/written. Encountering an interior page raises
  :class:`BTreeError`.
- No page splits. :meth:`BTree.insert` raises :class:`PageFullError` if
  the leaf is too full to hold the new cell.
- No compacting of freeblocks within a page (unused space from
  :meth:`BTree.delete` is not reclaimed until the page is rewritten by a
  subsequent :meth:`BTree.insert` that triggers a defragmentation pass).
- Page size is pinned at 4 096; *reserved_per_page* is always 0.
- The ``header_offset`` parameter accommodates page 1 (database header
  at bytes 0–99, B-tree header at byte 100). Callers that want a plain
  B-tree root page pass the default of 0.
"""

from __future__ import annotations

import struct
from collections.abc import Iterator

from storage_sqlite.errors import CorruptDatabaseError, StorageError
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.varint import decode as varint_decode
from storage_sqlite.varint import decode_signed as varint_decode_signed
from storage_sqlite.varint import encode as varint_encode
from storage_sqlite.varint import encode_signed as varint_encode_signed

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

# ── Overflow thresholds ───────────────────────────────────────────────────────

# SQLite formulas (§2.3.4 of the file-format spec):
#   usableSize  = pageSize - reservedSize  (reservedSize = 0 in v1)
#   maxLocal    = usableSize - 35
#   minLocal    = floor((usableSize - 12) * 32 / 255) - 23
#
# For 4 096-byte pages:
#   maxLocal = 4061
#   minLocal = floor(4084 * 32 / 255) - 23 = 512 - 23 = 489

_USABLE: int = PAGE_SIZE  # 4 096
_MAX_LOCAL: int = _USABLE - 35  # 4 061
_MIN_LOCAL: int = ((_USABLE - 12) * 32) // 255 - 23  # 489
_OVERFLOW_USABLE: int = _USABLE - 4  # bytes per overflow page (minus 4-byte next ptr)


# ── Errors ────────────────────────────────────────────────────────────────────


class BTreeError(StorageError):
    """Base for all B-tree layer errors."""


class PageFullError(BTreeError):
    """The leaf page has no room for the new cell.

    Phase 3 raises this rather than splitting the page — that lands in
    phase 4. Callers that want to grow a B-tree beyond one page must
    upgrade to the phase-4 implementation.
    """


class DuplicateRowidError(BTreeError):
    """A cell with the same rowid already exists in the leaf."""


# ── Internal helpers ──────────────────────────────────────────────────────────


def _local_payload_size(total: int) -> int:
    """Compute how many bytes of ``total`` stay on the leaf page.

    Returns ``total`` unchanged when no overflow is needed. When overflow
    is needed, returns the local portion per the SQLite formula — always
    ≤ ``_MAX_LOCAL`` and always ≥ ``_MIN_LOCAL``.

    The formula is designed so that adjacent overflow pages are used
    efficiently: the local portion is the largest value of the form
    ``minLocal + k * (pageSize - 4)`` that is ≤ maxLocal, given the
    total payload. In practice this pushes as much as possible inline
    while keeping overflow pages close to full.
    """
    if total <= _MAX_LOCAL:
        return total
    local = _MIN_LOCAL + (total - _MIN_LOCAL) % _OVERFLOW_USABLE
    return _MIN_LOCAL if local > _MAX_LOCAL else local


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


def _cell_size_on_page(rowid: int, total_payload: int) -> int:
    """Return the number of bytes a cell occupies on the leaf page.

    This is the varint for total-payload, plus the varint for rowid, plus
    the local payload portion, plus 4 if there is an overflow pointer.
    """
    local = _local_payload_size(total_payload)
    overflow = total_payload > local
    return (
        len(varint_encode(total_payload))
        + len(varint_encode_signed(rowid))
        + local
        + (4 if overflow else 0)
    )


def _read_hdr(page_data: bytes | bytearray, hdr_off: int) -> dict[str, int]:
    """Parse the 8-byte leaf page header at *hdr_off*."""
    page_type = page_data[hdr_off]
    freeblock, ncells, content_start, fragmented = struct.unpack_from(
        ">HHHB", page_data, hdr_off + 1
    )
    if content_start == 0:
        content_start = 65536
    return {
        "page_type": page_type,
        "freeblock": freeblock,
        "ncells": ncells,
        "content_start": content_start,
        "fragmented": fragmented,
    }


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


def _ptr_array_base(hdr_off: int) -> int:
    """Return the byte offset of the first cell pointer entry."""
    return hdr_off + _LEAF_HDR


def _read_ptrs(page_data: bytes | bytearray, hdr_off: int, ncells: int) -> list[int]:
    """Return the cell pointer array as a list of absolute page offsets.

    Guards against corrupt page data in two ways:

    1. **ncells cap** — a corrupt ``ncells`` of 65 535 would cause 131 070
       bytes of ``struct.unpack_from`` reads on a 4 096-byte page, reading
       far past the buffer and raising an obscure error deep inside
       ``struct``.  We reject any ``ncells`` that cannot physically fit.

    2. **pointer range** — each u16 pointer must land *inside* the cell
       content area, i.e. strictly after the pointer array itself and
       strictly before ``PAGE_SIZE``.  A pointer aimed at the page header,
       the pointer array, or beyond the page boundary would silently
       mis-decode cell data or produce out-of-bounds reads.

    Raises :class:`~storage_sqlite.errors.CorruptDatabaseError` if either
    check fails.
    """
    base = _ptr_array_base(hdr_off)

    # Maximum number of 2-byte pointers that can possibly fit on the page.
    max_possible: int = (PAGE_SIZE - hdr_off - _LEAF_HDR) // _CELL_PTR
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
        if ptr < ptr_array_end or ptr >= PAGE_SIZE:
            raise CorruptDatabaseError(
                f"cell pointer[{i}]={ptr} is outside valid cell-content range "
                f"[{ptr_array_end}, {PAGE_SIZE})"
            )
        ptrs.append(ptr)
    return ptrs


def _write_ptrs(buf: bytearray, hdr_off: int, ptrs: list[int]) -> None:
    """Write the cell pointer array from *ptrs* into *buf*."""
    base = _ptr_array_base(hdr_off)
    for i, ptr in enumerate(ptrs):
        struct.pack_into(">H", buf, base + i * _CELL_PTR, ptr)


def _init_leaf_page(buf: bytearray, hdr_off: int = 0) -> None:
    """Write a fresh empty leaf table page header into *buf*."""
    _write_hdr(buf, hdr_off, ncells=0, content_start=PAGE_SIZE)


# ── BTree ─────────────────────────────────────────────────────────────────────


class BTree:
    """Leaf-only table B-tree (phase 3 — no splits).

    The B-tree occupies a single root page (the root is also the only leaf
    for now). :meth:`insert` raises :class:`PageFullError` when the page
    would overflow — phase 4 adds splits and interior pages.

    All reads and writes go through the injected :class:`~storage_sqlite.pager.Pager`
    which handles caching, journalling, and crash recovery. The B-tree
    itself is stateless between calls — it reads and writes the root page
    on every operation.

    Parameters
    ----------
    pager:
        The page I/O layer this tree lives in.
    root_page:
        1-based page number of the root (and only leaf) page.
    header_offset:
        Byte offset within the root page where the B-tree page header
        starts. 0 for ordinary pages; 100 for page 1 (which has the
        100-byte SQLite database header in front of it).
    """

    __slots__ = ("_header_offset", "_pager", "_root_page")

    def __init__(
        self,
        pager: Pager,
        root_page: int,
        *,
        header_offset: int = 0,
    ) -> None:
        self._pager: Pager = pager
        self._root_page: int = root_page
        self._header_offset: int = header_offset

    # ── Construction ──────────────────────────────────────────────────────────

    @classmethod
    def create(
        cls,
        pager: Pager,
        *,
        header_offset: int = 0,
    ) -> BTree:
        """Allocate a fresh leaf page and initialise it as an empty B-tree.

        Returns a :class:`BTree` rooted at the newly allocated page. The
        caller must :meth:`~storage_sqlite.pager.Pager.commit` the pager
        when ready to persist.
        """
        pgno = pager.allocate()
        buf = bytearray(pager.read(pgno))
        _init_leaf_page(buf, header_offset)
        pager.write(pgno, bytes(buf))
        return cls(pager, pgno, header_offset=header_offset)

    @classmethod
    def open(
        cls,
        pager: Pager,
        root_page: int,
        *,
        header_offset: int = 0,
    ) -> BTree:
        """Open an existing B-tree rooted at *root_page*."""
        return cls(pager, root_page, header_offset=header_offset)

    # ── Properties ────────────────────────────────────────────────────────────

    @property
    def root_page(self) -> int:
        """1-based page number of the B-tree root."""
        return self._root_page

    # ── Public operations ─────────────────────────────────────────────────────

    def insert(self, rowid: int, payload: bytes) -> None:
        """Insert *(rowid, payload)* into the leaf.

        *payload* is the raw record bytes produced by
        :func:`~storage_sqlite.record.encode`. The B-tree stores and
        returns it opaquely — it does not interpret the record structure.

        If the total payload exceeds the per-page inline threshold
        (:data:`_MAX_LOCAL` = 4 061 bytes), the tail is spilled to one or
        more overflow pages allocated from the pager, and only the local
        portion stays on the leaf.

        Raises
        ------
        DuplicateRowidError
            A cell with the same *rowid* already exists.
        PageFullError
            The leaf page has insufficient free space for the new cell.
            Phase 4 will resolve this with splits; for now the caller must
            use a larger page or fewer rows.
        """
        page_data = bytearray(self._pager.read(self._root_page))
        hdr_off = self._header_offset
        hdr = _read_hdr(page_data, hdr_off)
        if hdr["page_type"] != PAGE_TYPE_LEAF_TABLE:
            raise BTreeError(
                f"page {self._root_page} is not a leaf table page "
                f"(type=0x{hdr['page_type']:02x}); phase 3 supports leaf only"
            )

        ncells = hdr["ncells"]
        content_start = hdr["content_start"]
        ptrs = _read_ptrs(page_data, hdr_off, ncells)

        # Validate content_start.  A corrupt header could give us a value
        # that overlaps the pointer array (content_start too low) or points
        # beyond the page (content_start > PAGE_SIZE).  Either would cause
        # us to write cell data on top of the pointer array or past the
        # buffer, silently producing a corrupt page.
        ptr_array_end_now: int = _ptr_array_base(hdr_off) + ncells * _CELL_PTR
        if content_start > PAGE_SIZE or content_start < ptr_array_end_now:
            raise CorruptDatabaseError(
                f"content_start={content_start} is out of valid range "
                f"[{ptr_array_end_now}, {PAGE_SIZE}] on page {self._root_page}"
            )

        # Binary-search for insert position; also detect duplicates.
        insert_idx = self._bisect(page_data, ptrs, rowid)

        # Build the overflow chain first (if needed), so we know the
        # overflow page number before writing the leaf cell.
        total = len(payload)
        local = _local_payload_size(total)
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
        cell_size = len(cell)

        # Check free space: pointer array must not collide with cell area.
        ptr_array_end = _ptr_array_base(hdr_off) + (ncells + 1) * _CELL_PTR
        new_content_start = content_start - cell_size
        if new_content_start < ptr_array_end:
            raise PageFullError(
                f"leaf page {self._root_page} is full "
                f"(need {cell_size + _CELL_PTR} bytes, "
                f"only {content_start - ptr_array_end} available)"
            )

        # Write cell at the bottom of the content area.
        page_data[new_content_start : new_content_start + cell_size] = cell

        # Shift pointer entries right of insert_idx to make room.
        base = _ptr_array_base(hdr_off)
        for i in range(ncells, insert_idx, -1):
            src = base + (i - 1) * _CELL_PTR
            dst = base + i * _CELL_PTR
            page_data[dst : dst + _CELL_PTR] = page_data[src : src + _CELL_PTR]

        # Write the new pointer.
        struct.pack_into(">H", page_data, base + insert_idx * _CELL_PTR, new_content_start)

        # Update the page header.
        _write_hdr(
            page_data,
            hdr_off,
            ncells=ncells + 1,
            content_start=new_content_start,
            freeblock=hdr["freeblock"],
            fragmented=hdr["fragmented"],
        )
        self._pager.write(self._root_page, bytes(page_data))

    def find(self, rowid: int) -> bytes | None:
        """Return the payload for *rowid*, or ``None`` if not found."""
        page_data = self._pager.read(self._root_page)
        hdr_off = self._header_offset
        hdr = _read_hdr(page_data, hdr_off)
        if hdr["page_type"] != PAGE_TYPE_LEAF_TABLE:
            raise BTreeError(
                f"page {self._root_page} is not a leaf table page "
                f"(type=0x{hdr['page_type']:02x})"
            )
        ptrs = _read_ptrs(page_data, hdr_off, hdr["ncells"])

        # Binary search.
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

        Follows overflow chains transparently, so the yielded payload is
        always the complete record bytes regardless of record size.
        """
        page_data = self._pager.read(self._root_page)
        hdr_off = self._header_offset
        hdr = _read_hdr(page_data, hdr_off)
        if hdr["page_type"] != PAGE_TYPE_LEAF_TABLE:
            raise BTreeError(
                f"page {self._root_page} is not a leaf table page "
                f"(type=0x{hdr['page_type']:02x})"
            )
        for ptr in _read_ptrs(page_data, hdr_off, hdr["ncells"]):
            yield self._read_cell(page_data, ptr)

    def delete(self, rowid: int) -> bool:
        """Remove the cell for *rowid*.

        Returns ``True`` if the cell was found and removed, ``False`` if
        *rowid* does not exist. Overflow pages for the deleted cell are
        freed (zeroed and returned to the pager's "allocated but empty"
        pool — proper freelist integration lands in phase 5).

        The page is **compacted** after deletion: remaining cells are
        rewritten from scratch into a fresh cell content area, so deleted
        space is immediately reclaimed without any freeblock bookkeeping.
        """
        page_data = bytearray(self._pager.read(self._root_page))
        hdr_off = self._header_offset
        hdr = _read_hdr(page_data, hdr_off)
        if hdr["page_type"] != PAGE_TYPE_LEAF_TABLE:
            raise BTreeError(
                f"page {self._root_page} is not a leaf table page "
                f"(type=0x{hdr['page_type']:02x})"
            )
        ptrs = _read_ptrs(page_data, hdr_off, hdr["ncells"])

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
                # Free overflow pages before dropping the cell.
                self._free_overflow(page_data, ptr)
            else:
                survivors.append(self._read_cell(page_data, ptr))

        # Rebuild the page from scratch with surviving cells.
        self._rewrite_page(survivors)
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

    # ── Internals ─────────────────────────────────────────────────────────────

    def _read_cell(self, page_data: bytes | bytearray, ptr: int) -> tuple[int, bytes]:
        """Decode a full (rowid, payload) pair from *ptr*, following any
        overflow chain.
        """
        offset = ptr
        total, n = varint_decode(page_data, offset)
        offset += n
        rowid, n = varint_decode_signed(page_data, offset)
        offset += n

        local = _local_payload_size(total)
        local_bytes = bytes(page_data[offset : offset + local])
        offset += local

        if total <= local:
            return rowid, local_bytes

        # Overflow: follow the chain.
        #
        # Two safety guards protect against corrupt data:
        #
        # 1. **Page-number validation** — each overflow pointer is checked
        #    against the pager's current logical size before dereferencing.
        #    An out-of-range pointer would let a corrupt file direct reads
        #    to an arbitrary page (e.g. the root page itself, which would
        #    decode garbage as payload).
        #
        # 2. **Cycle detection** — track every page number visited.  A
        #    circular chain (A → B → A → …) would otherwise loop forever,
        #    consuming CPU and memory until the process is killed.
        (overflow_pgno,) = struct.unpack_from(">I", page_data, offset)
        remaining = total - local
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
            chunk_size = min(remaining, _OVERFLOW_USABLE)
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

        while remaining:
            pgno = self._pager.allocate()
            buf = bytearray(PAGE_SIZE)
            chunk = remaining[:_OVERFLOW_USABLE]
            buf[4 : 4 + len(chunk)] = chunk
            struct.pack_into(">I", buf, 0, 0)  # next pointer = 0 (last)
            remaining = remaining[len(chunk):]

            if prev_buf is not None:
                # Point the previous overflow page at this one.
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
        """Zero out any overflow pages referenced by the cell at *ptr*.

        Phase 5 will add proper freelist integration; for now the pages
        are zeroed so they don't contain stale data, but they are not
        reclaimed for future allocations.
        """
        offset = ptr
        total, n = varint_decode(page_data, offset)
        offset += n
        _, n = varint_decode_signed(page_data, offset)  # skip rowid
        offset += n
        local = _local_payload_size(total)
        if total <= local:
            return  # no overflow

        offset += local
        (overflow_pgno,) = struct.unpack_from(">I", page_data, offset)
        # Same guards as _read_cell: validate page numbers and detect
        # circular chains before zeroing pages so a corrupt pointer
        # cannot trigger infinite looping or zeroing of arbitrary pages.
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
            self._pager.write(overflow_pgno, b"\x00" * PAGE_SIZE)
            overflow_pgno = next_pgno

    def _rewrite_page(self, cells: list[tuple[int, bytes]]) -> None:
        """Rebuild the root page from scratch using *cells*.

        Cells are written newest-first into the content area (lowest
        pointer offset first) while the pointer array is written in
        ascending rowid order from the bottom of the header upward.
        Both are derived from *cells* which must already be sorted
        ascending by rowid.
        """
        buf = bytearray(PAGE_SIZE)
        hdr_off = self._header_offset
        buf[hdr_off : hdr_off + PAGE_SIZE] = b"\x00" * (PAGE_SIZE - hdr_off)

        content_offset = PAGE_SIZE  # grows down
        new_ptrs: list[int] = []

        for rowid, payload in cells:
            total = len(payload)
            local = _local_payload_size(total)
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
            # Defensive underflow guard: the content area must not collide
            # with the growing pointer array.  Under normal operation this
            # cannot happen because _rewrite_page is only called with cells
            # that were already on the page, but an explicit check catches
            # logic bugs early rather than silently producing corrupt data.
            ptr_array_end_check = _ptr_array_base(hdr_off) + (len(new_ptrs) + 1) * _CELL_PTR
            content_offset -= cell_len
            if content_offset < ptr_array_end_check:
                raise BTreeError(
                    f"internal error: content_offset={content_offset} underran "
                    f"pointer array end={ptr_array_end_check} during page rewrite "
                    f"of page {self._root_page}"
                )
            buf[content_offset : content_offset + cell_len] = cell
            new_ptrs.append(content_offset)

        _write_hdr(buf, hdr_off, ncells=len(cells), content_start=content_offset)
        _write_ptrs(buf, hdr_off, new_ptrs)
        self._pager.write(self._root_page, bytes(buf))

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
        """Return the number of cells on the root page."""
        page_data = self._pager.read(self._root_page)
        return _read_hdr(page_data, self._header_offset)["ncells"]

    def free_space(self) -> int:
        """Return approximate free bytes on the root page.

        Accounts for the pointer array and the cell content area; does not
        account for fragmented gaps within the content area (freeblocks).
        """
        page_data = self._pager.read(self._root_page)
        hdr = _read_hdr(page_data, self._header_offset)
        ptr_end = _ptr_array_base(self._header_offset) + hdr["ncells"] * _CELL_PTR
        return hdr["content_start"] - ptr_end
