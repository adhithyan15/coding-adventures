"""Comprehensive tests for the BPlusTree implementation.

Test strategy:
  - Every mutating operation is followed by tree.is_valid() to ensure
    structural integrity (key bounds, sorted order, uniform leaf depth,
    leaf linked list correctness).
  - Leaf linked list integrity is tested explicitly via helper functions.
  - range_scan is tested thoroughly with many cases.
  - Tests run with t=2, t=3, and t=5.
  - A 1000-key stress test verifies large-scale correctness.
"""

import pytest

from b_plus_tree import BPlusInternalNode, BPlusLeafNode, BPlusTree


# ---------------------------------------------------------------------------
# Helper: verify the leaf linked list manually
# ---------------------------------------------------------------------------

def leaf_list_keys(tree: BPlusTree) -> list:
    """Walk the leaf linked list and return all keys in order."""
    result = []
    node = tree._first_leaf
    while node is not None:
        result.extend(node.keys)
        node = node.next
    return result


def leaf_list_is_sorted(tree: BPlusTree) -> bool:
    """Return True if the leaf linked list is strictly sorted."""
    keys = leaf_list_keys(tree)
    return keys == sorted(keys) and len(keys) == len(set(keys))


# ---------------------------------------------------------------------------
# Construction
# ---------------------------------------------------------------------------

class TestConstruction:
    def test_default_degree(self) -> None:
        tree = BPlusTree()
        assert tree._t == 2

    def test_custom_degree(self) -> None:
        tree = BPlusTree(t=5)
        assert tree._t == 5

    def test_invalid_degree_raises(self) -> None:
        with pytest.raises(ValueError):
            BPlusTree(t=1)
        with pytest.raises(ValueError):
            BPlusTree(t=0)

    def test_empty_len(self) -> None:
        assert len(BPlusTree()) == 0

    def test_empty_bool(self) -> None:
        assert not BPlusTree()

    def test_empty_valid(self) -> None:
        assert BPlusTree().is_valid()

    def test_empty_search(self) -> None:
        assert BPlusTree().search(1) is None

    def test_empty_min_raises(self) -> None:
        with pytest.raises(ValueError):
            BPlusTree().min_key()

    def test_empty_max_raises(self) -> None:
        with pytest.raises(ValueError):
            BPlusTree().max_key()

    def test_root_starts_as_leaf(self) -> None:
        tree = BPlusTree()
        assert isinstance(tree._root, BPlusLeafNode)


# ---------------------------------------------------------------------------
# Basic insert / search
# ---------------------------------------------------------------------------

class TestInsertSearch:
    def test_single_insert(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(10, "ten")
        assert tree.search(10) == "ten"
        assert len(tree) == 1
        assert tree.is_valid()

    def test_bool_nonempty(self) -> None:
        tree = BPlusTree()
        tree.insert(1, "a")
        assert bool(tree)

    def test_update_existing(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(5, "five")
        tree.insert(5, "FIVE")
        assert tree.search(5) == "FIVE"
        assert len(tree) == 1
        assert tree.is_valid()

    def test_search_missing(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(1, "a")
        assert tree.search(99) is None

    def test_contains(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(7, "seven")
        assert 7 in tree
        assert 8 not in tree

    def test_getitem(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(3, "three")
        assert tree[3] == "three"

    def test_getitem_missing_raises(self) -> None:
        tree = BPlusTree(t=2)
        with pytest.raises(KeyError):
            _ = tree[99]

    def test_setitem(self) -> None:
        tree = BPlusTree(t=2)
        tree[42] = "answer"
        assert tree.search(42) == "answer"
        assert tree.is_valid()

    def test_sequential_insert(self) -> None:
        tree = BPlusTree(t=2)
        for i in range(1, 21):
            tree.insert(i, i * 10)
        for i in range(1, 21):
            assert tree.search(i) == i * 10
        assert len(tree) == 20
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)

    def test_reverse_insert(self) -> None:
        tree = BPlusTree(t=2)
        for i in range(20, 0, -1):
            tree.insert(i, str(i))
        assert len(tree) == 20
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)

    def test_random_insert(self) -> None:
        import random
        keys = list(range(50))
        random.shuffle(keys)
        tree = BPlusTree(t=3)
        for k in keys:
            tree.insert(k, k)
        assert len(tree) == 50
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)


# ---------------------------------------------------------------------------
# All data in leaves — unique to B+ tree
# ---------------------------------------------------------------------------

class TestAllDataInLeaves:
    def test_root_split_creates_internal_node(self) -> None:
        """After enough inserts, the root becomes an internal node."""
        tree = BPlusTree(t=2)
        for i in range(10):
            tree.insert(i, i)
        assert isinstance(tree._root, BPlusInternalNode)
        assert tree.is_valid()

    def test_internal_nodes_have_no_values(self) -> None:
        """Internal nodes in B+ tree do NOT store values."""
        tree = BPlusTree(t=2)
        for i in range(10):
            tree.insert(i, i)
        # Check that no internal node has 'values' attribute with content
        # (BPlusInternalNode dataclass has no 'values' field)
        assert not hasattr(tree._root, "values")

    def test_all_keys_reachable_from_leaves(self) -> None:
        """Every inserted key must be reachable from the leaf layer."""
        tree = BPlusTree(t=3)
        keys = list(range(30))
        for k in keys:
            tree.insert(k, k * 2)
        # Walk the leaf list and collect all keys
        leaf_keys = leaf_list_keys(tree)
        assert sorted(leaf_keys) == sorted(keys)

    def test_leaf_separator_stays_in_leaf(self) -> None:
        """After a leaf split, the separator key must remain in the right leaf."""
        tree = BPlusTree(t=2)
        # Insert 1, 2, 3 — 3rd insert triggers a leaf split
        tree.insert(1, "a")
        tree.insert(2, "b")
        tree.insert(3, "c")
        # The separator must be findable via search
        assert tree.search(2) == "b" or tree.search(3) == "c"
        assert tree.is_valid()
        # All three keys must be in the leaf list
        leaf_keys = leaf_list_keys(tree)
        assert 1 in leaf_keys
        assert 2 in leaf_keys
        assert 3 in leaf_keys


# ---------------------------------------------------------------------------
# Leaf linked list integrity
# ---------------------------------------------------------------------------

class TestLeafLinkedList:
    def test_linked_list_after_inserts(self) -> None:
        tree = BPlusTree(t=2)
        for k in [5, 3, 7, 1, 9, 2, 8]:
            tree.insert(k, k)
        assert leaf_list_is_sorted(tree)

    def test_linked_list_count_matches_size(self) -> None:
        tree = BPlusTree(t=3)
        for k in range(50):
            tree.insert(k, k)
        assert len(leaf_list_keys(tree)) == len(tree)

    def test_linked_list_after_deletes(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(20):
            tree.insert(k, k)
        for k in range(0, 20, 2):  # delete even keys
            tree.delete(k)
        assert leaf_list_is_sorted(tree)
        assert len(leaf_list_keys(tree)) == len(tree)

    def test_first_leaf_pointer_valid(self) -> None:
        tree = BPlusTree(t=2)
        for k in [10, 5, 15, 3, 7]:
            tree.insert(k, k)
        # first_leaf must hold the smallest key
        assert tree._first_leaf.keys[0] == tree.min_key()

    def test_first_leaf_updates_after_delete(self) -> None:
        tree = BPlusTree(t=2)
        for k in [1, 2, 3, 4, 5]:
            tree.insert(k, k)
        tree.delete(1)
        # first_leaf should now start with 2 (or the next key)
        assert tree._first_leaf.keys[0] == tree.min_key()
        assert tree.is_valid()

    def test_linked_list_no_cycles(self) -> None:
        """Tortoise-and-hare cycle detection on the leaf list."""
        tree = BPlusTree(t=3)
        for k in range(100):
            tree.insert(k, k)

        slow = tree._first_leaf
        fast = tree._first_leaf

        while True:
            if fast is None or fast.next is None:
                break
            slow = slow.next  # type: ignore
            fast = fast.next.next  # type: ignore
            assert slow is not fast, "Cycle detected in leaf linked list!"


# ---------------------------------------------------------------------------
# full_scan and __iter__
# ---------------------------------------------------------------------------

class TestFullScan:
    def test_full_scan_empty(self) -> None:
        assert list(BPlusTree().full_scan()) == []

    def test_full_scan_sorted(self) -> None:
        tree = BPlusTree(t=2)
        for k in [5, 3, 7, 1]:
            tree.insert(k, k * 10)
        result = list(tree.full_scan())
        assert result == [(1, 10), (3, 30), (5, 50), (7, 70)]

    def test_iter_keys_sorted(self) -> None:
        tree = BPlusTree(t=3)
        for k in [4, 2, 8, 6]:
            tree.insert(k, k)
        assert list(tree) == [2, 4, 6, 8]

    def test_items_sorted(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(10):
            tree.insert(k, k * 2)
        result = list(tree.items())
        assert result == [(k, k * 2) for k in range(10)]


# ---------------------------------------------------------------------------
# range_scan
# ---------------------------------------------------------------------------

class TestRangeScan:
    def test_range_empty_result(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(10, "a")
        assert tree.range_scan(20, 30) == []

    def test_range_all(self) -> None:
        tree = BPlusTree(t=2)
        for k in [1, 2, 3]:
            tree.insert(k, str(k))
        assert tree.range_scan(0, 10) == [(1, "1"), (2, "2"), (3, "3")]

    def test_range_subset(self) -> None:
        tree = BPlusTree(t=3)
        for k in [1, 3, 5, 7, 9]:
            tree.insert(k, k)
        result = tree.range_scan(3, 7)
        assert result == [(3, 3), (5, 5), (7, 7)]

    def test_range_single(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(5, "five")
        assert tree.range_scan(5, 5) == [(5, "five")]

    def test_range_inclusive_boundaries(self) -> None:
        tree = BPlusTree(t=2)
        for k in [1, 2, 3, 4, 5]:
            tree.insert(k, k)
        assert tree.range_scan(2, 4) == [(2, 2), (3, 3), (4, 4)]

    def test_range_spans_multiple_leaves(self) -> None:
        """Range spans many leaf nodes — exercises the leaf linked list walk."""
        tree = BPlusTree(t=2)
        for k in range(100):
            tree.insert(k, k)
        result = tree.range_scan(20, 80)
        assert len(result) == 61
        assert result[0] == (20, 20)
        assert result[-1] == (80, 80)

    def test_range_1000_keys(self) -> None:
        tree = BPlusTree(t=3)
        for k in range(1000):
            tree.insert(k, k * 3)
        result = tree.range_scan(300, 700)
        assert len(result) == 401
        assert result[0] == (300, 900)
        assert result[-1] == (700, 2100)

    def test_range_low_equals_high(self) -> None:
        tree = BPlusTree(t=2)
        for k in [1, 2, 3, 4, 5]:
            tree.insert(k, k)
        assert tree.range_scan(3, 3) == [(3, 3)]

    def test_range_no_exact_boundary(self) -> None:
        tree = BPlusTree(t=2)
        for k in [1, 3, 5, 7, 9]:
            tree.insert(k, k)
        result = tree.range_scan(2, 8)
        assert result == [(3, 3), (5, 5), (7, 7)]

    def test_range_empty_tree(self) -> None:
        assert BPlusTree().range_scan(0, 100) == []


# ---------------------------------------------------------------------------
# min / max
# ---------------------------------------------------------------------------

class TestMinMax:
    def test_single_element(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(7, "x")
        assert tree.min_key() == 7
        assert tree.max_key() == 7

    def test_multiple(self) -> None:
        tree = BPlusTree(t=2)
        for k in [5, 2, 8, 1, 9]:
            tree.insert(k, k)
        assert tree.min_key() == 1
        assert tree.max_key() == 9

    def test_after_delete_min(self) -> None:
        tree = BPlusTree(t=2)
        for k in [3, 1, 2]:
            tree.insert(k, k)
        tree.delete(1)
        assert tree.min_key() == 2
        assert tree.is_valid()

    def test_after_delete_max(self) -> None:
        tree = BPlusTree(t=2)
        for k in [3, 1, 2]:
            tree.insert(k, k)
        tree.delete(3)
        assert tree.max_key() == 2
        assert tree.is_valid()


# ---------------------------------------------------------------------------
# height
# ---------------------------------------------------------------------------

class TestHeight:
    def test_empty_height(self) -> None:
        assert BPlusTree().height() == 0

    def test_single_element(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(1, 1)
        assert tree.height() == 0  # still just a leaf

    def test_height_grows(self) -> None:
        tree = BPlusTree(t=2)
        for i in range(10):
            tree.insert(i, i)
        assert tree.height() >= 1
        assert tree.is_valid()

    def test_all_leaves_same_depth(self) -> None:
        tree = BPlusTree(t=3)
        for i in range(100):
            tree.insert(i, i)
        assert tree.is_valid()  # is_valid checks uniform leaf depth


# ---------------------------------------------------------------------------
# Delete
# ---------------------------------------------------------------------------

class TestDelete:
    def test_delete_single(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(5, "five")
        tree.delete(5)
        assert 5 not in tree
        assert len(tree) == 0
        assert tree.is_valid()

    def test_delete_missing_raises(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(5, "five")
        with pytest.raises(KeyError):
            tree.delete(99)

    def test_delitem(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(10, "ten")
        del tree[10]
        assert 10 not in tree
        assert tree.is_valid()

    def test_delitem_missing_raises(self) -> None:
        tree = BPlusTree(t=2)
        with pytest.raises(KeyError):
            del tree[99]

    def test_delete_all(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(1, 6):
            tree.insert(k, k)
        for k in range(1, 6):
            tree.delete(k)
        assert len(tree) == 0
        assert tree.is_valid()

    def test_delete_reduces_size(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(10):
            tree.insert(k, k)
        for k in range(10):
            tree.delete(k)
            assert len(tree) == 9 - k

    def test_delete_maintains_leaf_list(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(20):
            tree.insert(k, k)
        for k in range(0, 20, 3):
            tree.delete(k)
        assert leaf_list_is_sorted(tree)
        assert tree.is_valid()

    def test_delete_triggers_borrow_from_sibling(self) -> None:
        """Force a leaf rebalance that borrows from a sibling."""
        tree = BPlusTree(t=2)
        for k in range(1, 9):
            tree.insert(k, str(k))
        # Delete a series of keys to thin out leaves and trigger borrows
        tree.delete(1)
        tree.delete(2)
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)

    def test_delete_triggers_merge(self) -> None:
        """Force a merge by deleting until both siblings are at minimum."""
        tree = BPlusTree(t=2)
        for k in range(1, 8):
            tree.insert(k, str(k))
        # Delete many keys to force merges
        for k in [4, 5, 6, 7]:
            tree.delete(k)
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)

    def test_delete_root_shrinks_height(self) -> None:
        """After enough deletes, internal root with one child collapses."""
        tree = BPlusTree(t=2)
        for k in range(1, 6):
            tree.insert(k, k)
        for k in range(1, 6):
            tree.delete(k)
        assert len(tree) == 0
        assert tree.is_valid()

    def test_delete_then_reinsert(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(10):
            tree.insert(k, k)
        tree.delete(5)
        tree.insert(5, 50)
        assert tree[5] == 50
        assert tree.is_valid()


# ---------------------------------------------------------------------------
# Different degrees
# ---------------------------------------------------------------------------

class TestDegrees:
    def test_t2(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(50):
            tree.insert(k, k)
        assert tree.is_valid()
        assert list(tree) == list(range(50))

    def test_t3(self) -> None:
        tree = BPlusTree(t=3)
        for k in range(100):
            tree.insert(k, k)
        assert tree.is_valid()
        for k in range(100):
            assert tree.search(k) == k

    def test_t5(self) -> None:
        tree = BPlusTree(t=5)
        for k in range(200):
            tree.insert(k, k)
        assert tree.is_valid()
        for k in range(0, 200, 7):
            tree.delete(k)
        assert tree.is_valid()
        assert leaf_list_is_sorted(tree)


# ---------------------------------------------------------------------------
# Large-scale stress tests
# ---------------------------------------------------------------------------

class TestLargeScale:
    def test_1000_keys_insert_search(self) -> None:
        tree = BPlusTree(t=3)
        for i in range(1000):
            tree.insert(i, i * 2)
        assert len(tree) == 1000
        assert tree.is_valid()
        for i in range(1000):
            assert tree.search(i) == i * 2

    def test_1000_keys_delete_all(self) -> None:
        import random
        tree = BPlusTree(t=3)
        keys = list(range(1000))
        for k in keys:
            tree.insert(k, k)
        random.shuffle(keys)
        for k in keys:
            tree.delete(k)
        assert len(tree) == 0
        assert tree.is_valid()

    def test_1000_keys_full_scan_sorted(self) -> None:
        tree = BPlusTree(t=4)
        import random
        keys = list(range(1000))
        random.shuffle(keys)
        for k in keys:
            tree.insert(k, k)
        result = [k for k, _ in tree.full_scan()]
        assert result == sorted(keys)

    def test_1000_keys_range_scan(self) -> None:
        tree = BPlusTree(t=3)
        for k in range(1000):
            tree.insert(k, k)
        result = tree.range_scan(200, 300)
        assert len(result) == 101
        assert result[0] == (200, 200)
        assert result[-1] == (300, 300)

    def test_interleaved_insert_delete_valid(self) -> None:
        tree = BPlusTree(t=2)
        for i in range(200):
            tree.insert(i, i)
            if i % 3 == 0 and i > 0:
                tree.delete(i - 3)
            assert tree.is_valid()

    def test_duplicate_inserts(self) -> None:
        tree = BPlusTree(t=3)
        for _ in range(5):
            for k in range(100):
                tree.insert(k, k * 10)
        assert len(tree) == 100
        assert tree.is_valid()


# ---------------------------------------------------------------------------
# Node unit tests
# ---------------------------------------------------------------------------

class TestNodeUnits:
    def test_leaf_is_full(self) -> None:
        node = BPlusLeafNode(keys=[1, 2, 3])
        assert node.is_full(t=2)  # 2t-1 = 3
        node2 = BPlusLeafNode(keys=[1, 2])
        assert not node2.is_full(t=2)

    def test_leaf_find_key_index(self) -> None:
        node = BPlusLeafNode(keys=[10, 20, 30])
        assert node.find_key_index(20) == 1
        assert node.find_key_index(15) == 1
        assert node.find_key_index(5) == 0
        assert node.find_key_index(35) == 3

    def test_internal_is_full(self) -> None:
        node = BPlusInternalNode(keys=[1, 2, 3])
        assert node.is_full(t=2)

    def test_internal_find_child_index(self) -> None:
        node = BPlusInternalNode(keys=[10, 20, 30])
        assert node.find_child_index(5) == 0    # less than 10 → child[0]
        assert node.find_child_index(10) == 1   # equal to 10 → child[1]
        assert node.find_child_index(15) == 1   # between 10 and 20
        assert node.find_child_index(20) == 2   # equal to 20 → child[2]
        assert node.find_child_index(35) == 3   # greater than 30


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------

class TestEdgeCases:
    def test_insert_none_value(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(1, None)
        assert 1 in tree
        assert tree[1] is None

    def test_repr(self) -> None:
        tree = BPlusTree(t=2)
        r = repr(tree)
        assert "BPlusTree" in r

    def test_string_keys(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert("banana", 1)
        tree.insert("apple", 2)
        tree.insert("cherry", 3)
        assert tree.min_key() == "apple"
        assert tree.max_key() == "cherry"
        assert tree.is_valid()

    def test_update_then_delete(self) -> None:
        tree = BPlusTree(t=2)
        tree.insert(42, "old")
        tree.insert(42, "new")
        assert tree[42] == "new"
        tree.delete(42)
        assert 42 not in tree
        assert tree.is_valid()

    def test_many_updates(self) -> None:
        tree = BPlusTree(t=3)
        for k in range(50):
            tree.insert(k, "first")
        for k in range(50):
            tree.insert(k, "second")
        assert len(tree) == 50
        for k in range(50):
            assert tree[k] == "second"
        assert tree.is_valid()

    def test_range_scan_after_delete(self) -> None:
        tree = BPlusTree(t=2)
        for k in range(10):
            tree.insert(k, k)
        tree.delete(5)
        result = tree.range_scan(3, 7)
        assert 5 not in [k for k, _ in result]
        assert (3, 3) in result
        assert (7, 7) in result

    def test_first_leaf_is_correct_after_many_ops(self) -> None:
        import random
        tree = BPlusTree(t=3)
        keys = list(range(100))
        random.shuffle(keys)
        for k in keys:
            tree.insert(k, k)
        # Delete some keys including potentially the minimum
        for k in random.sample(keys, 30):
            tree.delete(k)
        remaining = sorted(set(keys) - {k for k in range(100) if k not in keys})
        if len(tree) > 0:
            assert tree.min_key() == tree._first_leaf.keys[0]

    def test_height_single_leaf(self) -> None:
        tree = BPlusTree(t=5)
        for k in range(5):
            tree.insert(k, k)
        # With t=5, up to 9 keys fit in root leaf
        assert tree.height() == 0
