"""
Tests for ``storage_sqlite.freelist`` — the SQLite trunk/leaf freelist.

Test organisation
-----------------

Unit tests for :class:`~storage_sqlite.freelist.Freelist` in isolation:

1. **Empty freelist** — ``total_pages`` is 0, ``allocate()`` falls through to
   ``Pager.allocate()``, the file grows.
2. **Single free/allocate round-trip** — free a page, check ``total_pages``
   becomes 1, then allocate and get the same page back (LIFO).
3. **Multiple free then allocate (LIFO order)** — pages are returned in
   last-freed-first order, matching SQLite's behaviour.
4. **Trunk promotion** — free more than TRUNK_CAPACITY pages; a second trunk
   is created, and all pages are recoverable.
5. **Trunk-with-no-leaves is returned itself** — when an empty trunk is the
   only freelist entry, ``allocate()`` returns the trunk page and sets
   ``first_trunk`` to 0.
6. **Header fields on page 1** — ``Freelist`` only touches bytes 32–39 of
   page 1; all other bytes (including B-tree header at offset 100) are
   preserved.
7. **Persist across pager reopen** — commit, reopen, freelist state survives.
8. **Rollback reverts freelist** — ``Pager.rollback()`` after freelist
   mutations restores the prior state.
9. **Validation errors** — freeing page 1, page 0, or out-of-range pages
   raises ``ValueError``.

Integration tests for BTree + Freelist:

10. **Overflow pages freed and reused** — insert a large record (overflow),
    delete it, then insert another large record; the new record reuses the
    old overflow pages rather than extending the file.
11. **freelist=None preserves old behaviour** — all existing btree tests
    continue to pass because freelist defaults to None.
12. **BTree.create with freelist** — creating a tree reuses a freelist page
    for the root.
13. **Persist with freelist** — freelist state and B-tree data both survive
    commit + reopen.
"""

from __future__ import annotations

import struct

import pytest

from storage_sqlite import TRUNK_CAPACITY, BTree, Freelist, Pager, record
from storage_sqlite.freelist import (
    _HDR_FIRST_TRUNK_OFFSET,
    _HDR_TOTAL_OFFSET,
    _TRUNK_COUNT_OFFSET,
    _TRUNK_LEAVES_OFFSET,
    _TRUNK_NEXT_OFFSET,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_db(tmp_path):
    """Create a minimal 1-page database, return (pager, path).

    Page 1 is written so that the freelist fields at offsets 32 and 36 are
    initialised to zero (no freelist).  The B-tree uses the page-1 header
    area (offset 100), so we need the pager to think page 1 exists.
    """
    path = str(tmp_path / "test.db")
    pager = Pager.create(path)
    # Allocate page 1 and write a zeroed header (freelist fields = 0).
    pager.allocate()  # page 1
    pager.write(1, b"\x00" * pager.page_size)
    pager.commit()
    return pager, path


def _read_header_freelist(pager):
    """Return (first_trunk, total) from the page-1 header."""
    page1 = pager.read(1)
    (first_trunk,) = struct.unpack_from(">I", page1, _HDR_FIRST_TRUNK_OFFSET)
    (total,) = struct.unpack_from(">I", page1, _HDR_TOTAL_OFFSET)
    return first_trunk, total


# ---------------------------------------------------------------------------
# Unit tests — Freelist in isolation
# ---------------------------------------------------------------------------


class TestFreelistEmpty:
    """A freshly initialised freelist with total=0."""

    def test_total_pages_is_zero(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        assert fl.total_pages == 0

    def test_allocate_extends_file(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Page 1 already exists.  Allocating via freelist falls through to
        # pager.allocate() which returns page 2.
        pgno = fl.allocate()
        assert pgno == 2
        assert pager.size_pages == 2

    def test_header_fields_unchanged(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        fl.allocate()
        first_trunk, total = _read_header_freelist(pager)
        assert first_trunk == 0
        assert total == 0


class TestFreelistSinglePage:
    """Free one page, then allocate it back."""

    def test_free_increments_total(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Allocate a data page.
        pgno = pager.allocate()  # page 2
        # Return it to the freelist.
        fl.free(pgno)
        assert fl.total_pages == 1

    def test_free_sets_first_trunk_in_header(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()  # page 2
        fl.free(pgno)
        first_trunk, _ = _read_header_freelist(pager)
        # pgno became a trunk page (no existing trunk to add a leaf to).
        assert first_trunk == pgno

    def test_allocate_returns_freed_page(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()  # page 2
        fl.free(pgno)
        reused = fl.allocate()
        assert reused == pgno

    def test_allocate_decrements_total(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()
        fl.free(pgno)
        fl.allocate()
        assert fl.total_pages == 0

    def test_allocate_returns_zeroed_page(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()
        # Write something to the page to confirm zeroing happens.
        pager.write(pgno, b"\xFF" * pager.page_size)
        fl.free(pgno)
        reused = fl.allocate()
        assert pager.read(reused) == b"\x00" * pager.page_size

    def test_after_allocate_freelist_empty(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()
        fl.free(pgno)
        fl.allocate()
        first_trunk, total = _read_header_freelist(pager)
        assert first_trunk == 0
        assert total == 0


class TestFreelistMultiplePages:
    """Free several pages; verify LIFO order on allocate."""

    def test_lifo_order(self, tmp_path):
        """Pages are returned last-freed-first."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Allocate three data pages.
        p2 = pager.allocate()  # 2
        p3 = pager.allocate()  # 3
        p4 = pager.allocate()  # 4
        # p2 becomes first trunk (no trunk yet).
        fl.free(p2)
        # p3 is added as a leaf to the trunk (p2).
        fl.free(p3)
        # p4 is added as a leaf to the trunk (p2).
        fl.free(p4)
        assert fl.total_pages == 3
        # Allocate pops LIFO from the trunk's leaf array: p4, p3, then p2.
        assert fl.allocate() == p4
        assert fl.allocate() == p3
        # p2 is the trunk with no leaves — returned as the trunk itself.
        assert fl.allocate() == p2
        assert fl.total_pages == 0

    def test_total_pages_tracks_all_frees(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pages = [pager.allocate() for _ in range(10)]
        # First free creates a trunk, the remaining 9 are leaves.
        for p in pages:
            fl.free(p)
        assert fl.total_pages == 10

    def test_all_pages_recoverable(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        n = 20
        freed = [pager.allocate() for _ in range(n)]
        # First free -> trunk; remaining -> leaves, then new trunks as leaf
        # entries overflow.  In practice all 20 fit in one trunk (cap = 1022).
        for p in freed:
            fl.free(p)
        recovered = [fl.allocate() for _ in range(n)]
        assert sorted(recovered) == sorted(freed)


class TestFreelistTrunkPromotion:
    """When the current trunk is full, a new trunk is created."""

    def test_trunk_capacity_constant(self):
        """TRUNK_CAPACITY == (4096 - 8) // 4 == 1022."""
        assert TRUNK_CAPACITY == 1022

    def test_new_trunk_when_first_trunk_full(self, tmp_path):
        """Freeing TRUNK_CAPACITY + 2 pages creates two trunks.

        Trace (pages numbered from 2):

        * pages[0]  → no existing trunk, pages[0] becomes trunk, count=0
        * pages[1..TRUNK_CAPACITY] → TRUNK_CAPACITY leaves added, count=TRUNK_CAPACITY
        * pages[TRUNK_CAPACITY+1] → trunk is full, pages[TRUNK_CAPACITY+1] becomes
          a new trunk pointing to the old trunk as ``next``, count=0

        So ``TRUNK_CAPACITY + 2`` frees are needed to overflow a trunk.
        """
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Allocate TRUNK_CAPACITY + 2 data pages.
        pages = [pager.allocate() for _ in range(TRUNK_CAPACITY + 2)]
        for p in pages:
            fl.free(p)
        assert fl.total_pages == TRUNK_CAPACITY + 2
        # Header's first_trunk should point to the newest (second) trunk.
        first_trunk, total = _read_header_freelist(pager)
        assert total == TRUNK_CAPACITY + 2
        # Read the newest trunk to verify it is a fresh trunk page.
        trunk_data = pager.read(first_trunk)
        (nxt,) = struct.unpack_from(">I", trunk_data, _TRUNK_NEXT_OFFSET)
        (count,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)
        assert count == 0   # new trunk, no leaves yet
        assert nxt != 0     # old trunk is the successor

    def test_all_recoverable_across_two_trunks(self, tmp_path):
        """All TRUNK_CAPACITY + 2 freed pages can be allocated back."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pages = [pager.allocate() for _ in range(TRUNK_CAPACITY + 2)]
        for p in pages:
            fl.free(p)
        recovered = [fl.allocate() for _ in range(TRUNK_CAPACITY + 2)]
        assert sorted(recovered) == sorted(pages)
        assert fl.total_pages == 0


class TestFreelistHeaderPreservation:
    """Freelist only touches bytes 32–39 of page 1; B-tree data is untouched."""

    def test_bytes_outside_freelist_fields_unchanged(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        # Write a recognisable pattern to page 1 except at the freelist fields.
        buf = bytearray(b"\xAB" * pager.page_size)
        # Zero the freelist fields so Freelist sees an empty freelist.
        struct.pack_into(">II", buf, _HDR_FIRST_TRUNK_OFFSET, 0, 0)
        pager.write(1, bytes(buf))

        fl = Freelist(pager)
        pgno = pager.allocate()  # page 2
        fl.free(pgno)

        page1_after = pager.read(1)
        # Bytes 0..31 must still be 0xAB.
        assert page1_after[:_HDR_FIRST_TRUNK_OFFSET] == b"\xAB" * _HDR_FIRST_TRUNK_OFFSET
        # Bytes 40..4095 must still be 0xAB.
        assert page1_after[_HDR_TOTAL_OFFSET + 4 :] == b"\xAB" * (
            pager.page_size - _HDR_TOTAL_OFFSET - 4
        )

    def test_btree_header_area_untouched(self, tmp_path):
        """Bytes 100..4095 (B-tree cell area on page 1) are never touched."""
        pager, _ = _make_db(tmp_path)
        buf = bytearray(pager.page_size)
        # Plant a sentinel in the B-tree area (offset 100).
        buf[100] = 0x5A
        pager.write(1, bytes(buf))

        fl = Freelist(pager)
        pgno = pager.allocate()
        fl.free(pgno)
        fl.allocate()

        assert pager.read(1)[100] == 0x5A


class TestFreelistPersistence:
    """Freelist state survives commit + reopen."""

    def test_persist_and_reopen(self, tmp_path):
        path = str(tmp_path / "fl.db")
        pager = Pager.create(path)
        pager.allocate()  # page 1
        pager.write(1, b"\x00" * pager.page_size)

        fl = Freelist(pager)
        p2 = pager.allocate()
        p3 = pager.allocate()
        fl.free(p3)  # leaf in p2-as-trunk is p3 → actually p2 is trunk, p3 is leaf
        fl.free(p2)  # wait — let me think: first free(p3): no trunk, p3 becomes trunk
        # Re-check: free(p3) → p3 is new trunk, count=0. free(p2) → p2 added as leaf to trunk p3
        # So trunk=p3, leaf=[p2], total=2
        pager.commit()
        pager.close()

        pager2 = Pager.open(path)
        fl2 = Freelist(pager2)
        assert fl2.total_pages == 2
        # LIFO: p2 was last leaf → comes out first.
        assert fl2.allocate() == p2
        # Now trunk p3 has no leaves → trunk itself is returned.
        assert fl2.allocate() == p3
        assert fl2.total_pages == 0
        pager2.close()


class TestFreelistRollback:
    """Pager.rollback() reverts freelist mutations."""

    def test_rollback_reverts_free(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()
        fl.free(pgno)
        assert fl.total_pages == 1
        pager.rollback()
        # After rollback page 1 reverts → total should be 0.
        assert fl.total_pages == 0

    def test_rollback_reverts_allocate(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Seed the freelist.
        pgno = pager.allocate()
        fl.free(pgno)
        pager.commit()  # commit the free so it survives.

        # Now allocate in a new txn and roll back.
        fl.allocate()
        assert fl.total_pages == 0  # page was popped
        pager.rollback()
        # After rollback the freelist is back to 1.
        assert fl.total_pages == 1


class TestFreelistValidation:
    """freelist.free() rejects invalid page numbers."""

    def test_free_page_one_raises(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        with pytest.raises(ValueError, match="page 1"):
            fl.free(1)

    def test_free_page_zero_raises(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        with pytest.raises(ValueError, match="pgno must be >= 1"):
            fl.free(0)

    def test_free_beyond_size_raises(self, tmp_path):
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        with pytest.raises(ValueError, match="beyond the current database size"):
            fl.free(9999)


# ---------------------------------------------------------------------------
# Trunk page structure tests
# ---------------------------------------------------------------------------


class TestTrunkPageStructure:
    """Inspect the raw bytes of trunk pages after free() calls."""

    def test_first_free_creates_trunk_with_zero_count(self, tmp_path):
        """The first freed page becomes a trunk with count=0, next=0."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        pgno = pager.allocate()  # page 2
        fl.free(pgno)
        trunk_data = pager.read(pgno)
        (nxt,) = struct.unpack_from(">I", trunk_data, _TRUNK_NEXT_OFFSET)
        (cnt,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)
        assert nxt == 0
        assert cnt == 0

    def test_second_free_adds_leaf_to_trunk(self, tmp_path):
        """Second freed page is stored as a leaf in the first trunk."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        p2 = pager.allocate()  # 2 → will become trunk
        p3 = pager.allocate()  # 3 → will become leaf
        fl.free(p2)
        fl.free(p3)
        trunk_data = pager.read(p2)
        (cnt,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)
        (leaf,) = struct.unpack_from(">I", trunk_data, _TRUNK_LEAVES_OFFSET)
        assert cnt == 1
        assert leaf == p3

    def test_allocate_leaf_clears_trunk_count(self, tmp_path):
        """After popping the sole leaf, the trunk count drops to 0."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        p2 = pager.allocate()  # trunk
        p3 = pager.allocate()  # leaf
        fl.free(p2)
        fl.free(p3)
        fl.allocate()  # pops p3
        trunk_data = pager.read(p2)
        (cnt,) = struct.unpack_from(">I", trunk_data, _TRUNK_COUNT_OFFSET)
        assert cnt == 0


# ---------------------------------------------------------------------------
# Integration tests — BTree + Freelist
# ---------------------------------------------------------------------------


_LARGE_PAYLOAD = b"Y" * 4500  # > _MAX_LOCAL (4061), forces overflow


class TestBTreeFreelistIntegration:
    """BTree integrates with Freelist for overflow page reuse."""

    def test_overflow_pages_reused_after_delete(self, tmp_path):
        """File size does not grow when a deleted record's overflow is reused."""
        path = str(tmp_path / "tree.db")
        with Pager.create(path) as pager:
            pager.allocate()  # page 1 (header)
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            # Insert a large record — this allocates overflow pages.
            tree.insert(1, record.encode([_LARGE_PAYLOAD]))
            size_after_insert = pager.size_pages
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            tree = BTree.open(pager, 2, freelist=fl)  # root was page 2
            # Delete frees overflow pages back to the freelist.
            tree.delete(1)
            size_after_delete = pager.size_pages
            # Insert another large record — should reuse the freed overflow pages.
            tree.insert(2, record.encode([_LARGE_PAYLOAD]))
            size_after_reinsert = pager.size_pages
            pager.commit()

        # After reinserting a record of the same size, the file must be no
        # larger than it was right after the first insert.
        assert size_after_reinsert <= size_after_insert
        # And the delete must have populated the freelist (not grown the file).
        assert size_after_delete == size_after_insert  # file didn't shrink

    def test_freelist_total_increases_on_delete(self, tmp_path):
        """Freelist total grows by ≥1 after deleting a record with overflow."""
        path = str(tmp_path / "tree2.db")
        with Pager.create(path) as pager:
            pager.allocate()
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            tree.insert(1, record.encode([_LARGE_PAYLOAD]))
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            before = fl.total_pages
            tree = BTree.open(pager, 2, freelist=fl)
            tree.delete(1)
            after = fl.total_pages
            assert after > before

    def test_create_reuses_freelist_page(self, tmp_path):
        """BTree.create uses a freelist page for its root when available."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        # Allocate and free a page so the freelist has one entry.
        p2 = pager.allocate()
        fl.free(p2)
        assert fl.total_pages == 1
        # Create a B-tree: root should reuse p2 from the freelist.
        tree = BTree.create(pager, freelist=fl)
        assert tree.root_page == p2
        assert fl.total_pages == 0

    def test_no_freelist_still_works(self, tmp_path):
        """BTree without freelist continues to work exactly as before."""
        pager, _ = _make_db(tmp_path)
        tree = BTree.create(pager)  # no freelist
        for i in range(1, 101):
            tree.insert(i, record.encode([i]))
        assert tree.cell_count() == 100
        tree.delete(50)
        assert tree.cell_count() == 99

    def test_persist_with_freelist(self, tmp_path):
        """Data and freelist both survive commit + reopen."""
        path = str(tmp_path / "persist.db")
        with Pager.create(path) as pager:
            pager.allocate()
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            root = tree.root_page
            for i in range(1, 51):
                tree.insert(i, record.encode([f"row{i}"]))
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            tree = BTree.open(pager, root, freelist=fl)
            rows = [(rid, record.decode(pl)[0][0]) for rid, pl in tree.scan()]
            assert rows == [(i, f"row{i}") for i in range(1, 51)]

    def test_multiple_overflow_pages_all_freed(self, tmp_path):
        """A very large record (multiple overflow pages) frees all of them."""
        huge_payload = b"Z" * 20_000  # ~5 overflow pages
        path = str(tmp_path / "huge.db")
        with Pager.create(path) as pager:
            pager.allocate()
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            tree.insert(1, record.encode([huge_payload]))
            size_after_insert = pager.size_pages
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            tree = BTree.open(pager, 2, freelist=fl)
            tree.delete(1)
            # The freelist now has all the overflow pages.
            freed_count = fl.total_pages
            assert freed_count >= 4  # at least 4 overflow pages for 20 000 bytes

            # Reinserting the same payload reuses all of them.
            tree.insert(2, record.encode([huge_payload]))
            assert pager.size_pages <= size_after_insert

    def test_update_with_overflow_frees_old_pages(self, tmp_path):
        """update() frees the old overflow pages via freelist."""
        path = str(tmp_path / "upd.db")
        with Pager.create(path) as pager:
            pager.allocate()
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            tree.insert(1, record.encode([_LARGE_PAYLOAD]))
            size_after_insert = pager.size_pages
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            tree = BTree.open(pager, 2, freelist=fl)
            # Update with the same large payload — old overflow freed, new reused.
            tree.update(1, record.encode([_LARGE_PAYLOAD]))
            assert pager.size_pages <= size_after_insert
            # Verify data reads back correctly.
            payload, _ = record.decode(tree.find(1))
            assert payload[0] == _LARGE_PAYLOAD
            pager.commit()


class TestBTreeFreelistNoOverflow:
    """Ensure freelist integration doesn't break small-record operations."""

    def test_small_records_no_freelist_interaction(self, tmp_path):
        """Small records (no overflow) don't touch the freelist."""
        pager, _ = _make_db(tmp_path)
        fl = Freelist(pager)
        tree = BTree.create(pager, freelist=fl)
        before = fl.total_pages
        for i in range(1, 200):
            tree.insert(i, record.encode([i, f"name{i}"]))
        for i in range(1, 100):
            tree.delete(i)
        after = fl.total_pages
        # No overflow → freelist should be unchanged.
        assert after == before

    def test_scan_correct_after_freelist_operations(self, tmp_path):
        """scan() returns correct rows after mixed overflow free/reuse."""
        path = str(tmp_path / "mixed.db")
        with Pager.create(path) as pager:
            pager.allocate()
            pager.write(1, b"\x00" * pager.page_size)
            fl = Freelist(pager)
            tree = BTree.create(pager, freelist=fl)
            root = tree.root_page
            # Insert a mix of large and small records.
            tree.insert(1, record.encode([_LARGE_PAYLOAD]))
            tree.insert(2, record.encode(["small"]))
            tree.insert(3, record.encode([_LARGE_PAYLOAD]))
            pager.commit()

        with Pager.open(path) as pager:
            fl = Freelist(pager)
            tree = BTree.open(pager, root, freelist=fl)
            # Delete the two large records (frees overflow pages).
            tree.delete(1)
            tree.delete(3)
            # Insert a new large record (reuses overflow pages).
            tree.insert(4, record.encode([_LARGE_PAYLOAD]))
            rows = list(tree.scan())
            assert len(rows) == 2  # rowids 2 and 4
            assert rows[0][0] == 2
            assert rows[1][0] == 4
            payload4, _ = record.decode(rows[1][1])
            assert payload4[0] == _LARGE_PAYLOAD
            pager.commit()
