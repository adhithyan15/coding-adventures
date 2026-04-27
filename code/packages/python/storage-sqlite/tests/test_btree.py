"""Tests for the B-tree layer — leaf pages, overflow chains, scan/find/CRUD,
interior page traversal, root-leaf splits, and full recursive splits."""

from __future__ import annotations

import contextlib
import struct
from pathlib import Path

import pytest

from storage_sqlite.btree import (
    _CELL_PTR,
    _INTERIOR_HDR,
    _LEAF_HDR,
    _MAX_LOCAL,
    _MIN_LOCAL,
    _OVERFLOW_USABLE,
    PAGE_TYPE_INTERIOR_TABLE,
    PAGE_TYPE_LEAF_TABLE,
    BTree,
    DuplicateRowidError,
    PageFullError,
    _init_leaf_page,
    _interior_cell_encode,
    _interior_cells_fit,
    _local_payload_size,
    _read_hdr,
    _read_interior_cell,
    _read_interior_ptrs,
    _read_ptrs,
    _write_interior_hdr,
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
# PageFullError is kept in the API but no longer raised by normal inserts
# ------------------------------------------------------------------


def test_page_full_error_is_exported() -> None:
    """PageFullError must remain importable from storage_sqlite (public API)."""
    assert issubclass(PageFullError, Exception)


def test_many_rows_no_page_full_error(tmp_path: Path) -> None:
    """Phase 4b: inserting many rows must not raise PageFullError.

    Previously (phase 4a) this would raise once a non-root leaf filled up.
    Recursive splits now handle all cases transparently.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in range(1, 2001):
            b.insert(rowid, record_encode([rowid]))
        assert b.cell_count() == 2000


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
# Interior page helpers — unit tests
# ------------------------------------------------------------------


def test_write_read_interior_hdr() -> None:
    """_write_interior_hdr / _read_hdr round-trip for interior pages."""
    buf = bytearray(PAGE_SIZE)
    _write_interior_hdr(buf, 0, ncells=3, content_start=3800, rightmost_child=7)
    hdr = _read_hdr(buf, 0)
    assert hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
    assert hdr["ncells"] == 3
    assert hdr["content_start"] == 3800
    assert hdr["rightmost_child"] == 7


def test_interior_cell_encode_decode() -> None:
    """_interior_cell_encode / _read_interior_cell round-trip."""
    cell = _interior_cell_encode(42, 999)
    # 4 bytes for left_child + varint(999) = 2 bytes → 6 bytes total
    left, sep = _read_interior_cell(cell + b"\x00" * 10, 0)
    assert left == 42
    assert sep == 999


def test_read_interior_ptrs_rejects_oversized_ncells() -> None:
    """ncells too large for an interior page must raise CorruptDatabaseError."""
    buf = bytearray(PAGE_SIZE)
    _write_interior_hdr(buf, 0, ncells=0, content_start=PAGE_SIZE, rightmost_child=1)
    max_possible = (PAGE_SIZE - _INTERIOR_HDR) // _CELL_PTR
    with pytest.raises(CorruptDatabaseError, match="ncells"):
        _read_interior_ptrs(bytes(buf), 0, max_possible + 1)


# ------------------------------------------------------------------
# Interior corrupt-child guard
# ------------------------------------------------------------------


def test_find_corrupt_zero_child_raises(tmp_path: Path) -> None:
    """Interior page with rightmost_child=0 (invalid) must raise CorruptDatabaseError."""
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        _write_interior_hdr(buf, 0, ncells=0, content_start=PAGE_SIZE, rightmost_child=0)
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(CorruptDatabaseError, match="invalid child pointer"):
            b.find(1)


def test_scan_corrupt_zero_child_raises(tmp_path: Path) -> None:
    """Interior page with rightmost_child=0 must raise CorruptDatabaseError during scan."""
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        _write_interior_hdr(buf, 0, ncells=0, content_start=PAGE_SIZE, rightmost_child=0)
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(CorruptDatabaseError):
            list(b.scan())


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
# Phase 4a — root-leaf split + multi-level traversal
# ------------------------------------------------------------------


def _fill_until_split(b: BTree, start: int = 1) -> int:
    """Insert rows with sequential rowids until the root becomes interior.

    Returns the rowid of the last successfully inserted row.  At that
    point the root page type is PAGE_TYPE_INTERIOR_TABLE.
    """
    rowid = start
    while True:
        b.insert(rowid, record_encode([rowid]))
        root_data = b._pager.read(b.root_page)
        if root_data[0] == PAGE_TYPE_INTERIOR_TABLE:
            return rowid
        rowid += 1


def test_root_split_promotes_root_to_interior(tmp_path: Path) -> None:
    """After filling a leaf-root, the root must become an interior page."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        _fill_until_split(b)
        root_data = p.read(b.root_page)
        assert root_data[0] == PAGE_TYPE_INTERIOR_TABLE


def test_root_split_cell_count(tmp_path: Path) -> None:
    """cell_count() must sum cells across both child leaves after a split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        for extra in range(last + 1, last + 11):
            b.insert(extra, record_encode([extra]))
        assert b.cell_count() == last + 10


def test_root_split_find_all_rows(tmp_path: Path) -> None:
    """find() must locate every row regardless of which child leaf it lives in."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        for rowid in range(1, last + 1):
            raw = b.find(rowid)
            assert raw is not None, f"find({rowid}) returned None after split"
            values, _ = record_decode(raw)
            assert values == [rowid]


def test_root_split_scan_order(tmp_path: Path) -> None:
    """scan() must return all rows in ascending rowid order after a split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, last + 1))
        for rowid, raw in rows:
            values, _ = record_decode(raw)
            assert values == [rowid]


def test_root_split_delete_from_left_leaf(tmp_path: Path) -> None:
    """delete() works on a row in the left child leaf after a split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        # Rowid 1 is the smallest — guaranteed in the left leaf.
        assert b.delete(1) is True
        assert b.find(1) is None
        assert b.cell_count() == last - 1


def test_root_split_delete_from_right_leaf(tmp_path: Path) -> None:
    """delete() works on a row in the right child leaf after a split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        # Largest rowid is guaranteed in the right leaf.
        assert b.delete(last) is True
        assert b.find(last) is None
        assert b.cell_count() == last - 1


def test_root_split_update(tmp_path: Path) -> None:
    """update() replaces records correctly on both sides of the split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        assert b.update(1, record_encode(["left-updated"])) is True
        assert b.update(last, record_encode(["right-updated"])) is True
        vals_l, _ = record_decode(b.find(1))  # type: ignore[arg-type]
        vals_r, _ = record_decode(b.find(last))  # type: ignore[arg-type]
        assert vals_l == ["left-updated"]
        assert vals_r == ["right-updated"]


def test_root_split_find_missing_returns_none(tmp_path: Path) -> None:
    """find() returns None for a non-existent rowid after a split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        _fill_until_split(b)
        assert b.find(999_999) is None


def test_root_split_duplicate_rowid_raises(tmp_path: Path) -> None:
    """DuplicateRowidError is still raised on the correct child leaf."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        with pytest.raises(DuplicateRowidError):
            b.insert(last, record_encode(["dup"]))


def test_root_split_delete_missing_returns_false(tmp_path: Path) -> None:
    """delete() returns False for a rowid not in the split tree."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        _fill_until_split(b)
        assert b.delete(999_999) is False


def test_root_split_persist_and_reopen(tmp_path: Path) -> None:
    """Split tree written and committed must round-trip through close/reopen."""
    p = make_pager(tmp_path)
    b = BTree.create(p)
    last = _fill_until_split(b)
    p.commit()
    p.close()

    p2 = Pager.open(tmp_path / "db")
    b2 = BTree.open(p2, 1)
    assert b2.cell_count() == last
    rows = list(b2.scan())
    assert [r for r, _ in rows] == list(range(1, last + 1))
    p2.close()


def test_root_split_reverse_insert_order(tmp_path: Path) -> None:
    """Descending-order inserts produce a correctly-split tree."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        count = 500
        for rowid in range(count, 0, -1):
            b.insert(rowid, record_encode([rowid]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, count + 1))


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


# ------------------------------------------------------------------
# Phase 4a — extra coverage for new interior-page code paths
# ------------------------------------------------------------------


def test_interior_cell_ptr_range_error() -> None:
    """_read_interior_ptrs rejects a cell pointer that lands in the header area."""
    buf = bytearray(PAGE_SIZE)
    # Write an interior header with ncells=1.
    _write_interior_hdr(buf, 0, ncells=1, content_start=PAGE_SIZE, rightmost_child=2)
    # The pointer array base for an interior page is offset 12.
    # Write a pointer value of 5, which is inside the 12-byte header.
    struct.pack_into(">H", buf, _INTERIOR_HDR, 5)
    with pytest.raises(CorruptDatabaseError, match="interior cell pointer"):
        _read_interior_ptrs(bytes(buf), 0, 1)


def test_find_unexpected_page_type_raises(tmp_path: Path) -> None:
    """_find_leaf_page raises CorruptDatabaseError on an unknown page type byte."""
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = 0xFF  # unknown type
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(CorruptDatabaseError, match="unexpected type"):
            b.find(1)


def test_scan_unexpected_page_type_raises(tmp_path: Path) -> None:
    """_scan_page raises CorruptDatabaseError on an unknown page type byte."""
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = 0xFF  # unknown type
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(CorruptDatabaseError, match="unexpected type"):
            list(b.scan())


def test_scan_cycle_in_tree_structure_raises(tmp_path: Path) -> None:
    """_scan_page detects a cycle when an interior page is visited twice."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_split(b)
        # Read the interior root to find the left-child page number.
        root_data = bytearray(p.read(b.root_page))
        hdr = _read_hdr(root_data, 0)
        assert hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        ptr = struct.unpack_from(">H", root_data, _INTERIOR_HDR)[0]
        left_child, _ = _read_interior_cell(root_data, ptr)
        # Corrupt the left child: make it point back to the root as its
        # rightmost child, creating a cycle root → left_child → root.
        child_buf = bytearray(p.read(left_child))
        child_buf[0] = PAGE_TYPE_INTERIOR_TABLE
        # Write a zero-cell interior header with rightmost_child = root page.
        _write_interior_hdr(
            child_buf, 0, ncells=0, content_start=PAGE_SIZE,
            rightmost_child=b.root_page
        )
        p.write(left_child, bytes(child_buf))
        with pytest.raises(CorruptDatabaseError, match="cycle"):
            list(b.scan())
        _ = last  # used to determine tree shape


def test_cell_count_unexpected_page_type_raises(tmp_path: Path) -> None:
    """cell_count raises CorruptDatabaseError on an unknown page type byte."""
    with make_pager(tmp_path) as p:
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        buf[0] = 0xFF  # unknown type
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno)
        with pytest.raises(CorruptDatabaseError, match="unexpected type"):
            b.cell_count()


def test_free_space_on_interior_root(tmp_path: Path) -> None:
    """free_space() returns a sensible non-negative value after a root split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        _fill_until_split(b)
        # After split, root is interior. free_space should be positive.
        fs = b.free_space()
        assert fs >= 0


def test_root_split_with_header_offset_100(tmp_path: Path) -> None:
    """Root split on page 1 (header_offset=100) preserves the database header prefix.

    With header_offset=100 the B-tree page-type byte lives at page byte 100,
    not byte 0.  This test exercises the ``hdr_off > 0`` branches of both
    :meth:`_split_root_leaf` and :meth:`_write_cells_to_leaf`.
    """
    with make_pager(tmp_path) as p:
        # Simulate page-1 layout: first 100 bytes are a "database header".
        pgno = p.allocate()
        buf = bytearray(p.read(pgno))
        # Write a distinctive sentinel at bytes 0-99 so we can verify it
        # survives the root-split rewrite.
        buf[:100] = bytes([0xAB] * 100)
        _init_leaf_page(buf, 100)
        p.write(pgno, bytes(buf))
        b = BTree.open(p, pgno, header_offset=100)
        # Fill until root splits.  NOTE: with header_offset=100 the page-type
        # byte is at offset 100 in the raw page, not offset 0.
        rowid = 1
        while True:
            b.insert(rowid, record_encode([rowid]))
            root_data = p.read(b.root_page)
            if root_data[100] == PAGE_TYPE_INTERIOR_TABLE:
                break
            rowid += 1
        # The database-header prefix must still be intact after the rewrite.
        root_data = bytearray(p.read(b.root_page))
        assert root_data[:100] == bytes([0xAB] * 100)
        # All inserted rows must be findable through the split tree.
        for rid in range(1, rowid + 1):
            assert b.find(rid) is not None


# ==================================================================
# Phase 4b — non-root leaf splits and interior node splits
# ==================================================================


# ------------------------------------------------------------------
# Helper: insert until a non-root leaf split has happened
# (i.e. root interior page has at least 2 cells)
# ------------------------------------------------------------------


def _fill_until_non_root_split(b: BTree, start: int = 1) -> int:
    """Insert small rows until the root interior page has ≥ 2 separator cells.

    Returns the rowid of the last inserted row.  At that point at least
    one non-root leaf split has propagated a separator up to the root.
    """
    rowid = start
    while True:
        b.insert(rowid, record_encode([rowid]))
        root_data = b._pager.read(b.root_page)
        if root_data[0] == PAGE_TYPE_INTERIOR_TABLE:
            hdr = _read_hdr(root_data, 0)
            if hdr["ncells"] >= 2:
                return rowid
        rowid += 1


# ------------------------------------------------------------------
# Non-root leaf split — basic correctness
# ------------------------------------------------------------------


def test_non_root_leaf_split_triggers(tmp_path: Path) -> None:
    """Inserting enough rows must cause the root to accumulate multiple cells."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        last = _fill_until_non_root_split(b)
        root_data = p.read(b.root_page)
        hdr = _read_hdr(root_data, 0)
        assert hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        assert hdr["ncells"] >= 2
        assert b.cell_count() == last


def test_non_root_leaf_split_cell_count(tmp_path: Path) -> None:
    """cell_count() must equal the number of inserted rows after multiple splits."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 1000
        for rowid in range(1, n + 1):
            b.insert(rowid, record_encode([rowid]))
        assert b.cell_count() == n


def test_non_root_leaf_split_scan_all(tmp_path: Path) -> None:
    """scan() must yield every row in ascending rowid order after many splits."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 1000
        for rowid in range(1, n + 1):
            b.insert(rowid, record_encode([rowid]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, n + 1))
        for rowid, raw in rows:
            values, _ = record_decode(raw)
            assert values == [rowid]


def test_non_root_leaf_split_find_all(tmp_path: Path) -> None:
    """find() must succeed for every rowid after non-root leaf splits."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 800
        for rowid in range(1, n + 1):
            b.insert(rowid, record_encode([rowid]))
        for rowid in range(1, n + 1):
            raw = b.find(rowid)
            assert raw is not None
            values, _ = record_decode(raw)
            assert values == [rowid]


def test_non_root_leaf_split_find_missing(tmp_path: Path) -> None:
    """find() returns None for a rowid not in a multi-split tree."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in range(1, 601):
            b.insert(rowid, record_encode([rowid]))
        assert b.find(99_999) is None


def test_non_root_leaf_split_delete(tmp_path: Path) -> None:
    """delete() removes the correct row from a tree that has undergone multiple splits."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 800
        for rowid in range(1, n + 1):
            b.insert(rowid, record_encode([rowid]))
        # Delete every even rowid.
        for rowid in range(2, n + 1, 2):
            assert b.delete(rowid) is True
        assert b.cell_count() == n // 2
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, n + 1, 2))


def test_non_root_leaf_split_update(tmp_path: Path) -> None:
    """update() replaces payloads correctly in a multi-split tree."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 600
        for rowid in range(1, n + 1):
            b.insert(rowid, record_encode([rowid]))
        # Update a handful of rows on different leaves.
        for rowid in [1, 100, 300, n]:
            assert b.update(rowid, record_encode([rowid * 10])) is True
        for rowid in [1, 100, 300, n]:
            values, _ = record_decode(b.find(rowid))  # type: ignore[arg-type]
            assert values == [rowid * 10]


def test_non_root_leaf_split_duplicate_rowid(tmp_path: Path) -> None:
    """DuplicateRowidError is raised even when the tree has multiple leaves."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in range(1, 601):
            b.insert(rowid, record_encode([rowid]))
        with pytest.raises(DuplicateRowidError):
            b.insert(300, record_encode(["dup"]))


def test_non_root_leaf_split_reverse_order(tmp_path: Path) -> None:
    """Descending-order inserts still produce a correctly-structured tree."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 800
        for rowid in range(n, 0, -1):
            b.insert(rowid, record_encode([rowid]))
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, n + 1))


def test_non_root_leaf_split_persist_reopen(tmp_path: Path) -> None:
    """Multi-split tree committed to disk must round-trip through close/reopen."""
    p = make_pager(tmp_path)
    b = BTree.create(p)
    n = 600
    for rowid in range(1, n + 1):
        b.insert(rowid, record_encode([f"row{rowid}"]))
    p.commit()
    p.close()

    p2 = Pager.open(tmp_path / "db")
    b2 = BTree.open(p2, 1)
    assert b2.cell_count() == n
    rows = list(b2.scan())
    assert [r for r, _ in rows] == list(range(1, n + 1))
    for rowid, raw in rows:
        values, _ = record_decode(raw)
        assert values == [f"row{rowid}"]
    p2.close()


# ------------------------------------------------------------------
# _interior_cells_fit — unit tests
# ------------------------------------------------------------------


def test_interior_cells_fit_empty_list() -> None:
    """An empty cell list always fits."""
    assert _interior_cells_fit(0, []) is True


def test_interior_cells_fit_small_list() -> None:
    """A handful of small cells fit on a fresh interior page."""
    cells = [(i, i * 100) for i in range(1, 11)]
    assert _interior_cells_fit(0, cells) is True


def test_interior_cells_fit_overfull() -> None:
    """A cell list that exceeds PAGE_SIZE must not fit."""
    # Each interior cell for small rowids is 4 + 1 = 5 bytes, plus 2 ptr = 7.
    # Available space = 4096 - 12 = 4084. Forcing 600 cells = 4200 > 4084.
    cells = [(i, i) for i in range(1, 601)]
    assert _interior_cells_fit(0, cells) is False


def test_interior_cells_fit_with_header_offset() -> None:
    """header_offset=100 reduces available space by 100 bytes.

    For cells = [(i, i) for i in range(1, n+1)] with small i values:
    - hdr_off=0  : avail = 4084 bytes, max cells = 526
    - hdr_off=100: avail = 3984 bytes, max cells = 513

    Using 520 cells sits between the two thresholds: fits at 0, overflows at 100.
    """
    cells_520 = [(i, i) for i in range(1, 521)]
    # Fits at hdr_off=0 (needs ~4033 bytes ≤ 4084 available).
    assert _interior_cells_fit(0, cells_520) is True
    # Does not fit at hdr_off=100 (needs ~4033 bytes > 3984 available).
    assert _interior_cells_fit(100, cells_520) is False
    # A shorter list fits at both offsets.
    cells_short = [(i, i) for i in range(1, 200)]
    assert _interior_cells_fit(0, cells_short) is True
    assert _interior_cells_fit(100, cells_short) is True


# ------------------------------------------------------------------
# _write_interior_page — unit tests
# ------------------------------------------------------------------


def test_write_interior_page_round_trip(tmp_path: Path) -> None:
    """_write_interior_page produces a page that _read_hdr + _read_interior_ptrs
    can decode correctly."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        extra = p.allocate()
        cells = [(2, 10), (3, 20), (4, 30)]
        b._write_interior_page(extra, 0, cells, rightmost_child=5)
        page_data = p.read(extra)
        hdr = _read_hdr(page_data, 0)
        assert hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        assert hdr["ncells"] == 3
        assert hdr["rightmost_child"] == 5
        ptrs = _read_interior_ptrs(page_data, 0, 3)
        decoded = [_read_interior_cell(page_data, ptr) for ptr in ptrs]
        assert decoded == cells


def test_write_interior_page_empty(tmp_path: Path) -> None:
    """_write_interior_page with no cells writes a valid header."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        extra = p.allocate()
        b._write_interior_page(extra, 0, [], rightmost_child=7)
        page_data = p.read(extra)
        hdr = _read_hdr(page_data, 0)
        assert hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        assert hdr["ncells"] == 0
        assert hdr["rightmost_child"] == 7


# ------------------------------------------------------------------
# Interior node split — triggered end-to-end with large payloads
#
# With ~800-byte payloads each leaf holds ~5 cells.  Non-root splits
# propagate a separator to the root interior page each time a leaf
# fills.  After ~510 such splits the root interior page fills and
# _split_root_interior is invoked.  Inserting ~1800 rows reliably
# triggers that code path.
# ------------------------------------------------------------------


# A payload large enough to make each leaf hold only ~5 cells so that
# the root interior page fills up within a reasonable row count.
_LARGE_PAYLOAD = b"X" * 800


def test_interior_split_cell_count(tmp_path: Path) -> None:
    """cell_count() is exact after a root interior split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 1800
        for rowid in range(1, n + 1):
            b.insert(rowid, _LARGE_PAYLOAD)
        assert b.cell_count() == n


def test_interior_split_scan_all(tmp_path: Path) -> None:
    """scan() returns all rows in ascending order after an interior split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 1800
        for rowid in range(1, n + 1):
            b.insert(rowid, _LARGE_PAYLOAD)
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, n + 1))
        for _, payload in rows:
            assert payload == _LARGE_PAYLOAD


def test_interior_split_find_all(tmp_path: Path) -> None:
    """find() works for every rowid after an interior split."""
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        n = 1800
        for rowid in range(1, n + 1):
            b.insert(rowid, _LARGE_PAYLOAD)
        for rowid in range(1, n + 1):
            assert b.find(rowid) == _LARGE_PAYLOAD, f"find({rowid}) failed"


def test_interior_split_root_has_depth_three(tmp_path: Path) -> None:
    """After a root interior split the tree must have depth ≥ 3.

    When the root interior page splits, it gains two interior children,
    making depth = root (interior) + interior child + leaf = 3.
    The root should have exactly one separator cell after its split.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        for rowid in range(1, 1801):
            b.insert(rowid, _LARGE_PAYLOAD)
        root_data = p.read(b.root_page)
        root_hdr = _read_hdr(root_data, 0)
        assert root_hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        # After root interior split the root has exactly 1 cell.
        assert root_hdr["ncells"] == 1
        # Both children must also be interior pages.
        ptrs = _read_interior_ptrs(root_data, 0, 1)
        left_child, _ = _read_interior_cell(root_data, ptrs[0])
        right_child = root_hdr["rightmost_child"]
        left_data = p.read(left_child)
        right_data = p.read(right_child)
        assert _read_hdr(left_data, 0)["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        assert _read_hdr(right_data, 0)["page_type"] == PAGE_TYPE_INTERIOR_TABLE


def test_interior_split_persist_reopen(tmp_path: Path) -> None:
    """Tree with interior splits round-trips through commit + reopen."""
    p = make_pager(tmp_path)
    b = BTree.create(p)
    n = 1800
    for rowid in range(1, n + 1):
        b.insert(rowid, _LARGE_PAYLOAD)
    p.commit()
    p.close()

    p2 = Pager.open(tmp_path / "db")
    b2 = BTree.open(p2, 1)
    assert b2.cell_count() == n
    rows = list(b2.scan())
    assert [r for r, _ in rows] == list(range(1, n + 1))
    p2.close()


# ------------------------------------------------------------------
# _split_root_interior — direct unit test
# ------------------------------------------------------------------


def test_split_root_interior_direct(tmp_path: Path) -> None:
    """Call _split_root_interior directly with a synthetic full-root cell list.

    We allocate placeholder pages for the children and verify that the
    resulting root has exactly 1 cell and two interior children.
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        # Allocate 512 placeholder child pages.
        child_pages = [p.allocate() for _ in range(513)]
        # Build 512 synthetic cells: [(child_pages[0], 0), ..., (child_pages[511], 511)]
        cells_512 = [(child_pages[i], i * 100) for i in range(512)]
        all_rightmost = child_pages[512]
        # Call _split_root_interior with the synthetic cell list.
        b._split_root_interior(cells_512, all_rightmost)
        p.commit()

    with Pager.open(tmp_path / "db") as p2:
        root_data = p2.read(1)
        root_hdr = _read_hdr(root_data, 0)
        assert root_hdr["page_type"] == PAGE_TYPE_INTERIOR_TABLE
        # After splitting 512 cells the root should have 1 cell (the median).
        assert root_hdr["ncells"] == 1
        # Both children should be valid (non-zero) interior pages.
        ptr = struct.unpack_from(">H", root_data, _INTERIOR_HDR)[0]
        left_child, median_sep = _read_interior_cell(root_data, ptr)
        right_child = root_hdr["rightmost_child"]
        assert left_child != 0
        assert right_child != 0
        assert left_child != right_child
        # Median sep must be between 0 and 51100 (the range of our synthetic cells).
        assert 0 <= median_sep <= 51100


# ------------------------------------------------------------------
# Push-separator-up with full parent — exercises _split_interior_page
# ------------------------------------------------------------------


def test_push_separator_up_full_parent(tmp_path: Path) -> None:
    """When _push_separator_up encounters a full parent, the parent is split.

    We craft this scenario by:
    1. Performing enough inserts with large payloads to trigger a non-root
       leaf split (parent = root interior, has 1+ cells).
    2. Then filling the root interior completely by inserting many more rows.
    3. The next non-root leaf split must push a separator into the full root,
       triggering _split_root_interior.

    After this the tree must be fully navigable (scan all, find all).
    """
    with make_pager(tmp_path) as p:
        b = BTree.create(p)
        # Insert enough large-payload rows to guarantee at least one root
        # interior split (same end-to-end as the interior split tests above).
        n = 2000
        for rowid in range(1, n + 1):
            b.insert(rowid, _LARGE_PAYLOAD)
        assert b.cell_count() == n
        rows = list(b.scan())
        assert [r for r, _ in rows] == list(range(1, n + 1))
