"""Tests for the B-tree layer — leaf pages, overflow chains, scan/find/CRUD."""

from __future__ import annotations

import contextlib
import struct
from pathlib import Path

import pytest

from storage_sqlite.btree import (
    _CELL_PTR,
    _LEAF_HDR,
    _MAX_LOCAL,
    _MIN_LOCAL,
    _OVERFLOW_USABLE,
    PAGE_TYPE_INTERIOR_TABLE,
    PAGE_TYPE_LEAF_TABLE,
    BTree,
    BTreeError,
    DuplicateRowidError,
    PageFullError,
    _init_leaf_page,
    _local_payload_size,
    _read_hdr,
    _read_ptrs,
)
from storage_sqlite.errors import CorruptDatabaseError
from storage_sqlite.pager import PAGE_SIZE, Pager
from storage_sqlite.record import decode as record_decode
from storage_sqlite.record import encode as record_encode
from storage_sqlite.varint import decode as varint_decode
from storage_sqlite.varint import decode_signed as varint_decode_signed

# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------


def make_pager(tmp_path: Path) -> Pager:
    return Pager.create(tmp_path / "db")


def btree(tmp_path: Path) -> tuple[BTree, Pager]:
    p = make_pager(tmp_path)
    b = BTree.create(p)
    return b, p


# ------------------------------------------------------------------
# _local_payload_size
# ------------------------------------------------------------------


def test_local_payload_no_overflow() -> None:
    assert _local_payload_size(100) == 100
    assert _local_payload_size(_MAX_LOCAL) == _MAX_LOCAL


def test_local_payload_overflow_min_local_or_less() -> None:
    # Values just above max_local may produce local = MIN_LOCAL.
    local = _local_payload_size(_MAX_LOCAL + 1)
    assert local <= _MAX_LOCAL
    assert local >= _MIN_LOCAL


def test_local_payload_never_exceeds_max_local() -> None:
    for total in range(4000, 10000, 100):
        assert _local_payload_size(total) <= _MAX_LOCAL


def test_local_payload_always_at_least_min_local_when_overflow() -> None:
    for total in range(_MAX_LOCAL + 1, _MAX_LOCAL + 5000, 50):
        assert _local_payload_size(total) >= _MIN_LOCAL


# ------------------------------------------------------------------
# _init_leaf_page / _read_hdr
# ------------------------------------------------------------------


def test_init_leaf_page_header() -> None:
    buf = bytearray(PAGE_SIZE)
    _init_leaf_page(buf)
    hdr = _read_hdr(buf, 0)
    assert hdr["page_type"] == PAGE_TYPE_LEAF_TABLE
    assert hdr["ncells"] == 0
    assert hdr["content_start"] == PAGE_SIZE
    assert hdr["freeblock"] == 0
    assert hdr["fragmented"] == 0


def test_init_leaf_page_with_offset() -> None:
    buf = bytearray(PAGE_SIZE)
    _init_leaf_page(buf, 100)
    hdr = _read_hdr(buf, 100)
    assert hdr["page_type"] == PAGE_TYPE_LEAF_TABLE
    assert hdr["ncells"] == 0


# ------------------------------------------------------------------
# BTree.create / open
# ------------------------------------------------------------------


def test_create_allocates_page(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        assert b.root_page == 1
        assert b.cell_count() == 0


def test_open_reads_existing_tree(tmp_path: Path) -> None:
    p = make_pager(tmp_path)
    b = BTree.create(p)
    b.insert(1, record_encode([42]))
    p.commit()
    p.close()

    p2 = Pager.open(tmp_path / "db")
    b2 = BTree.open(p2, 1)
    assert b2.cell_count() == 1
    p2.close()


# ------------------------------------------------------------------
# Insert / find basics
# ------------------------------------------------------------------


def test_insert_and_find_single(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, record_encode(["hello"]))
        raw = b.find(1)
        assert raw is not None
        values, _ = record_decode(raw)
        assert values == ["hello"]


def test_find_missing_rowid_returns_none(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, record_encode([1]))
        assert b.find(99) is None


def test_insert_multiple_sorted_rowids(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in [3, 1, 4, 1, 5, 9, 2, 6]:
            with contextlib.suppress(DuplicateRowidError):
                b.insert(rowid, record_encode([rowid]))
        # After de-dup: 1,2,3,4,5,6,9
        result = list(b.scan())
        rowids = [r for r, _ in result]
        assert rowids == sorted(rowids)


def test_insert_preserves_rowid_sort_order_when_inserted_reverse(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in [100, 50, 25, 10, 5, 1]:
            b.insert(rowid, record_encode([rowid]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == [1, 5, 10, 25, 50, 100]


def test_duplicate_rowid_raises(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, record_encode([1]))
        with pytest.raises(DuplicateRowidError):
            b.insert(1, record_encode([2]))


# ------------------------------------------------------------------
# Scan
# ------------------------------------------------------------------


def test_scan_empty_tree(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        assert list(b.scan()) == []


def test_scan_returns_records_in_rowid_order(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in range(20, 0, -1):
            b.insert(rowid, record_encode([rowid * 10]))
        pairs = list(b.scan())
        assert [r for r, _ in pairs] == list(range(1, 21))
        for rowid, raw in pairs:
            values, _ = record_decode(raw)
            assert values == [rowid * 10]


# ------------------------------------------------------------------
# Delete
# ------------------------------------------------------------------


def test_delete_existing_rowid(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in [1, 2, 3]:
            b.insert(rowid, record_encode([rowid]))
        assert b.delete(2) is True
        assert b.find(2) is None
        assert b.cell_count() == 2
        rows = list(b.scan())
        assert [r for r, _ in rows] == [1, 3]


def test_delete_missing_rowid_returns_false(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, record_encode([1]))
        assert b.delete(99) is False


def test_delete_then_reinsert(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, record_encode(["original"]))
        b.delete(1)
        b.insert(1, record_encode(["reinserted"]))
        raw = b.find(1)
        assert raw is not None
        values, _ = record_decode(raw)
        assert values == ["reinserted"]


# ------------------------------------------------------------------
# Update
# ------------------------------------------------------------------


def test_update_existing(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(5, record_encode(["old"]))
        assert b.update(5, record_encode(["new"])) is True
        raw = b.find(5)
        assert raw is not None
        values, _ = record_decode(raw)
        assert values == ["new"]


def test_update_missing_returns_false(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        assert b.update(99, record_encode(["x"])) is False


# ------------------------------------------------------------------
# Persistence across open/close
# ------------------------------------------------------------------


def test_commit_persists_across_reopen(tmp_path: Path) -> None:
    p = make_pager(tmp_path)
    b = BTree.create(p)
    for rowid in range(1, 6):
        b.insert(rowid, record_encode([f"row{rowid}"]))
    p.commit()
    p.close()

    p2 = Pager.open(tmp_path / "db")
    b2 = BTree.open(p2, 1)
    rows = list(b2.scan())
    assert len(rows) == 5
    for rowid, raw in rows:
        values, _ = record_decode(raw)
        assert values == [f"row{rowid}"]
    p2.close()


# ------------------------------------------------------------------
# Overflow chains
# ------------------------------------------------------------------


def test_large_payload_stored_correctly(tmp_path: Path) -> None:
    """A payload bigger than max_local must round-trip correctly."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"X" * (_MAX_LOCAL + 500)
        b.insert(1, big)
        assert b.find(1) == big


def test_multiple_overflow_pages(tmp_path: Path) -> None:
    """A record large enough to span several overflow pages."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        # 3 full overflow pages worth: MIN_LOCAL + 3 * (PAGE_SIZE - 4)
        size = _MIN_LOCAL + 3 * _OVERFLOW_USABLE + 100
        big = bytes(range(256)) * (size // 256 + 1)
        big = big[:size]
        b.insert(42, big)
        assert b.find(42) == big


def test_overflow_scan(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"A" * (_MAX_LOCAL + 1000)
        b.insert(7, big)
        b.insert(3, record_encode([1]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == [3, 7]
        assert rows[1][1] == big


def test_delete_frees_overflow_pages(tmp_path: Path) -> None:
    """After delete, scan must not see the row."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"Z" * (_MAX_LOCAL + 100)
        b.insert(1, big)
        b.delete(1)
        assert b.find(1) is None
        assert list(b.scan()) == []


def test_update_overflow_to_inline(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"B" * (_MAX_LOCAL + 200)
        b.insert(1, big)
        b.update(1, b"small")
        assert b.find(1) == b"small"


def test_update_inline_to_overflow(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        b.insert(1, b"small")
        big = b"C" * (_MAX_LOCAL + 200)
        b.update(1, big)
        assert b.find(1) == big


# ------------------------------------------------------------------
# Page-full detection
# ------------------------------------------------------------------


def test_page_full_raises(tmp_path: Path) -> None:
    """Overfill a page with many cells until PageFullError is raised."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        # Each cell with an empty record is ~3 bytes (2 varint + payload).
        # Header + N*2 pointers + N*3 cell bytes must stay within 4096.
        # Insert until the page is full.
        with pytest.raises(PageFullError):
            for rowid in range(1, 10_000):
                b.insert(rowid, record_encode([rowid]))


# ------------------------------------------------------------------
# header_offset = 100 (page 1 support)
# ------------------------------------------------------------------


def test_header_offset_100(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        # Manually set up a page-1-style btree at offset 100.
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        _init_leaf_page(buf, 100)
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno, header_offset=100)
        b.insert(1, record_encode(["page1"]))
        raw = b.find(1)
        assert raw is not None
        values, _ = record_decode(raw)
        assert values == ["page1"]


# ------------------------------------------------------------------
# Interior page guard
# ------------------------------------------------------------------


def test_insert_on_interior_page_raises(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = PAGE_TYPE_INTERIOR_TABLE
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(BTreeError, match="leaf"):
            b.insert(1, b"x")


def test_scan_on_interior_page_raises(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = PAGE_TYPE_INTERIOR_TABLE
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(BTreeError, match="leaf"):
            list(b.scan())


def test_find_on_interior_page_raises(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = PAGE_TYPE_INTERIOR_TABLE
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(BTreeError, match="leaf"):
            b.find(1)


# ------------------------------------------------------------------
# negative rowids (signed)
# ------------------------------------------------------------------


def test_negative_rowids(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in [-10, -5, -1, 0, 1, 5, 10]:
            b.insert(rowid, record_encode([rowid]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == [-10, -5, -1, 0, 1, 5, 10]
        for rowid, raw in rows:
            values, _ = record_decode(raw)
            assert values == [rowid]


# ------------------------------------------------------------------
# free_space helper
# ------------------------------------------------------------------


def test_free_space_decreases_on_insert(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        before = b.free_space()
        b.insert(1, record_encode([42]))
        assert b.free_space() < before


def test_free_space_recovers_after_delete(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        payload = record_encode([42])
        b.insert(1, payload)
        b.delete(1)
        # After compact-rewrite, free space should be back to near-maximum.
        assert b.free_space() > PAGE_SIZE - 100


# ------------------------------------------------------------------
# Structural constant sanity checks
# ------------------------------------------------------------------


def test_leaf_header_constant() -> None:
    assert _LEAF_HDR == 8


def test_max_local_for_4096_pages() -> None:
    assert _MAX_LOCAL == 4061


def test_min_local_for_4096_pages() -> None:
    assert _MIN_LOCAL == 489


def test_overflow_usable_for_4096_pages() -> None:
    assert _OVERFLOW_USABLE == PAGE_SIZE - 4


# ------------------------------------------------------------------
# _cell_size_on_page helper
# ------------------------------------------------------------------


def test_cell_size_on_page_no_overflow() -> None:
    from storage_sqlite.btree import _cell_size_on_page

    size = _cell_size_on_page(1, 10)
    # varint(10)=1, varint(1)=1, payload=10, no overflow
    assert size == 12


def test_cell_size_on_page_with_overflow() -> None:
    from storage_sqlite.btree import _cell_size_on_page
    from storage_sqlite.varint import encode as ve
    from storage_sqlite.varint import encode_signed as ves

    total = _MAX_LOCAL + 100
    local = _local_payload_size(total)
    size = _cell_size_on_page(1, total)
    # varint(total) + varint(1) + local + 4 (overflow ptr)
    expected = len(ve(total)) + len(ves(1)) + local + 4
    assert size == expected


# ------------------------------------------------------------------
# Delete on interior page raises
# ------------------------------------------------------------------


def test_delete_on_interior_page_raises(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = PAGE_TYPE_INTERIOR_TABLE
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(BTreeError, match="leaf"):
            b.delete(1)


# ------------------------------------------------------------------
# Rewrite-page with overflow (delete middle of multi-row tree where
# surrounding rows also have overflow — exercises _rewrite_page's
# _write_overflow branch)
# ------------------------------------------------------------------


def test_delete_middle_row_with_large_siblings(tmp_path: Path) -> None:
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"X" * (_MAX_LOCAL + 200)
        b.insert(1, big)
        b.insert(2, record_encode(["middle"]))
        b.insert(3, big)
        assert b.delete(2) is True
        rows = list(b.scan())
        assert [r for r, _ in rows] == [1, 3]
        assert rows[0][1] == big
        assert rows[1][1] == big


# ------------------------------------------------------------------
# Delete binary-search coverage — hi=mid branch
# ------------------------------------------------------------------


def test_delete_first_of_three_exercises_hi_branch(tmp_path: Path) -> None:
    """Deleting the smallest rowid forces the binary search to take hi=mid.

    With cells [1, 3, 5]:  mid=1 → rowid=3 > 1 → hi=1; then mid=0 → found.
    This exercises the ``hi = mid`` branch that is not reached when the
    target happens to be at the initial mid position.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in [1, 3, 5]:
            b.insert(rowid, record_encode([rowid]))
        assert b.delete(1) is True
        rows = list(b.scan())
        assert [r for r, _ in rows] == [3, 5]


# ------------------------------------------------------------------
# Security regression tests — corrupt page data guards
# ------------------------------------------------------------------


def _find_overflow_ptr_offset(page_data: bytes | bytearray, cell_ptr: int) -> int:
    """Navigate past total-payload varint + rowid varint + local bytes,
    returning the byte offset of the 4-byte overflow page pointer."""
    off = cell_ptr
    total, n = varint_decode(page_data, off)
    off += n
    _, n = varint_decode_signed(page_data, off)
    off += n
    off += _local_payload_size(total)
    return off


def test_read_ptrs_rejects_oversized_ncells() -> None:
    """ncells larger than can fit on the page must raise CorruptDatabaseError."""
    buf = bytearray(PAGE_SIZE)
    _init_leaf_page(buf, 0)
    max_possible = (PAGE_SIZE - _LEAF_HDR) // _CELL_PTR
    with pytest.raises(CorruptDatabaseError, match="ncells"):
        _read_ptrs(bytes(buf), 0, max_possible + 1)


def test_read_ptrs_rejects_pointer_into_header_area() -> None:
    """A cell pointer that aims inside the pointer array must be rejected."""
    fake = bytearray(PAGE_SIZE)
    _init_leaf_page(fake, 0)
    struct.pack_into(">H", fake, 3, 1)   # ncells = 1
    struct.pack_into(">H", fake, _LEAF_HDR, 4)  # pointer[0] = 4 (inside the 8-byte header)
    with pytest.raises(CorruptDatabaseError, match="cell pointer"):
        _read_ptrs(bytes(fake), 0, 1)


def test_read_ptrs_rejects_pointer_beyond_page() -> None:
    """A cell pointer >= PAGE_SIZE must be rejected."""
    fake = bytearray(PAGE_SIZE)
    _init_leaf_page(fake, 0)
    struct.pack_into(">H", fake, 3, 1)          # ncells = 1
    struct.pack_into(">H", fake, _LEAF_HDR, PAGE_SIZE)  # pointer = PAGE_SIZE (invalid)
    with pytest.raises(CorruptDatabaseError, match="cell pointer"):
        _read_ptrs(bytes(fake), 0, 1)


def test_insert_rejects_corrupt_content_start(tmp_path: Path) -> None:
    """A corrupt content_start that overlaps the pointer array must be rejected."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        page_data = bytearray(p.read(b.root_page))
        # content_start is at hdr_off + 5 (u16 BE).  Write 1, which is below the
        # pointer array end of 8 bytes, so the validation must fire.
        struct.pack_into(">H", page_data, 5, 1)
        p.write(b.root_page, bytes(page_data))
        with pytest.raises(CorruptDatabaseError, match="content_start"):
            b.insert(1, record_encode([42]))


def test_read_cell_rejects_out_of_range_overflow_pgno(tmp_path: Path) -> None:
    """An overflow pointer referencing a non-existent page must raise CorruptDatabaseError."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"Y" * (_MAX_LOCAL + 100)
        b.insert(1, big)
        # Corrupt the overflow pointer in the leaf cell to point to page 9999
        # (well beyond the 2-page database created by this insert).
        page_data = bytearray(p.read(b.root_page))
        hdr = _read_hdr(page_data, 0)
        ptrs = _read_ptrs(bytes(page_data), 0, hdr["ncells"])
        ov_off = _find_overflow_ptr_offset(page_data, ptrs[0])
        struct.pack_into(">I", page_data, ov_off, 9999)
        p.write(b.root_page, bytes(page_data))
        with pytest.raises(CorruptDatabaseError, match="overflow page pointer"):
            b.find(1)


def test_read_cell_rejects_circular_overflow_chain(tmp_path: Path) -> None:
    """A circular overflow chain must raise CorruptDatabaseError, not loop forever.

    A payload that needs exactly 2 overflow pages is used so that both pages
    are visited.  The first overflow page is made to point back to itself.
    On the second visit the cycle-detection set fires before remaining hits 0.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        # _local_payload_size(489 + 2*4092) = 489 (k=2, no remainder) →
        # overflow = 2 * 4092 bytes → two overflow pages.
        two_page_overflow_total = _MIN_LOCAL + 2 * _OVERFLOW_USABLE
        big = b"Z" * two_page_overflow_total
        b.insert(1, big)
        # Find the first overflow page number from the leaf cell.
        page_data = bytearray(p.read(b.root_page))
        hdr = _read_hdr(page_data, 0)
        ptrs = _read_ptrs(bytes(page_data), 0, hdr["ncells"])
        ov_off = _find_overflow_ptr_offset(page_data, ptrs[0])
        (first_ov_pgno,) = struct.unpack_from(">I", page_data, ov_off)
        # Make the first overflow page point back to itself instead of page 2.
        ov_buf = bytearray(p.read(first_ov_pgno))
        struct.pack_into(">I", ov_buf, 0, first_ov_pgno)
        p.write(first_ov_pgno, bytes(ov_buf))
        with pytest.raises(CorruptDatabaseError, match="circular"):
            b.find(1)


def test_free_overflow_rejects_out_of_range_pgno(tmp_path: Path) -> None:
    """_free_overflow must reject an overflow pointer to a non-existent page.

    We insert an overflow row, corrupt the overflow pointer in the leaf cell
    to an out-of-range value, then call delete() — which calls _free_overflow
    on the deleted cell.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"Y" * (_MAX_LOCAL + 100)
        b.insert(1, big)
        page_data = bytearray(p.read(b.root_page))
        hdr = _read_hdr(page_data, 0)
        ptrs = _read_ptrs(bytes(page_data), 0, hdr["ncells"])
        ov_off = _find_overflow_ptr_offset(page_data, ptrs[0])
        struct.pack_into(">I", page_data, ov_off, 9999)
        p.write(b.root_page, bytes(page_data))
        with pytest.raises(CorruptDatabaseError, match="overflow page pointer"):
            b.delete(1)


def test_free_overflow_rejects_circular_chain(tmp_path: Path) -> None:
    """_free_overflow must raise on a circular overflow chain, not loop forever.

    Unlike _read_cell, _free_overflow has no 'remaining' budget, so even a
    single-overflow-page self-loop is enough to trigger the cycle detector.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        big = b"Z" * (_MAX_LOCAL + 100)
        b.insert(1, big)
        # Find the overflow page number via the leaf cell's overflow pointer.
        page_data = bytearray(p.read(b.root_page))
        hdr = _read_hdr(page_data, 0)
        ptrs = _read_ptrs(bytes(page_data), 0, hdr["ncells"])
        ov_off = _find_overflow_ptr_offset(page_data, ptrs[0])
        (ov_pgno,) = struct.unpack_from(">I", page_data, ov_off)
        # Make the overflow page point to itself.
        ov_buf = bytearray(p.read(ov_pgno))
        struct.pack_into(">I", ov_buf, 0, ov_pgno)
        p.write(ov_pgno, bytes(ov_buf))
        with pytest.raises(CorruptDatabaseError, match="circular"):
            b.delete(1)
