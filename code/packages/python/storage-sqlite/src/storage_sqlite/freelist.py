"""
SQLite freelist — trunk/leaf page-reuse mechanism.

When does the freelist grow?
----------------------------

In SQLite a B-tree page is *never* removed from the tree due to ordinary
row-level DELETE — the page stays allocated, just with fewer cells.  Pages
return to the freelist only when their *payload* is released:

* **Overflow page deletion** — when a record that spills into overflow pages
  is deleted (or updated with a shorter payload), every overflow page in
  the chain is freed.
* **DROP TABLE / DROP INDEX** — every page in the entire B-tree is freed
  (phase 6).

The freelist is therefore primarily an overflow-page recycler in phase 5,
with DROP TABLE support added in phase 6.

On-disk layout
--------------

The freelist is rooted by two fields in the 100-byte database header on
page 1:

======= ===== ============================================================
offset   size  meaning
======= ===== ============================================================
32        4    Page number of the first **trunk page** (0 = freelist empty)
36        4    Total number of freelist pages (trunk + leaf combined)
======= ===== ============================================================

Each **trunk page** holds:

======= ===== ============================================================
offset   size  meaning
======= ===== ============================================================
0         4    Next trunk page number (0 = this is the last trunk)
4         4    Number of leaf entries stored in this trunk page
8+        4×N  Leaf page numbers (N up to 1 022 for 4 096-byte pages)
======= ===== ============================================================

**Leaf pages** are just freed data pages.  Their content is zero-filled on
allocation (so callers always receive clean pages) but their content while
waiting on the freelist is irrelevant — the freelist is a set of page
numbers, not a set of page contents.

Trunk-page capacity
-------------------

A 4 096-byte page has 8 bytes of trunk header, leaving 4 088 bytes for
leaf pointers of 4 bytes each → **1 022 leaf entries per trunk page**::

    (PAGE_SIZE - 8) // 4 == 1022

Free protocol (``free``)
------------------------

1. Read the header to get ``first_trunk`` and ``total``.
2. If there is a current trunk and it has room (count < 1 022): append
   *pgno* as a new leaf entry in that trunk.
3. Otherwise (no trunk, or trunk is full): promote *pgno* to a new trunk
   page with 0 leaf entries, pointing to the old trunk as its successor.
4. Increment ``total`` in the header.

Allocate protocol (``allocate``)
---------------------------------

1. If the freelist is empty (``first_trunk == 0``): fall through to
   ``Pager.allocate()`` to extend the file.
2. Read the first trunk.
3. If the trunk has leaf entries: pop the last leaf (LIFO — good locality),
   decrement trunk's count, zero-fill the leaf page, return it.
4. If the trunk has **no** leaf entries: the trunk page itself is returned,
   the header's ``first_trunk`` advances to the next trunk, the page is
   zero-filled.

Why LIFO?  SQLite pops from the end of the leaf array.  Using the same
policy helps our files look byte-similar to SQLite's output for the same
sequence of operations, which is one of the project's goals.

Thread / transaction safety
----------------------------

v1 is single-process, single-writer.  No locking is needed.  All reads
and writes go through the :class:`~storage_sqlite.pager.Pager` which
stages dirty pages in memory until :meth:`~storage_sqlite.pager.Pager.commit`.
A rollback reverts every freelist update atomically along with all other
dirty pages.

Invariants maintained
---------------------

* Page 1 is never freed — it is the database header page.
* ``total`` in the header equals the sum of all leaf entries across all
  trunk pages, plus the number of trunk pages themselves.
* Trunk pages are never listed as leaf entries — a page is either a trunk
  *or* a leaf, never both.
"""

from __future__ import annotations

import struct

from storage_sqlite.pager import PAGE_SIZE, Pager

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

_HDR_FIRST_TRUNK_OFFSET: int = 32
"""Offset inside page 1 of the first-trunk-page pointer (u32 BE)."""

_HDR_TOTAL_OFFSET: int = 36
"""Offset inside page 1 of the total-freelist-page count (u32 BE)."""

_TRUNK_NEXT_OFFSET: int = 0
"""Offset inside a trunk page of the next-trunk pointer (u32 BE)."""

_TRUNK_COUNT_OFFSET: int = 4
"""Offset inside a trunk page of the leaf-entry count (u32 BE)."""

_TRUNK_LEAVES_OFFSET: int = 8
"""Offset inside a trunk page where leaf page numbers begin (u32 BE each)."""

_LEAF_ENTRY_SIZE: int = 4
"""Bytes per leaf entry (a u32 page number)."""

TRUNK_CAPACITY: int = (PAGE_SIZE - _TRUNK_LEAVES_OFFSET) // _LEAF_ENTRY_SIZE
"""Maximum leaf entries per trunk page (1 022 for 4 096-byte pages).

Exposed for testing.  Formula: ``(PAGE_SIZE - 8) // 4``.
"""


# ---------------------------------------------------------------------------
# Freelist class
# ---------------------------------------------------------------------------


class Freelist:
    """Manages SQLite's trunk/leaf freelist of reusable pages.

    Pass a :class:`~storage_sqlite.pager.Pager` on construction.  The
    freelist reads and writes only through that pager, so all changes are
    staged in the pager's dirty-page table and committed or rolled back
    atomically with the rest of the transaction.

    Usage example::

        with Pager.create("app.db") as pager:
            fl = Freelist(pager)
            # ... B-tree operations that free overflow pages ...
            fl.free(overflow_pgno)          # returns page to freelist
            reused = fl.allocate()          # reuses it on next alloc
            pager.commit()
    """

    __slots__ = ("_pager",)

    def __init__(self, pager: Pager) -> None:
        self._pager: Pager = pager

    # ── Properties ────────────────────────────────────────────────────────────

    @property
    def total_pages(self) -> int:
        """Total number of pages currently on the freelist (trunk + leaf).

        Reads the header field at offset 36 of page 1.  Returns 0 when the
        freelist is empty.  This value is recomputed from disk on every
        access — call sparingly in tight loops.
        """
        _, total = self._read_header_fields()
        return total

    # ── Core operations ───────────────────────────────────────────────────────

    def allocate(self) -> int:
        """Return a reusable page number, or extend the file if the list is empty.

        The returned page is zero-filled in the pager's dirty table, so the
        caller always starts with a clean page regardless of what it held
        previously.

        Algorithm:

        1. If ``first_trunk == 0``: fall through to ``Pager.allocate()``.
        2. Read the first trunk page.
        3. If the trunk has leaf entries: pop the *last* leaf (LIFO), zero
           it, return it.
        4. If the trunk has no leaf entries: the trunk itself is the free
           page; advance ``first_trunk`` to the next trunk, zero the old
           trunk, return it.
        """
        first_trunk, total = self._read_header_fields()
        if first_trunk == 0:
            # Freelist empty — must extend the file.
            return self._pager.allocate()

        trunk_data = bytearray(self._pager.read(first_trunk))
        (next_trunk,) = struct.unpack_from(">I", trunk_data, _TRUNK_NEXT_OFFSET)
        (count,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)

        if count > 0:
            # Pop the last leaf entry (LIFO, matching SQLite behaviour).
            leaf_offset = _TRUNK_LEAVES_OFFSET + (count - 1) * _LEAF_ENTRY_SIZE
            (leaf_pgno,) = struct.unpack_from(">I", trunk_data, leaf_offset)
            # Decrement the leaf count in the trunk and write it back.
            struct.pack_into(">I", trunk_data, _TRUNK_COUNT_OFFSET, count - 1)
            self._pager.write(first_trunk, bytes(trunk_data))
            # Update the header total.
            self._write_header_fields(first_trunk, total - 1)
            # Zero-fill the leaf page so callers receive a clean slate.
            self._pager.write(leaf_pgno, b"\x00" * PAGE_SIZE)
            return leaf_pgno

        else:
            # Trunk has no leaves — return the trunk page itself.
            # Advance the header's first-trunk pointer to the next trunk.
            self._write_header_fields(next_trunk, total - 1)
            # Zero-fill the old trunk page.
            self._pager.write(first_trunk, b"\x00" * PAGE_SIZE)
            return first_trunk

    def free(self, pgno: int) -> None:
        """Add *pgno* to the freelist.

        *pgno* must be a valid, allocated page number other than page 1 (the
        database header page).  The caller is responsible for ensuring the
        page is no longer referenced anywhere in the database.

        Algorithm:

        1. If there is a current trunk with room (count < :data:`TRUNK_CAPACITY`):
           append *pgno* as a new leaf entry.
        2. Otherwise (no trunk, or current trunk is full): promote *pgno* to a
           new trunk page with 0 leaf entries, pointing to the old trunk as its
           successor, and update the header's ``first_trunk``.
        3. Increment ``total`` in the header.
        """
        if pgno <= 0:
            raise ValueError(f"pgno must be >= 1, got {pgno}")
        if pgno == 1:
            raise ValueError("page 1 (the database header page) cannot be freed")
        if pgno > self._pager.size_pages:
            raise ValueError(
                f"pgno {pgno} is beyond the current database size "
                f"({self._pager.size_pages} pages)"
            )

        first_trunk, total = self._read_header_fields()

        if first_trunk != 0:
            trunk_data = bytearray(self._pager.read(first_trunk))
            (count,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)
            if count < TRUNK_CAPACITY:
                # There is room in the current trunk: append *pgno* as a leaf.
                leaf_offset = _TRUNK_LEAVES_OFFSET + count * _LEAF_ENTRY_SIZE
                struct.pack_into(">I", trunk_data, leaf_offset, pgno)
                struct.pack_into(">I", trunk_data, _TRUNK_COUNT_OFFSET, count + 1)
                self._pager.write(first_trunk, bytes(trunk_data))
                self._write_header_fields(first_trunk, total + 1)
                return

        # No trunk exists, or the current trunk is full.
        # Promote *pgno* to a new trunk page that points to the old trunk.
        new_trunk = bytearray(PAGE_SIZE)
        struct.pack_into(">I", new_trunk, _TRUNK_NEXT_OFFSET, first_trunk)
        struct.pack_into(">I", new_trunk, _TRUNK_COUNT_OFFSET, 0)
        self._pager.write(pgno, bytes(new_trunk))
        self._write_header_fields(pgno, total + 1)

    # ── Internal helpers ──────────────────────────────────────────────────────

    def _read_header_fields(self) -> tuple[int, int]:
        """Read ``(first_trunk_pgno, total_freelist_pages)`` from page 1.

        Both values are u32 BE at fixed offsets in the 100-byte database
        header.  This reads page 1 through the pager (so dirty updates to
        page 1 within the current transaction are visible immediately).
        """
        page1 = self._pager.read(1)
        (first_trunk,) = struct.unpack_from(">I", page1, _HDR_FIRST_TRUNK_OFFSET)
        (total,) = struct.unpack_from(">I", page1, _HDR_TOTAL_OFFSET)
        return first_trunk, total

    def _write_header_fields(self, first_trunk: int, total: int) -> None:
        """Update the freelist fields in the page-1 header.

        Reads page 1, overwrites only the two freelist fields, and writes the
        modified page back through the pager.  All other header bytes
        (including the B-tree cell area starting at offset 100) are untouched.
        """
        buf = bytearray(self._pager.read(1))
        struct.pack_into(">I", buf, _HDR_FIRST_TRUNK_OFFSET, first_trunk)
        struct.pack_into(">I", buf, _HDR_TOTAL_OFFSET, total)
        self._pager.write(1, bytes(buf))
