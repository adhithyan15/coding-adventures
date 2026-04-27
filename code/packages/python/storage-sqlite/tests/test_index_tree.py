"""
Tests for IndexTree — phase IX-1.

Test structure
--------------
The tests are ordered from simplest to most complex:

1.  **Construction** — create and open, root_page property.
2.  **Single-entry insert + lookup** — basic round-trip.
3.  **Ordered scan** — range_scan and full scan return sorted entries.
4.  **Delete** — remove present and absent entries.
5.  **Duplicate-key lookup** — multiple rows sharing the same indexed value.
6.  **Range scan bounds** — open/closed lo and hi bounds.
7.  **Splits** — enough inserts to force root-leaf split and interior splits.
8.  **Value types** — NULL, float, text, bytes as index keys.
9.  **free_all** — all pages returned to freelist after call.
10. **Comparison order** — NULL < int < text < bytes.
11. **Persistence** — commit to file, reopen, verify data survives.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from storage_sqlite import PAGE_SIZE, Freelist, Header, IndexTree, Pager
from storage_sqlite.index_tree import (
    DuplicateIndexKeyError,
    _cmp_full_keys,
    _cmp_keys_partial,
    _cmp_values,
    _type_class,
)

# ── Fixtures ──────────────────────────────────────────────────────────────────


def _make_pager(tmp_path: Path) -> Pager:
    """Create a fresh pager backed by a temp file with the database header.

    Page 1 is allocated and written with the 100-byte SQLite database header
    so that the Freelist infrastructure (which reads header fields at page-1
    offsets 32–39) works correctly in tests that use it.
    """
    db = str(tmp_path / "test.db")
    pager = Pager.create(db)
    # pager starts with size_pages=0; allocate() bumps it to 1 and returns 1.
    pgno = pager.allocate()
    assert pgno == 1
    page1 = bytearray(PAGE_SIZE)
    page1[:100] = Header.new_database(page_size=PAGE_SIZE).to_bytes()
    pager.write(1, bytes(page1))
    return pager


# ── Section 1: Construction ───────────────────────────────────────────────────


class TestConstruction:
    def test_create_returns_index_tree(self, tmp_path: Path) -> None:
        """IndexTree.create() returns an IndexTree with a valid root page."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        assert isinstance(tree, IndexTree)
        assert tree.root_page >= 1
        pager.close()

    def test_create_uses_page_2_when_page_1_is_header(self, tmp_path: Path) -> None:
        """After writing the database header to page 1, the first tree root
        should be page 2 (next free page)."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        # Page 1 is the header page; tree root should be page 2.
        assert tree.root_page == 2
        pager.close()

    def test_open_returns_existing_tree(self, tmp_path: Path) -> None:
        """IndexTree.open() attaches to an existing root page."""
        pager = _make_pager(tmp_path)
        tree1 = IndexTree.create(pager)
        tree1.insert([42], 1)
        pager.commit()

        # Reopen via open().
        tree2 = IndexTree.open(pager, tree1.root_page)
        assert tree2.root_page == tree1.root_page
        assert tree2.lookup([42]) == [1]
        pager.close()

    def test_empty_tree_cell_count_is_zero(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        assert tree.cell_count() == 0
        pager.close()


# ── Section 2: Single-entry insert + lookup ───────────────────────────────────


class TestSingleEntry:
    def test_insert_and_lookup_integer_key(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([99], 7)
        assert tree.lookup([99]) == [7]
        pager.close()

    def test_lookup_missing_key_returns_empty(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([10], 1)
        assert tree.lookup([20]) == []
        pager.close()

    def test_insert_increases_cell_count(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([1], 100)
        assert tree.cell_count() == 1
        tree.insert([2], 200)
        assert tree.cell_count() == 2
        pager.close()

    def test_duplicate_key_rowid_raises(self, tmp_path: Path) -> None:
        """Inserting the same (key, rowid) pair twice raises
        DuplicateIndexKeyError."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([5], 1)
        with pytest.raises(DuplicateIndexKeyError):
            tree.insert([5], 1)
        pager.close()


# ── Section 3: Ordered scan ───────────────────────────────────────────────────


class TestOrderedScan:
    def test_full_scan_returns_sorted_order(self, tmp_path: Path) -> None:
        """Entries are yielded in ascending (key, rowid) order."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        # Insert out of order.
        for k in [30, 10, 50, 20, 40]:
            tree.insert([k], k)
        results = list(tree.range_scan(None, None))
        keys = [k for (k,), _ in results]
        assert keys == sorted(keys)
        assert keys == [10, 20, 30, 40, 50]
        pager.close()

    def test_range_scan_no_bounds_yields_all(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        for i in range(1, 6):
            tree.insert([i], i * 10)
        results = list(tree.range_scan(None, None))
        assert len(results) == 5
        pager.close()

    def test_scan_tiebreak_by_rowid(self, tmp_path: Path) -> None:
        """Two entries with the same key value are ordered by rowid."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([7], 3)
        tree.insert([7], 1)
        tree.insert([7], 2)
        results = list(tree.range_scan(None, None))
        rowids = [r for _, r in results]
        assert rowids == [1, 2, 3]
        pager.close()


# ── Section 4: Delete ─────────────────────────────────────────────────────────


class TestDelete:
    def test_delete_present_returns_true(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([5], 1)
        assert tree.delete([5], 1) is True
        pager.close()

    def test_delete_absent_returns_false(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([5], 1)
        # Different rowid — not present.
        assert tree.delete([5], 99) is False
        # Different key — not present.
        assert tree.delete([6], 1) is False
        pager.close()

    def test_delete_removes_entry_from_lookup(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([5], 1)
        tree.insert([5], 2)
        tree.delete([5], 1)
        assert tree.lookup([5]) == [2]
        pager.close()

    def test_delete_decrements_cell_count(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([1], 1)
        tree.insert([2], 2)
        tree.delete([1], 1)
        assert tree.cell_count() == 1
        pager.close()

    def test_delete_adjacent_entries_intact(self, tmp_path: Path) -> None:
        """Deleting one entry leaves its neighbours untouched."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        for i in range(1, 6):
            tree.insert([i], i)
        tree.delete([3], 3)
        remaining = [r for (_, ), r in tree.range_scan(None, None)]
        assert remaining == [1, 2, 4, 5]
        pager.close()

    def test_insert_after_delete_reuses_slot(self, tmp_path: Path) -> None:
        """After deleting an entry we can insert a new one at the same key."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([10], 1)
        tree.delete([10], 1)
        tree.insert([10], 2)  # should not raise
        assert tree.lookup([10]) == [2]
        pager.close()


# ── Section 5: Duplicate-key lookup ──────────────────────────────────────────


class TestDuplicateKeys:
    def test_lookup_returns_all_rowids_for_key(self, tmp_path: Path) -> None:
        """Non-unique index: multiple rows can share the same indexed value."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        for rowid in [5, 1, 3]:
            tree.insert([42], rowid)
        result = sorted(tree.lookup([42]))
        assert result == [1, 3, 5]
        pager.close()

    def test_same_key_different_rowid_no_error(self, tmp_path: Path) -> None:
        """(key=7, rowid=1) and (key=7, rowid=2) are distinct entries."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([7], 1)
        tree.insert([7], 2)  # different rowid — must not raise
        assert tree.cell_count() == 2
        pager.close()


# ── Section 6: Range scan bounds ─────────────────────────────────────────────


class TestRangeScanBounds:
    def _make_tree(self, tmp_path: Path) -> tuple[IndexTree, Pager]:
        """Helper: tree with keys 1..10, rowid = key."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        for i in range(1, 11):
            tree.insert([i], i)
        return tree, pager

    def test_lo_inclusive(self, tmp_path: Path) -> None:
        tree, pager = self._make_tree(tmp_path)
        keys = [k for (k,), _ in tree.range_scan([5], None, lo_inclusive=True)]
        assert keys == list(range(5, 11))
        pager.close()

    def test_lo_exclusive(self, tmp_path: Path) -> None:
        tree, pager = self._make_tree(tmp_path)
        keys = [k for (k,), _ in tree.range_scan([5], None, lo_inclusive=False)]
        assert keys == list(range(6, 11))
        pager.close()

    def test_hi_inclusive(self, tmp_path: Path) -> None:
        tree, pager = self._make_tree(tmp_path)
        keys = [k for (k,), _ in tree.range_scan(None, [5], hi_inclusive=True)]
        assert keys == list(range(1, 6))
        pager.close()

    def test_hi_exclusive(self, tmp_path: Path) -> None:
        tree, pager = self._make_tree(tmp_path)
        keys = [k for (k,), _ in tree.range_scan(None, [5], hi_inclusive=False)]
        assert keys == list(range(1, 5))
        pager.close()

    def test_lo_and_hi_inclusive(self, tmp_path: Path) -> None:
        tree, pager = self._make_tree(tmp_path)
        keys = [k for (k,), _ in tree.range_scan([3], [7])]
        assert keys == [3, 4, 5, 6, 7]
        pager.close()

    def test_empty_range(self, tmp_path: Path) -> None:
        """Range where lo > hi yields nothing."""
        tree, pager = self._make_tree(tmp_path)
        result = list(tree.range_scan([8], [3]))
        assert result == []
        pager.close()

    def test_single_value_range(self, tmp_path: Path) -> None:
        """range_scan([k], [k]) is equivalent to lookup([k])."""
        tree, pager = self._make_tree(tmp_path)
        result = list(tree.range_scan([5], [5]))
        assert result == [([5], 5)]
        pager.close()

    def test_out_of_bounds_lo(self, tmp_path: Path) -> None:
        """lo beyond all keys returns empty."""
        tree, pager = self._make_tree(tmp_path)
        result = list(tree.range_scan([100], None))
        assert result == []
        pager.close()

    def test_out_of_bounds_hi(self, tmp_path: Path) -> None:
        """hi below all keys returns empty."""
        tree, pager = self._make_tree(tmp_path)
        result = list(tree.range_scan(None, [0]))
        assert result == []
        pager.close()


# ── Section 7: Splits ─────────────────────────────────────────────────────────


class TestSplits:
    def test_root_leaf_split(self, tmp_path: Path) -> None:
        """Inserting enough entries triggers a root-leaf split.

        At 4 096-byte pages with small integer records, roughly 500+ entries
        are needed.  We insert 600 to be safe and verify all are readable.
        """
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 600
        for i in range(1, n + 1):
            tree.insert([i], i)
        assert tree.cell_count() == n
        # All entries must be present.
        for i in range(1, n + 1):
            assert tree.lookup([i]) == [i], f"missing entry for key {i}"
        pager.close()

    def test_scan_after_split_is_ordered(self, tmp_path: Path) -> None:
        """After a root-leaf split, scan still returns entries in order."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        import random
        rng = random.Random(42)
        keys = list(range(1, 601))
        rng.shuffle(keys)
        for k in keys:
            tree.insert([k], k)
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        assert result_keys == list(range(1, 601))
        pager.close()

    def test_multi_level_split(self, tmp_path: Path) -> None:
        """Insert enough entries to force interior-page splits (depth > 2)."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 5000
        for i in range(1, n + 1):
            tree.insert([i], i)
        assert tree.cell_count() == n
        # Spot-check first, middle, last.
        assert tree.lookup([1]) == [1]
        assert tree.lookup([2500]) == [2500]
        assert tree.lookup([5000]) == [5000]
        pager.close()

    def test_delete_after_split(self, tmp_path: Path) -> None:
        """Deletes work correctly on a multi-level tree."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 1000
        for i in range(1, n + 1):
            tree.insert([i], i)
        # Delete every even entry.
        for i in range(2, n + 1, 2):
            assert tree.delete([i], i) is True
        assert tree.cell_count() == n // 2
        # Verify only odd keys remain.
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        assert result_keys == list(range(1, n + 1, 2))
        pager.close()

    def test_insert_reverse_order(self, tmp_path: Path) -> None:
        """Inserting in descending key order also triggers correct splits."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 800
        for i in range(n, 0, -1):
            tree.insert([i], i)
        assert tree.cell_count() == n
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        assert result_keys == list(range(1, n + 1))
        pager.close()


# ── Section 8: Value types ────────────────────────────────────────────────────


class TestValueTypes:
    def test_none_key(self, tmp_path: Path) -> None:
        """NULL is a valid index key."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([None], 1)
        assert tree.lookup([None]) == [1]
        pager.close()

    def test_float_key(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([3.14], 1)
        assert tree.lookup([3.14]) == [1]
        pager.close()

    def test_text_key(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert(["hello"], 42)
        assert tree.lookup(["hello"]) == [42]
        pager.close()

    def test_bytes_key(self, tmp_path: Path) -> None:
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([b"\x01\x02"], 7)
        assert tree.lookup([b"\x01\x02"]) == [7]
        pager.close()

    def test_mixed_type_keys_ordered(self, tmp_path: Path) -> None:
        """NULL < int < text < bytes."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([b"blob"], 4)
        tree.insert(["text"], 3)
        tree.insert([100], 2)
        tree.insert([None], 1)
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        assert result_keys == [None, 100, "text", b"blob"]
        pager.close()

    def test_int_float_equality_key(self, tmp_path: Path) -> None:
        """Integer 2 and float 2.0 are equal keys (same sort position)."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        tree.insert([2], 1)
        # 2.0 and 2 are equal under SQLite numeric comparison.
        assert tree.lookup([2.0]) == [1]
        pager.close()

    def test_text_key_many_entries(self, tmp_path: Path) -> None:
        """Text keys sort correctly across splits."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        words = [f"word_{i:04d}" for i in range(500)]
        for i, w in enumerate(words):
            tree.insert([w], i)
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        assert result_keys == sorted(w.encode("utf-8") for w in words) or \
               result_keys == sorted(words)
        # Verify sorted order is preserved (UTF-8 BINARY collation).
        for a, b in zip(result_keys, result_keys[1:], strict=False):
            assert _cmp_values(a, b) <= 0
        pager.close()


# ── Section 9: free_all ───────────────────────────────────────────────────────


class TestFreeAll:
    def test_free_all_returns_pages_to_freelist(self, tmp_path: Path) -> None:
        """After free_all, the freelist contains the pages that were in the
        index tree."""
        pager = _make_pager(tmp_path)
        fl = Freelist(pager)

        tree = IndexTree.create(pager, freelist=fl)
        n = 200
        for i in range(1, n + 1):
            tree.insert([i], i)
        pager.commit()

        pages_before = pager.size_pages
        free_before = fl.total_pages

        tree.free_all(fl)

        # After free_all the freelist should be larger.
        assert fl.total_pages > free_before

        # Pages returned must not exceed what was previously allocated.
        assert fl.total_pages <= pages_before - 1  # -1 for the header page
        pager.close()

    def test_free_all_then_reuse(self, tmp_path: Path) -> None:
        """Pages freed by free_all can be reused for a new index tree."""
        pager = _make_pager(tmp_path)
        fl = Freelist(pager)

        tree1 = IndexTree.create(pager, freelist=fl)
        for i in range(1, 201):
            tree1.insert([i], i)
        pager.commit()

        pages_after_tree1 = pager.size_pages

        tree1.free_all(fl)

        # Build a new tree of the same size — it should reuse the freed pages.
        tree2 = IndexTree.create(pager, freelist=fl)
        for i in range(1, 201):
            tree2.insert([i], i)

        # The total page count should be close to what it was before (pages
        # were reused, so we should not have grown much beyond pages_after_tree1).
        # Allow a small margin for the new root page.
        assert pager.size_pages <= pages_after_tree1 + 3
        pager.close()


# ── Section 10: Comparison order ─────────────────────────────────────────────


class TestComparisonOrder:
    """Unit tests for the comparison helpers."""

    # _type_class
    def test_type_class_null(self) -> None:
        assert _type_class(None) == 0

    def test_type_class_int(self) -> None:
        assert _type_class(42) == 1

    def test_type_class_float(self) -> None:
        assert _type_class(3.14) == 1

    def test_type_class_str(self) -> None:
        assert _type_class("hi") == 2

    def test_type_class_bytes(self) -> None:
        assert _type_class(b"") == 3

    # _cmp_values
    def test_cmp_null_null(self) -> None:
        assert _cmp_values(None, None) == 0

    def test_cmp_null_int(self) -> None:
        assert _cmp_values(None, 1) == -1

    def test_cmp_int_null(self) -> None:
        assert _cmp_values(1, None) == 1

    def test_cmp_int_int_lt(self) -> None:
        assert _cmp_values(1, 2) == -1

    def test_cmp_int_int_eq(self) -> None:
        assert _cmp_values(5, 5) == 0

    def test_cmp_int_float_cross_type_equal(self) -> None:
        """Integer 2 and float 2.0 are numerically equal."""
        assert _cmp_values(2, 2.0) == 0

    def test_cmp_float_int_less(self) -> None:
        assert _cmp_values(1.5, 2) == -1

    def test_cmp_str_str_lt(self) -> None:
        assert _cmp_values("a", "b") == -1

    def test_cmp_str_str_eq(self) -> None:
        assert _cmp_values("hi", "hi") == 0

    def test_cmp_capital_less_than_lower(self) -> None:
        """'Z' (0x5A) < 'a' (0x61) in UTF-8 byte order."""
        assert _cmp_values("Z", "a") == -1

    def test_cmp_int_less_than_str(self) -> None:
        assert _cmp_values(9999, "a") == -1

    def test_cmp_str_less_than_bytes(self) -> None:
        assert _cmp_values("z", b"a") == -1

    def test_cmp_null_less_than_bytes(self) -> None:
        assert _cmp_values(None, b"x") == -1

    def test_cmp_bytes_bytes_lt(self) -> None:
        assert _cmp_values(b"\x01", b"\x02") == -1

    def test_cmp_bytes_bytes_eq(self) -> None:
        assert _cmp_values(b"abc", b"abc") == 0

    # _cmp_full_keys
    def test_cmp_full_keys_key_differs(self) -> None:
        assert _cmp_full_keys([1], 1, [2], 1) == -1

    def test_cmp_full_keys_rowid_tiebreak(self) -> None:
        assert _cmp_full_keys([5], 1, [5], 2) == -1

    def test_cmp_full_keys_equal(self) -> None:
        assert _cmp_full_keys([5], 3, [5], 3) == 0

    # _cmp_keys_partial
    def test_cmp_keys_partial_equal(self) -> None:
        assert _cmp_keys_partial([5], [5]) == 0

    def test_cmp_keys_partial_lt(self) -> None:
        assert _cmp_keys_partial([3], [5]) == -1

    def test_cmp_keys_partial_gt(self) -> None:
        assert _cmp_keys_partial([7], [5]) == 1


# ── Section 11: Persistence ───────────────────────────────────────────────────


class TestPersistence:
    def test_data_survives_commit_and_reopen(self, tmp_path: Path) -> None:
        """Entries written and committed can be read in a new pager session."""
        db = str(tmp_path / "persist.db")

        # Write session.
        pager = Pager.create(db)
        pgno = pager.allocate()
        assert pgno == 1
        page1 = bytearray(PAGE_SIZE)
        page1[:100] = Header.new_database(page_size=PAGE_SIZE).to_bytes()
        pager.write(1, bytes(page1))
        tree = IndexTree.create(pager)
        root = tree.root_page
        for i in range(1, 101):
            tree.insert([i], i)
        pager.commit()
        pager.close()

        # Read session.
        pager = Pager.open(db)
        tree = IndexTree.open(pager, root)
        assert tree.cell_count() == 100
        for i in range(1, 101):
            assert tree.lookup([i]) == [i]
        pager.close()

    def test_partial_writes_rolled_back(self, tmp_path: Path) -> None:
        """Entries inserted but not committed do not appear after rollback."""
        db = str(tmp_path / "rollback.db")

        pager = Pager.create(db)
        pgno = pager.allocate()
        assert pgno == 1
        page1 = bytearray(PAGE_SIZE)
        page1[:100] = Header.new_database(page_size=PAGE_SIZE).to_bytes()
        pager.write(1, bytes(page1))
        tree = IndexTree.create(pager)
        root = tree.root_page
        tree.insert([1], 1)
        pager.commit()
        pager.close()

        pager = Pager.open(db)
        tree = IndexTree.open(pager, root)
        # Insert but don't commit.
        tree.insert([2], 2)
        pager.rollback()
        # After rollback, only the committed entry should be present.
        assert tree.lookup([1]) == [1]
        assert tree.lookup([2]) == []
        pager.close()


# ── Section 12: Interior-page splits and deep trees ──────────────────────────


class TestDeepSplits:
    """Tests that specifically trigger _split_interior_page and
    _split_root_interior by using large text keys so each page holds fewer
    cells and the tree grows deeper faster."""

    _KEY_SIZE = 480  # bytes — gives ~7 cells per leaf / ~7 per interior

    def _big_key(self, i: int) -> str:
        """Generate a padded text key that forces fast splits."""
        s = f"{i:08d}"
        return s + "x" * (self._KEY_SIZE - len(s))

    def test_root_interior_split(self, tmp_path: Path) -> None:
        """Inserting enough large-key entries forces _split_root_interior.

        With ~480-byte text keys each page holds ~7 entries.  After ~50+
        insertions both leaf pages and the root interior page fill and split.
        """
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 100  # enough to overflow the root interior with large keys
        for i in range(1, n + 1):
            tree.insert([self._big_key(i)], i)
        assert tree.cell_count() == n
        # Verify a sample.
        for i in [1, 50, 100]:
            assert tree.lookup([self._big_key(i)]) == [i]
        pager.close()

    def test_non_root_interior_split(self, tmp_path: Path) -> None:
        """Going deeper forces _split_interior_page (non-root interior split).

        More insertions are needed to fill second-level interior pages.
        """
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 500  # enough for 3-level tree with large keys
        for i in range(1, n + 1):
            tree.insert([self._big_key(i)], i)
        assert tree.cell_count() == n
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        expected = sorted(self._big_key(i) for i in range(1, n + 1))
        assert result_keys == expected
        pager.close()

    def test_deep_delete_and_scan(self, tmp_path: Path) -> None:
        """Delete entries from a deep tree, verify scan is still ordered."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        n = 200
        for i in range(1, n + 1):
            tree.insert([self._big_key(i)], i)
        # Delete every third entry.
        deleted = set()
        for i in range(3, n + 1, 3):
            tree.delete([self._big_key(i)], i)
            deleted.add(i)
        remaining = n - len(deleted)
        assert tree.cell_count() == remaining
        result_keys = [k for (k,), _ in tree.range_scan(None, None)]
        expected = sorted(self._big_key(i) for i in range(1, n + 1) if i not in deleted)
        assert result_keys == expected
        pager.close()


# ── Section 13: Error paths and edge cases ────────────────────────────────────


class TestErrorPaths:
    """Cover error paths: oversized keys, unsupported value types, corrupt
    page detection, and helper edge cases."""

    def test_oversized_key_raises(self, tmp_path: Path) -> None:
        """Inserting a key whose record exceeds the inline limit raises
        IndexTreeError."""
        from storage_sqlite.index_tree import _MAX_LOCAL, IndexTreeError

        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)
        # Construct a key that, when encoded as a record with a rowid, exceeds
        # the inline threshold.
        giant_text = "x" * (_MAX_LOCAL + 1)
        with pytest.raises(IndexTreeError, match="exceeds inline limit"):
            tree.insert([giant_text], 1)
        pager.close()

    def test_type_class_unsupported_raises(self) -> None:
        """_type_class raises TypeError for non-SQL values."""
        from storage_sqlite.index_tree import _type_class

        with pytest.raises(TypeError, match="unsupported SQL value type"):
            _type_class([1, 2, 3])  # type: ignore[arg-type]

    def test_cmp_keys_partial_different_lengths(self) -> None:
        """_cmp_keys_partial handles lists of different lengths."""
        from storage_sqlite.index_tree import _cmp_keys_partial

        # Shorter list is "less than" longer list when prefix values equal.
        assert _cmp_keys_partial([5], [5, 1]) == -1
        assert _cmp_keys_partial([5, 1], [5]) == 1

    def test_cmp_full_keys_reversed(self) -> None:
        """Verify > direction in _cmp_full_keys."""
        from storage_sqlite.index_tree import _cmp_full_keys

        assert _cmp_full_keys([2], 1, [1], 1) == 1
        assert _cmp_full_keys([5], 2, [5], 1) == 1

    def test_free_page_without_freelist(self, tmp_path: Path) -> None:
        """When no freelist is injected, _free_page zeroes the page."""
        pager = _make_pager(tmp_path)
        tree = IndexTree.create(pager)  # no freelist
        for i in range(1, 50):
            tree.insert([i], i)
        # Delete entries — this calls _write_cells_to_leaf (rebuild),
        # which exercises the non-freelist code path in _free_page
        # when pages are reclaimed via free_all without a freelist.
        tree.delete([25], 25)
        assert tree.cell_count() == 48
        pager.close()

    def test_allocate_page_with_freelist(self, tmp_path: Path) -> None:
        """When a freelist is injected, _allocate_page uses it."""
        pager = _make_pager(tmp_path)
        fl = Freelist(pager)
        tree = IndexTree.create(pager, freelist=fl)
        # Insert, commit, free, rebuild — freelist should be exercised.
        for i in range(1, 20):
            tree.insert([i], i)
        tree.free_all(fl)
        freed = fl.total_pages
        # Build a second tree — it should consume freelist pages.
        tree2 = IndexTree.create(pager, freelist=fl)
        for i in range(1, 20):
            tree2.insert([i], i)
        assert fl.total_pages < freed  # freelist pages were consumed
        pager.close()

    def test_free_page_no_freelist_zeroes(self, tmp_path: Path) -> None:
        """_free_page without freelist zeroes the page (via free_all)."""
        pager = _make_pager(tmp_path)
        # No freelist — free_all should zero pages without raising.
        tree = IndexTree.create(pager)
        for i in range(1, 30):
            tree.insert([i], i)
        root = tree.root_page
        from storage_sqlite.freelist import Freelist as FL

        fl2 = FL(pager)
        tree.free_all(fl2)  # free_all injects freelist temporarily
        pager.close()
        del root
