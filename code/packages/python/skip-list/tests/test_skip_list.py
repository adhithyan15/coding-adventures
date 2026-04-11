"""Tests for the skip list implementation.

Note: import skip_list.skip_list internals for sentinel coverage tests.

Test strategy: Every skip list operation is verified against a plain sorted
Python list as the reference. If the skip list and the reference agree on
every operation, the skip list is correct (regardless of its internal structure).

Coverage areas:
  - Basic insert and search
  - Delete (found / not found)
  - Sorted iteration
  - Range queries (inclusive and exclusive)
  - Rank and by_rank (with round-trip checks)
  - Duplicate key insertion (value update)
  - None values stored against real keys
  - Large dataset (10,000 random insertions)
  - Edge cases: empty list, single element, missing keys
  - __len__, __contains__, __repr__
  - Custom max_level and probability
"""

import random

import pytest

from skip_list import SkipList
from skip_list.skip_list import NEG_INF, POS_INF

# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------


@pytest.fixture()
def empty_sl() -> SkipList:
    return SkipList()


@pytest.fixture()
def small_sl() -> SkipList:
    """Skip list with keys [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]."""
    sl = SkipList()
    for k in [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]:
        sl.insert(k, k * 10)
    return sl


# ---------------------------------------------------------------------------
# Empty list edge cases
# ---------------------------------------------------------------------------


class TestEmptyList:
    def test_len_is_zero(self, empty_sl: SkipList) -> None:
        assert len(empty_sl) == 0

    def test_search_returns_none(self, empty_sl: SkipList) -> None:
        assert empty_sl.search(42) is None

    def test_contains_returns_false(self, empty_sl: SkipList) -> None:
        assert (42 in empty_sl) is False

    def test_delete_returns_false(self, empty_sl: SkipList) -> None:
        assert empty_sl.delete(42) is False

    def test_iter_yields_nothing(self, empty_sl: SkipList) -> None:
        assert list(empty_sl) == []

    def test_rank_returns_none(self, empty_sl: SkipList) -> None:
        assert empty_sl.rank(42) is None

    def test_by_rank_returns_none(self, empty_sl: SkipList) -> None:
        assert empty_sl.by_rank(0) is None

    def test_range_query_empty(self, empty_sl: SkipList) -> None:
        assert empty_sl.range_query(1, 10) == []

    def test_repr(self, empty_sl: SkipList) -> None:
        assert repr(empty_sl) == "SkipList([])"


# ---------------------------------------------------------------------------
# Single-element list
# ---------------------------------------------------------------------------


class TestSingleElement:
    def test_insert_and_search(self) -> None:
        sl = SkipList()
        sl.insert(42, "answer")
        assert sl.search(42) == "answer"

    def test_len(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert len(sl) == 1

    def test_contains(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert (42 in sl) is True
        assert (99 in sl) is False

    def test_iter(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert list(sl) == [42]

    def test_rank(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert sl.rank(42) == 0

    def test_by_rank(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert sl.by_rank(0) == 42
        assert sl.by_rank(1) is None

    def test_delete_found(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert sl.delete(42) is True
        assert len(sl) == 0
        assert sl.search(42) is None

    def test_delete_not_found(self) -> None:
        sl = SkipList()
        sl.insert(42)
        assert sl.delete(99) is False
        assert len(sl) == 1


# ---------------------------------------------------------------------------
# Insert and search
# ---------------------------------------------------------------------------


class TestInsertAndSearch:
    def test_multiple_inserts(self) -> None:
        sl = SkipList()
        pairs = [(1, "one"), (2, "two"), (3, "three"), (10, "ten")]
        for k, v in pairs:
            sl.insert(k, v)
        for k, v in pairs:
            assert sl.search(k) == v

    def test_search_missing_key(self, small_sl: SkipList) -> None:
        assert small_sl.search(999) is None
        assert small_sl.search(-1) is None
        assert small_sl.search(0) is None

    def test_duplicate_key_updates_value(self) -> None:
        sl = SkipList()
        sl.insert(10, "original")
        assert sl.search(10) == "original"
        sl.insert(10, "updated")
        assert sl.search(10) == "updated"
        assert len(sl) == 1  # size should not change

    def test_none_value(self) -> None:
        """Inserting with value=None should be detectable via contains."""
        sl = SkipList()
        sl.insert(7, None)
        assert (7 in sl) is True
        assert sl.search(7) is None  # value IS None — search returns None too
        assert len(sl) == 1

    def test_none_value_contains(self) -> None:
        """contains() should distinguish None value from missing key."""
        sl = SkipList()
        sl.insert(7, None)
        assert sl.contains(7) is True
        assert sl.contains(8) is False

    def test_insert_order_does_not_matter(self) -> None:
        """Inserting in reverse order should still produce sorted output."""
        sl = SkipList()
        for k in reversed(range(10)):
            sl.insert(k, k)
        assert list(sl) == list(range(10))

    def test_string_keys(self) -> None:
        sl = SkipList()
        words = ["banana", "apple", "cherry", "date"]
        for w in words:
            sl.insert(w, len(w))
        assert list(sl) == sorted(words)
        assert sl.search("apple") == 5

    def test_float_keys(self) -> None:
        sl = SkipList()
        keys = [3.14, 2.71, 1.41, 0.57]
        for k in keys:
            sl.insert(k, str(k))
        assert list(sl) == sorted(keys)


# ---------------------------------------------------------------------------
# Delete
# ---------------------------------------------------------------------------


class TestDelete:
    def test_delete_existing(self, small_sl: SkipList) -> None:
        assert (37 in small_sl) is True
        result = small_sl.delete(37)
        assert result is True
        assert (37 in small_sl) is False
        assert small_sl.search(37) is None

    def test_delete_nonexistent(self, small_sl: SkipList) -> None:
        original_len = len(small_sl)
        result = small_sl.delete(999)
        assert result is False
        assert len(small_sl) == original_len

    def test_delete_reduces_size(self, small_sl: SkipList) -> None:
        original_len = len(small_sl)
        small_sl.delete(5)
        assert len(small_sl) == original_len - 1

    def test_delete_all_elements(self) -> None:
        sl = SkipList()
        keys = [1, 2, 3, 4, 5]
        for k in keys:
            sl.insert(k)
        for k in keys:
            assert sl.delete(k) is True
        assert len(sl) == 0
        assert list(sl) == []

    def test_delete_first_element(self) -> None:
        sl = SkipList()
        for k in [1, 2, 3]:
            sl.insert(k)
        sl.delete(1)
        assert list(sl) == [2, 3]

    def test_delete_last_element(self) -> None:
        sl = SkipList()
        for k in [1, 2, 3]:
            sl.insert(k)
        sl.delete(3)
        assert list(sl) == [1, 2]

    def test_delete_middle_element(self) -> None:
        sl = SkipList()
        for k in [1, 2, 3]:
            sl.insert(k)
        sl.delete(2)
        assert list(sl) == [1, 3]

    def test_reinsert_after_delete(self) -> None:
        sl = SkipList()
        sl.insert(5, "a")
        sl.delete(5)
        sl.insert(5, "b")
        assert sl.search(5) == "b"
        assert len(sl) == 1

    def test_delete_then_sorted_order_maintained(self, small_sl: SkipList) -> None:
        keys_before = list(small_sl)
        small_sl.delete(20)
        small_sl.delete(63)
        keys_after = list(small_sl)
        assert keys_after == [k for k in keys_before if k not in (20, 63)]


# ---------------------------------------------------------------------------
# Sorted iteration
# ---------------------------------------------------------------------------


class TestSortedIteration:
    def test_sorted_after_random_inserts(self) -> None:
        rng = random.Random(42)
        keys = [rng.randint(0, 1000) for _ in range(200)]
        sl = SkipList()
        for k in keys:
            sl.insert(k)
        assert list(sl) == sorted(set(keys))

    def test_iter_gives_only_keys(self) -> None:
        sl = SkipList()
        sl.insert(1, "a")
        sl.insert(2, "b")
        sl.insert(3, "c")
        # __iter__ yields keys only
        assert list(sl) == [1, 2, 3]

    def test_for_loop(self) -> None:
        sl = SkipList()
        for k in [30, 10, 20]:
            sl.insert(k)
        collected = []
        for k in sl:
            collected.append(k)
        assert collected == [10, 20, 30]


# ---------------------------------------------------------------------------
# Range queries
# ---------------------------------------------------------------------------


class TestRangeQuery:
    def test_range_within_bounds(self, small_sl: SkipList) -> None:
        # keys: [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]
        result = small_sl.range_query(20, 55)
        assert [k for k, _ in result] == [20, 37, 42, 50, 55]

    def test_range_values_correct(self, small_sl: SkipList) -> None:
        result = small_sl.range_query(42, 42)
        assert result == [(42, 420)]

    def test_range_empty_result(self, small_sl: SkipList) -> None:
        assert small_sl.range_query(200, 300) == []

    def test_range_no_match_between_elements(self, small_sl: SkipList) -> None:
        # 6 and 11 are between 5 and 12 but not in the list
        assert small_sl.range_query(6, 11) == []

    def test_range_full_list(self, small_sl: SkipList) -> None:
        result = small_sl.range_query(0, 200)
        assert [k for k, _ in result] == [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]

    def test_range_exclusive(self, small_sl: SkipList) -> None:
        # exclusive: lo < key < hi
        result = small_sl.range_query(20, 55, inclusive=False)
        assert [k for k, _ in result] == [37, 42, 50]

    def test_range_exclusive_single_boundary(self, small_sl: SkipList) -> None:
        result = small_sl.range_query(42, 42, inclusive=False)
        assert result == []

    def test_range_at_list_boundaries(self, small_sl: SkipList) -> None:
        result = small_sl.range_query(5, 5)
        assert result == [(5, 50)]
        result = small_sl.range_query(100, 100)
        assert result == [(100, 1000)]

    def test_range_on_empty_list(self, empty_sl: SkipList) -> None:
        assert empty_sl.range_query(1, 10) == []

    def test_range_lo_greater_than_hi(self, small_sl: SkipList) -> None:
        # When lo > hi, result should always be empty
        assert small_sl.range_query(50, 20) == []


# ---------------------------------------------------------------------------
# Rank and by_rank
# ---------------------------------------------------------------------------


class TestRankOperations:
    def test_rank_of_each_element(self) -> None:
        sl = SkipList()
        for k in [10, 20, 30, 40, 50]:
            sl.insert(k)
        assert sl.rank(10) == 0
        assert sl.rank(20) == 1
        assert sl.rank(30) == 2
        assert sl.rank(40) == 3
        assert sl.rank(50) == 4

    def test_rank_missing_key(self) -> None:
        sl = SkipList()
        for k in [10, 20, 30]:
            sl.insert(k)
        assert sl.rank(99) is None
        assert sl.rank(0) is None
        assert sl.rank(15) is None  # between 10 and 20

    def test_by_rank_each_position(self) -> None:
        sl = SkipList()
        for k in [10, 20, 30, 40, 50]:
            sl.insert(k)
        assert sl.by_rank(0) == 10
        assert sl.by_rank(1) == 20
        assert sl.by_rank(2) == 30
        assert sl.by_rank(3) == 40
        assert sl.by_rank(4) == 50

    def test_by_rank_out_of_range(self) -> None:
        sl = SkipList()
        for k in [10, 20, 30]:
            sl.insert(k)
        assert sl.by_rank(3) is None   # == len
        assert sl.by_rank(100) is None
        assert sl.by_rank(-1) is None

    def test_rank_and_by_rank_are_inverses(self) -> None:
        """rank and by_rank should be inverses of each other."""
        rng = random.Random(123)
        keys = sorted(set(rng.randint(0, 500) for _ in range(50)))
        sl = SkipList()
        for k in keys:
            sl.insert(k)

        # rank → by_rank roundtrip
        for k in keys:
            r = sl.rank(k)
            assert r is not None
            assert sl.by_rank(r) == k

        # by_rank → rank roundtrip
        for i in range(len(keys)):
            k = sl.by_rank(i)
            assert k is not None
            assert sl.rank(k) == i

    def test_rank_after_delete(self) -> None:
        sl = SkipList()
        for k in [10, 20, 30, 40]:
            sl.insert(k)
        sl.delete(20)
        assert sl.rank(10) == 0
        assert sl.rank(30) == 1
        assert sl.rank(40) == 2
        assert sl.rank(20) is None

    def test_by_rank_after_insert(self) -> None:
        sl = SkipList()
        sl.insert(10)
        sl.insert(30)
        sl.insert(20)  # inserted in wrong order; should be rank 1
        assert sl.by_rank(0) == 10
        assert sl.by_rank(1) == 20
        assert sl.by_rank(2) == 30


# ---------------------------------------------------------------------------
# __len__, __contains__, __repr__
# ---------------------------------------------------------------------------


class TestProtocol:
    def test_len_tracks_insertions(self) -> None:
        sl = SkipList()
        for i in range(10):
            sl.insert(i)
            assert len(sl) == i + 1

    def test_len_tracks_deletions(self) -> None:
        sl = SkipList()
        for i in range(5):
            sl.insert(i)
        for i in range(5):
            sl.delete(i)
            assert len(sl) == 4 - i

    def test_contains_true(self, small_sl: SkipList) -> None:
        for k in [5, 12, 20, 37, 42, 50, 55, 63, 75, 100]:
            assert (k in small_sl) is True

    def test_contains_false(self, small_sl: SkipList) -> None:
        for k in [0, 6, 13, 101, -1, 999]:
            assert (k in small_sl) is False

    def test_repr_nonempty(self) -> None:
        sl = SkipList()
        sl.insert(3)
        sl.insert(1)
        sl.insert(2)
        assert repr(sl) == "SkipList([1, 2, 3])"

    def test_repr_empty(self, empty_sl: SkipList) -> None:
        assert repr(empty_sl) == "SkipList([])"


# ---------------------------------------------------------------------------
# Custom parameters
# ---------------------------------------------------------------------------


class TestCustomParameters:
    def test_small_max_level(self) -> None:
        """A max_level=1 skip list is just a sorted linked list; still correct."""
        sl = SkipList(max_level=1, p=0.5)
        for k in [5, 3, 7, 1, 9]:
            sl.insert(k)
        assert list(sl) == [1, 3, 5, 7, 9]
        assert sl.search(3) is None  # no associated value
        assert (7 in sl) is True

    def test_high_probability(self) -> None:
        """p=0.9 creates taller nodes; correctness should still hold."""
        sl = SkipList(max_level=8, p=0.9)
        keys = list(range(20))
        for k in keys:
            sl.insert(k, k * 2)
        assert list(sl) == keys
        for k in keys:
            assert sl.search(k) == k * 2

    def test_low_probability(self) -> None:
        """p=0.1 creates very flat nodes; correctness should still hold."""
        sl = SkipList(max_level=16, p=0.1)
        keys = list(range(100))
        random.shuffle(keys)
        for k in keys:
            sl.insert(k)
        assert list(sl) == sorted(keys)


# ---------------------------------------------------------------------------
# Stress tests
# ---------------------------------------------------------------------------


class TestStress:
    def test_large_dataset_sorted_order(self) -> None:
        """10,000 random insertions; output must be sorted."""
        rng = random.Random(0)
        keys = [rng.randint(0, 100_000) for _ in range(10_000)]
        sl = SkipList()
        for k in keys:
            sl.insert(k, k)
        result = list(sl)
        assert result == sorted(set(keys))

    def test_large_dataset_search_correctness(self) -> None:
        """Search on 10,000 elements must find all inserted keys."""
        rng = random.Random(1)
        keys = sorted(set(rng.randint(0, 50_000) for _ in range(5_000)))
        sl = SkipList()
        for k in keys:
            sl.insert(k, k * 3)
        for k in keys:
            assert sl.search(k) == k * 3

    def test_mixed_insert_delete_invariants(self) -> None:
        """Interleaved inserts and deletes maintain correct size and sorted order."""
        rng = random.Random(42)
        sl = SkipList()
        reference: set[int] = set()

        for _ in range(5_000):
            op = rng.choices(["insert", "delete"], weights=[0.7, 0.3])[0]
            key = rng.randint(0, 500)

            if op == "insert":
                sl.insert(key, key)
                reference.add(key)
            else:
                sl.delete(key)
                reference.discard(key)

        assert len(sl) == len(reference)
        assert list(sl) == sorted(reference)

    def test_rank_consistency_after_many_ops(self) -> None:
        """After many inserts, rank and by_rank must be consistent."""
        rng = random.Random(7)
        keys = sorted(set(rng.randint(0, 200) for _ in range(100)))
        sl = SkipList()
        for k in keys:
            sl.insert(k)

        for i, k in enumerate(keys):
            assert sl.rank(k) == i
            assert sl.by_rank(i) == k

    def test_range_query_matches_reference(self) -> None:
        """Range query results must match a simple list filter."""
        rng = random.Random(99)
        keys_vals = sorted(
            {rng.randint(0, 1000): rng.randint(0, 9999) for _ in range(300)}.items()
        )
        sl = SkipList()
        for k, v in keys_vals:
            sl.insert(k, v)

        lo, hi = 200, 600
        expected = [(k, v) for k, v in keys_vals if lo <= k <= hi]
        assert sl.range_query(lo, hi) == expected

    def test_repeated_duplicate_inserts(self) -> None:
        """Inserting the same key repeatedly should always update value."""
        sl = SkipList()
        for i in range(100):
            sl.insert(42, i)
        assert sl.search(42) == 99
        assert len(sl) == 1

    def test_insert_delete_all_reinsert(self) -> None:
        """Delete all elements then re-insert; list should work from scratch."""
        sl = SkipList()
        keys = list(range(50))
        for k in keys:
            sl.insert(k)
        for k in keys:
            sl.delete(k)
        assert len(sl) == 0
        # Re-insert in reverse order
        for k in reversed(keys):
            sl.insert(k)
        assert list(sl) == keys
        assert len(sl) == len(keys)


# ---------------------------------------------------------------------------
# Internal sentinel comparison coverage
# ---------------------------------------------------------------------------


class TestSentinels:
    """Exercise _NegInf and _PosInf comparison operators directly.

    These sentinels are used internally but some of their comparison
    branches are not exercised through normal SkipList operations.
    """

    def test_neg_inf_lt(self) -> None:
        assert (NEG_INF < 0) is True
        assert (NEG_INF < "hello") is True

    def test_neg_inf_le(self) -> None:
        assert (NEG_INF <= 0) is True
        assert (NEG_INF <= NEG_INF) is True

    def test_neg_inf_gt(self) -> None:
        assert (NEG_INF > 0) is False

    def test_neg_inf_ge(self) -> None:
        assert (NEG_INF >= 0) is False
        assert (NEG_INF >= NEG_INF) is True

    def test_neg_inf_eq(self) -> None:
        assert (NEG_INF == NEG_INF) is True
        assert (NEG_INF == 0) is False

    def test_neg_inf_repr(self) -> None:
        assert repr(NEG_INF) == "-inf"

    def test_pos_inf_lt(self) -> None:
        assert (POS_INF < 0) is False

    def test_pos_inf_le(self) -> None:
        assert (POS_INF <= 0) is False
        assert (POS_INF <= POS_INF) is True

    def test_pos_inf_gt(self) -> None:
        assert (POS_INF > 0) is True
        assert (POS_INF > "anything") is True

    def test_pos_inf_ge(self) -> None:
        assert (POS_INF >= 0) is True

    def test_pos_inf_eq(self) -> None:
        assert (POS_INF == POS_INF) is True
        assert (POS_INF == 0) is False

    def test_pos_inf_repr(self) -> None:
        assert repr(POS_INF) == "+inf"
