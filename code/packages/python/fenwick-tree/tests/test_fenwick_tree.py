"""
Tests for fenwick_tree.py — Binary Indexed Tree.

Test strategy:
- Correctness against brute-force prefix/range sums
- Update correctness with interleaved queries
- Edge cases: empty tree, single element, all zeros, negative values
- find_kth order statistics
- Error conditions: out-of-range indices, invalid arguments
- Stress test: 500 random arrays × all prefix and range sums
"""

from __future__ import annotations

import random

import pytest

from fenwick_tree import FenwickError, FenwickTree
from fenwick_tree.fenwick_tree import EmptyTreeError, IndexOutOfRangeError


# ─── Helpers ─────────────────────────────────────────────────────────────────


def brute_prefix(arr: list[int | float], i: int) -> int | float:
    """Sum of arr[0..i-1] (0-indexed), corresponding to positions 1..i."""
    return sum(arr[:i])


def brute_range(arr: list[int | float], l: int, r: int) -> int | float:
    """Sum of arr[l-1..r-1] where l, r are 1-indexed."""
    return sum(arr[l - 1 : r])


# ─── Construction ─────────────────────────────────────────────────────────────


class TestConstruction:
    def test_empty_tree(self) -> None:
        ft = FenwickTree(0)
        assert len(ft) == 0

    def test_negative_size_raises(self) -> None:
        with pytest.raises(FenwickError):
            FenwickTree(-1)

    def test_from_list_basic(self) -> None:
        ft = FenwickTree.from_list([3, 2, 1, 7, 4])
        assert len(ft) == 5

    def test_from_list_empty(self) -> None:
        ft = FenwickTree.from_list([])
        assert len(ft) == 0

    def test_from_list_single(self) -> None:
        ft = FenwickTree.from_list([42])
        assert ft.prefix_sum(1) == 42

    def test_from_list_all_zeros(self) -> None:
        ft = FenwickTree.from_list([0, 0, 0, 0])
        assert ft.prefix_sum(4) == 0

    def test_from_list_negative_values(self) -> None:
        arr = [-3, -1, 5, -2]
        ft = FenwickTree.from_list(arr)
        assert ft.prefix_sum(4) == sum(arr)

    def test_from_list_floats(self) -> None:
        arr = [1.5, 2.5, 3.0]
        ft = FenwickTree.from_list(arr)
        assert abs(ft.prefix_sum(3) - 7.0) < 1e-9

    def test_repr(self) -> None:
        ft = FenwickTree.from_list([1, 2])
        r = repr(ft)
        assert "FenwickTree" in r
        assert "n=2" in r


# ─── Prefix Sum ───────────────────────────────────────────────────────────────


class TestPrefixSum:
    def test_spec_example(self) -> None:
        # From spec: arr = [3, 2, 1, 7, 4]
        # bit after build: [_, 3, 5, 1, 13, 4]
        ft = FenwickTree.from_list([3, 2, 1, 7, 4])
        assert ft.prefix_sum(1) == 3
        assert ft.prefix_sum(2) == 5
        assert ft.prefix_sum(3) == 6
        assert ft.prefix_sum(4) == 13
        assert ft.prefix_sum(5) == 17

    def test_prefix_sum_zero(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        assert ft.prefix_sum(0) == 0

    def test_prefix_sum_out_of_range_high(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(IndexOutOfRangeError):
            ft.prefix_sum(4)

    def test_prefix_sum_out_of_range_negative(self) -> None:
        ft = FenwickTree.from_list([1, 2])
        with pytest.raises(IndexOutOfRangeError):
            ft.prefix_sum(-1)

    def test_prefix_sum_power_of_two(self) -> None:
        # prefix_sum(8) should be a single-step query
        arr = list(range(1, 9))
        ft = FenwickTree.from_list(arr)
        assert ft.prefix_sum(8) == sum(arr)

    def test_prefix_sum_all_ones(self) -> None:
        n = 16
        ft = FenwickTree.from_list([1] * n)
        for i in range(1, n + 1):
            assert ft.prefix_sum(i) == i


# ─── Range Sum ────────────────────────────────────────────────────────────────


class TestRangeSum:
    def test_spec_example(self) -> None:
        ft = FenwickTree.from_list([3, 2, 1, 7, 4])
        # range_sum(2, 4) = 2+1+7 = 10
        assert ft.range_sum(2, 4) == 10

    def test_range_sum_single_element(self) -> None:
        ft = FenwickTree.from_list([3, 2, 1, 7, 4])
        assert ft.range_sum(3, 3) == 1
        assert ft.range_sum(4, 4) == 7

    def test_range_sum_full(self) -> None:
        arr = [3, 2, 1, 7, 4]
        ft = FenwickTree.from_list(arr)
        assert ft.range_sum(1, 5) == sum(arr)

    def test_range_sum_l_gt_r_raises(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(FenwickError):
            ft.range_sum(3, 1)

    def test_range_sum_out_of_range(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(IndexOutOfRangeError):
            ft.range_sum(0, 3)

    def test_range_sum_starting_at_1(self) -> None:
        # When l=1, uses the fast path (prefix_sum only, no subtraction)
        arr = [5, 3, 8, 1]
        ft = FenwickTree.from_list(arr)
        assert ft.range_sum(1, 4) == sum(arr)
        assert ft.range_sum(1, 2) == 8

    def test_range_sum_negative_values(self) -> None:
        arr = [-5, 3, -2, 4]
        ft = FenwickTree.from_list(arr)
        assert ft.range_sum(1, 4) == 0
        assert ft.range_sum(2, 3) == 1


# ─── Point Query ──────────────────────────────────────────────────────────────


class TestPointQuery:
    def test_initial_values(self) -> None:
        arr = [3, 2, 1, 7, 4]
        ft = FenwickTree.from_list(arr)
        for i, v in enumerate(arr, start=1):
            assert ft.point_query(i) == v

    def test_out_of_range(self) -> None:
        ft = FenwickTree.from_list([1, 2])
        with pytest.raises(IndexOutOfRangeError):
            ft.point_query(3)

    def test_zero_value(self) -> None:
        ft = FenwickTree.from_list([0, 5, 0])
        assert ft.point_query(1) == 0
        assert ft.point_query(2) == 5
        assert ft.point_query(3) == 0


# ─── Update ───────────────────────────────────────────────────────────────────


class TestUpdate:
    def test_spec_update(self) -> None:
        # From spec: arr[3] += 5 changes [3,2,1,7,4] to [3,2,6,7,4]
        arr = [3, 2, 1, 7, 4]
        ft = FenwickTree.from_list(arr)
        ft.update(3, 5)
        assert ft.prefix_sum(3) == 11  # 3+2+6
        assert ft.prefix_sum(4) == 18  # 3+2+6+7
        assert ft.point_query(3) == 6

    def test_update_propagation_from_1(self) -> None:
        # Updating index 1 propagates to all power-of-2 cells
        n = 8
        ft = FenwickTree.from_list([0] * n)
        ft.update(1, 10)
        # All prefix sums from 1 to n should now be 10
        for i in range(1, n + 1):
            assert ft.prefix_sum(i) == 10

    def test_update_negative_delta(self) -> None:
        arr = [5, 5, 5]
        ft = FenwickTree.from_list(arr)
        ft.update(2, -3)
        assert ft.point_query(2) == 2
        assert ft.prefix_sum(3) == 12  # 5+2+5

    def test_update_to_zero(self) -> None:
        ft = FenwickTree.from_list([7, 3, 2])
        ft.update(1, -7)  # arr[1] = 0
        assert ft.point_query(1) == 0
        assert ft.prefix_sum(3) == 5

    def test_update_out_of_range(self) -> None:
        ft = FenwickTree.from_list([1, 2])
        with pytest.raises(IndexOutOfRangeError):
            ft.update(3, 5)

    def test_update_out_of_range_zero(self) -> None:
        ft = FenwickTree.from_list([1, 2])
        with pytest.raises(IndexOutOfRangeError):
            ft.update(0, 5)

    def test_update_last_element(self) -> None:
        arr = [1, 2, 3, 4, 5]
        ft = FenwickTree.from_list(arr)
        ft.update(5, 10)
        assert ft.point_query(5) == 15
        assert ft.prefix_sum(5) == sum(arr) + 10

    def test_multiple_updates(self) -> None:
        arr = [1, 1, 1, 1, 1]
        ft = FenwickTree.from_list(arr)
        ft.update(1, 9)   # arr[1] = 10
        ft.update(3, 4)   # arr[3] = 5
        ft.update(5, -1)  # arr[5] = 0
        assert ft.prefix_sum(5) == 10 + 1 + 5 + 1 + 0


# ─── find_kth ─────────────────────────────────────────────────────────────────


class TestFindKth:
    def test_spec_examples(self) -> None:
        # arr = [1, 2, 3, 4, 5] → prefix sums: 1, 3, 6, 10, 15
        arr = [1, 2, 3, 4, 5]
        ft = FenwickTree.from_list(arr)
        assert ft.find_kth(1) == 1
        assert ft.find_kth(2) == 2
        assert ft.find_kth(3) == 2
        assert ft.find_kth(4) == 3
        assert ft.find_kth(10) == 4
        assert ft.find_kth(11) == 5

    def test_find_kth_k_equals_1(self) -> None:
        ft = FenwickTree.from_list([5, 3, 2])
        assert ft.find_kth(1) == 1

    def test_find_kth_all_ones(self) -> None:
        ft = FenwickTree.from_list([1, 1, 1, 1, 1])
        for k in range(1, 6):
            assert ft.find_kth(k) == k

    def test_find_kth_k_equals_total(self) -> None:
        arr = [3, 2, 1, 7, 4]
        ft = FenwickTree.from_list(arr)
        # total = 17, so find_kth(17) should return 5
        assert ft.find_kth(17) == 5

    def test_find_kth_large_first_element(self) -> None:
        ft = FenwickTree.from_list([100, 1, 1, 1])
        assert ft.find_kth(50) == 1
        assert ft.find_kth(100) == 1
        assert ft.find_kth(101) == 2

    def test_find_kth_empty_tree_raises(self) -> None:
        ft = FenwickTree(0)
        with pytest.raises(EmptyTreeError):
            ft.find_kth(1)

    def test_find_kth_k_zero_raises(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(FenwickError):
            ft.find_kth(0)

    def test_find_kth_k_negative_raises(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(FenwickError):
            ft.find_kth(-1)

    def test_find_kth_k_exceeds_total_raises(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        with pytest.raises(FenwickError):
            ft.find_kth(100)

    def test_find_kth_power_of_two_size(self) -> None:
        # n = 8, all values = 2; prefix sums: 2, 4, 6, 8, 10, 12, 14, 16
        ft = FenwickTree.from_list([2] * 8)
        assert ft.find_kth(1) == 1
        assert ft.find_kth(2) == 1
        assert ft.find_kth(3) == 2
        assert ft.find_kth(16) == 8


# ─── Lowbit Correctness ───────────────────────────────────────────────────────


class TestLowbit:
    """
    The lowbit function (i & -i) is the heart of the Fenwick tree.
    These tests verify the values match the spec's table.
    """

    def test_lowbit_values(self) -> None:
        expected = {
            1: 1, 2: 2, 3: 1, 4: 4, 5: 1, 6: 2,
            7: 1, 8: 8, 12: 4, 16: 16, 24: 8, 32: 32, 64: 64,
        }
        for i, lb in expected.items():
            assert (i & -i) == lb, f"lowbit({i}) should be {lb}"


# ─── Correctness Against Brute Force ─────────────────────────────────────────


class TestBruteForce:
    def test_all_prefix_sums_random_arrays(self) -> None:
        """Verify prefix_sum matches naive summation on 200 random arrays."""
        rng = random.Random(42)
        for _ in range(200):
            n = rng.randint(1, 50)
            arr = [rng.randint(-50, 50) for _ in range(n)]
            ft = FenwickTree.from_list(arr)
            for i in range(1, n + 1):
                assert ft.prefix_sum(i) == brute_prefix(arr, i), (
                    f"Mismatch at prefix_sum({i}) for arr={arr}"
                )

    def test_all_range_sums_random_arrays(self) -> None:
        """Verify range_sum matches naive summation on 100 random arrays."""
        rng = random.Random(99)
        for _ in range(100):
            n = rng.randint(1, 30)
            arr = [rng.randint(-50, 50) for _ in range(n)]
            ft = FenwickTree.from_list(arr)
            for l in range(1, n + 1):
                for r in range(l, n + 1):
                    assert ft.range_sum(l, r) == brute_range(arr, l, r), (
                        f"Mismatch at range_sum({l},{r}) for arr={arr}"
                    )

    def test_interleaved_updates_and_queries(self) -> None:
        """Stress test: 2000 random updates/queries against ground truth."""
        rng = random.Random(7)
        n = 100
        arr = [rng.randint(1, 100) for _ in range(n)]
        ft = FenwickTree.from_list(arr)

        for _ in range(2000):
            if rng.random() < 0.4:
                # Query
                l = rng.randint(1, n)
                r = rng.randint(l, n)
                assert ft.range_sum(l, r) == brute_range(arr, l, r)
            else:
                # Update
                i = rng.randint(1, n)
                delta = rng.randint(-50, 50)
                arr[i - 1] += delta
                ft.update(i, delta)


# ─── Edge Cases ───────────────────────────────────────────────────────────────


class TestEdgeCases:
    def test_single_element(self) -> None:
        ft = FenwickTree.from_list([99])
        assert ft.prefix_sum(1) == 99
        assert ft.range_sum(1, 1) == 99
        assert ft.point_query(1) == 99
        ft.update(1, 1)
        assert ft.point_query(1) == 100

    def test_all_negative(self) -> None:
        arr = [-1, -2, -3, -4]
        ft = FenwickTree.from_list(arr)
        assert ft.prefix_sum(4) == -10
        assert ft.range_sum(2, 3) == -5

    def test_large_array(self) -> None:
        n = 1000
        arr = [1] * n
        ft = FenwickTree.from_list(arr)
        assert ft.prefix_sum(n) == n
        assert ft.range_sum(1, n) == n
        ft.update(500, 999)
        assert ft.point_query(500) == 1000
        assert ft.prefix_sum(n) == n + 999

    def test_prefix_sum_equals_total(self) -> None:
        arr = [5, 10, 15]
        ft = FenwickTree.from_list(arr)
        assert ft.prefix_sum(3) == 30

    def test_update_all_positions(self) -> None:
        """Update every position and verify all prefix sums."""
        n = 8
        ft = FenwickTree(n)
        arr = [0] * n
        for i in range(1, n + 1):
            ft.update(i, i * 2)
            arr[i - 1] = i * 2
        for i in range(1, n + 1):
            assert ft.prefix_sum(i) == sum(arr[:i])

    def test_float_values(self) -> None:
        arr = [0.1, 0.2, 0.3, 0.4]
        ft = FenwickTree.from_list(arr)
        assert abs(ft.prefix_sum(4) - 1.0) < 1e-9
        assert abs(ft.range_sum(2, 3) - 0.5) < 1e-9


# ─── Dunder Methods ───────────────────────────────────────────────────────────


class TestDunderMethods:
    def test_len(self) -> None:
        ft = FenwickTree.from_list([1, 2, 3])
        assert len(ft) == 3

    def test_len_empty(self) -> None:
        ft = FenwickTree(0)
        assert len(ft) == 0

    def test_repr_includes_n(self) -> None:
        ft = FenwickTree.from_list([10, 20])
        assert "n=2" in repr(ft)
