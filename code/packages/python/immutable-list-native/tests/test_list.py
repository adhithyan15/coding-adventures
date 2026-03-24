"""
Tests for immutable-list-native
================================

These tests verify the Python native extension wrapping the Rust
ImmutableList crate. The key property being tested throughout is
**immutability**: every mutation method (push, set, pop) must return
a NEW list and leave the original unchanged.

Test categories:
  1. Construction and basic operations
  2. Push / structural sharing
  3. Get with positive and negative indices
  4. Set (returns new list)
  5. Pop (returns tuple of new list and removed value)
  6. Large list (1000+ elements)
  7. Iterator protocol (__iter__)
  8. __getitem__ with negative indices
  9. __len__
  10. __eq__ / __repr__
  11. Edge cases (empty list)
  12. from_list class method
"""

import pytest
from immutable_list_native import ImmutableList, ImmutableListError


# =========================================================================
# 1. Construction and basic operations
# =========================================================================

class TestConstruction:
    """Test creating ImmutableList objects."""

    def test_empty_list(self):
        """A freshly constructed list should be empty."""
        lst = ImmutableList()
        assert lst.len() == 0
        assert lst.is_empty() is True

    def test_from_list_empty(self):
        """from_list([]) should produce an empty list."""
        lst = ImmutableList.from_list([])
        assert lst.len() == 0
        assert lst.is_empty() is True

    def test_from_list_with_items(self):
        """from_list should create a list with the given elements."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert lst.len() == 3
        assert lst.get(0) == "a"
        assert lst.get(1) == "b"
        assert lst.get(2) == "c"

    def test_from_list_preserves_order(self):
        """Elements should appear in the same order as the input."""
        items = [f"item_{i}" for i in range(20)]
        lst = ImmutableList.from_list(items)
        for i, expected in enumerate(items):
            assert lst.get(i) == expected


# =========================================================================
# 2. Push and structural sharing
# =========================================================================

class TestPush:
    """Test the push method and verify immutability."""

    def test_push_returns_new_list(self):
        """push() must return a new list; the original stays unchanged."""
        a = ImmutableList()
        b = a.push("hello")
        assert a.len() == 0  # original unchanged
        assert b.len() == 1
        assert b.get(0) == "hello"

    def test_push_chain(self):
        """Chaining pushes builds up the list incrementally."""
        lst = ImmutableList()
        lst = lst.push("x")
        lst = lst.push("y")
        lst = lst.push("z")
        assert lst.len() == 3
        assert lst.get(0) == "x"
        assert lst.get(1) == "y"
        assert lst.get(2) == "z"

    def test_structural_sharing(self):
        """
        Pushing on a clone (or just keeping a reference) should NOT
        affect the original. This verifies structural sharing works
        correctly -- both lists share internal trie nodes but have
        independent identities.
        """
        original = ImmutableList.from_list(["a", "b", "c"])
        extended = original.push("d")

        # original is untouched
        assert original.len() == 3
        assert original.to_list() == ["a", "b", "c"]

        # extended has the new element
        assert extended.len() == 4
        assert extended.to_list() == ["a", "b", "c", "d"]

    def test_multiple_branches_from_same_base(self):
        """
        Multiple pushes from the same base list create independent branches.
        This is the fundamental persistence property.
        """
        base = ImmutableList.from_list(["root"])
        branch1 = base.push("left")
        branch2 = base.push("right")

        assert base.len() == 1
        assert branch1.len() == 2
        assert branch2.len() == 2

        assert branch1.get(1) == "left"
        assert branch2.get(1) == "right"

        # base is still the same
        assert base.to_list() == ["root"]


# =========================================================================
# 3. Get with positive and negative indices
# =========================================================================

class TestGet:
    """Test the get() method."""

    def test_get_valid_indices(self):
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert lst.get(0) == "a"
        assert lst.get(1) == "b"
        assert lst.get(2) == "c"

    def test_get_out_of_bounds_returns_none(self):
        """get() with an out-of-bounds index should return None."""
        lst = ImmutableList.from_list(["a"])
        assert lst.get(5) is None

    def test_get_negative_index(self):
        """get() supports negative indices (Python convention)."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert lst.get(-1) == "c"
        assert lst.get(-2) == "b"
        assert lst.get(-3) == "a"

    def test_get_negative_out_of_bounds(self):
        """Negative index past the beginning should return None."""
        lst = ImmutableList.from_list(["a", "b"])
        assert lst.get(-3) is None
        assert lst.get(-100) is None

    def test_get_on_empty(self):
        """get() on an empty list should return None."""
        lst = ImmutableList()
        assert lst.get(0) is None


# =========================================================================
# 4. Set
# =========================================================================

class TestSet:
    """Test the set() method (returns a new list with one element changed)."""

    def test_set_returns_new_list(self):
        """set() must return a new list; the original stays unchanged."""
        original = ImmutableList.from_list(["a", "b", "c"])
        modified = original.set(1, "B")
        assert original.get(1) == "b"  # original unchanged
        assert modified.get(1) == "B"  # modified has new value
        assert modified.len() == 3

    def test_set_first_element(self):
        lst = ImmutableList.from_list(["x", "y"])
        result = lst.set(0, "X")
        assert result.to_list() == ["X", "y"]

    def test_set_last_element(self):
        lst = ImmutableList.from_list(["x", "y"])
        result = lst.set(1, "Y")
        assert result.to_list() == ["x", "Y"]

    def test_set_out_of_bounds_raises(self):
        """set() with an out-of-bounds index should raise."""
        lst = ImmutableList.from_list(["a"])
        with pytest.raises(Exception):
            lst.set(5, "oops")

    def test_set_negative_index(self):
        """set() should support negative indices."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        result = lst.set(-1, "C")
        assert result.to_list() == ["a", "b", "C"]


# =========================================================================
# 5. Pop
# =========================================================================

class TestPop:
    """Test the pop() method (returns tuple of new list and removed value)."""

    def test_pop_returns_tuple(self):
        """pop() returns a (new_list, value) tuple."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        result = lst.pop()
        assert isinstance(result, tuple)
        assert len(result) == 2

    def test_pop_value(self):
        """pop() returns the last element as the second tuple element."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        new_list, value = lst.pop()
        assert value == "c"
        assert new_list.len() == 2
        assert new_list.to_list() == ["a", "b"]

    def test_pop_original_unchanged(self):
        """pop() must not modify the original list."""
        lst = ImmutableList.from_list(["a", "b"])
        new_list, _ = lst.pop()
        assert lst.len() == 2   # original unchanged
        assert new_list.len() == 1

    def test_pop_to_empty(self):
        """Popping the last element should yield an empty list."""
        lst = ImmutableList.from_list(["only"])
        new_list, value = lst.pop()
        assert value == "only"
        assert new_list.len() == 0
        assert new_list.is_empty() is True

    def test_pop_empty_raises(self):
        """pop() on an empty list should raise ImmutableListError."""
        lst = ImmutableList()
        with pytest.raises(ImmutableListError):
            lst.pop()


# =========================================================================
# 6. Large list (stress test)
# =========================================================================

class TestLargeList:
    """Test with 1000+ elements to exercise trie promotion."""

    def test_large_push_and_access(self):
        """Push 1500 elements and verify they are all accessible."""
        lst = ImmutableList()
        for i in range(1500):
            lst = lst.push(str(i))

        assert lst.len() == 1500
        # Spot-check various positions
        assert lst.get(0) == "0"
        assert lst.get(31) == "31"   # end of first tail
        assert lst.get(32) == "32"   # start of second block
        assert lst.get(999) == "999"
        assert lst.get(1499) == "1499"

    def test_large_from_list(self):
        """from_list with 1000 items should work correctly."""
        items = [f"elem_{i}" for i in range(1000)]
        lst = ImmutableList.from_list(items)
        assert lst.len() == 1000
        assert lst.get(0) == "elem_0"
        assert lst.get(999) == "elem_999"

    def test_large_to_list_roundtrip(self):
        """to_list should return all elements in order."""
        items = [str(i) for i in range(500)]
        lst = ImmutableList.from_list(items)
        assert lst.to_list() == items

    def test_large_pop_chain(self):
        """Pop all elements from a large list."""
        items = [str(i) for i in range(100)]
        lst = ImmutableList.from_list(items)
        collected = []
        while not lst.is_empty():
            lst, val = lst.pop()
            collected.append(val)
        # pop removes from the end, so collected is reversed
        collected.reverse()
        assert collected == items


# =========================================================================
# 7. Iterator protocol
# =========================================================================

class TestIterator:
    """Test __iter__ protocol."""

    def test_iter_empty(self):
        """Iterating an empty list should yield nothing."""
        lst = ImmutableList()
        assert list(lst) == []

    def test_iter_elements(self):
        """Iterating should yield all elements in order."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert list(lst) == ["a", "b", "c"]

    def test_iter_for_loop(self):
        """for-in loop should work correctly."""
        lst = ImmutableList.from_list(["x", "y", "z"])
        collected = []
        for item in lst:
            collected.append(item)
        assert collected == ["x", "y", "z"]

    def test_iter_large(self):
        """Iterator should handle large lists."""
        items = [str(i) for i in range(200)]
        lst = ImmutableList.from_list(items)
        assert list(lst) == items

    def test_iter_multiple_times(self):
        """Iterating the same list twice should work (list is immutable)."""
        lst = ImmutableList.from_list(["a", "b"])
        first = list(lst)
        second = list(lst)
        assert first == second == ["a", "b"]


# =========================================================================
# 8. __getitem__ with negative indices
# =========================================================================

class TestGetItem:
    """Test __getitem__ (bracket notation)."""

    def test_getitem_positive(self):
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert lst[0] == "a"
        assert lst[1] == "b"
        assert lst[2] == "c"

    def test_getitem_negative(self):
        """Negative indices should count from the end."""
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert lst[-1] == "c"
        assert lst[-2] == "b"
        assert lst[-3] == "a"

    def test_getitem_out_of_bounds_raises(self):
        """Out-of-bounds access via [] should raise IndexError."""
        lst = ImmutableList.from_list(["a"])
        with pytest.raises(IndexError):
            _ = lst[5]

    def test_getitem_negative_out_of_bounds_raises(self):
        lst = ImmutableList.from_list(["a", "b"])
        with pytest.raises(IndexError):
            _ = lst[-3]


# =========================================================================
# 9. __len__
# =========================================================================

class TestLen:
    """Test __len__ (Python's len() function)."""

    def test_len_empty(self):
        assert len(ImmutableList()) == 0

    def test_len_with_items(self):
        lst = ImmutableList.from_list(["a", "b", "c"])
        assert len(lst) == 3

    def test_len_after_push(self):
        lst = ImmutableList().push("x").push("y")
        assert len(lst) == 2


# =========================================================================
# 10. __eq__ and __repr__
# =========================================================================

class TestEqRepr:
    """Test equality comparison and string representation."""

    def test_eq_same_elements(self):
        """Lists with same elements should be equal."""
        a = ImmutableList.from_list(["a", "b", "c"])
        b = ImmutableList.from_list(["a", "b", "c"])
        assert a == b

    def test_eq_different_elements(self):
        """Lists with different elements should not be equal."""
        a = ImmutableList.from_list(["a", "b"])
        b = ImmutableList.from_list(["a", "c"])
        assert a != b

    def test_eq_different_lengths(self):
        a = ImmutableList.from_list(["a"])
        b = ImmutableList.from_list(["a", "b"])
        assert a != b

    def test_eq_both_empty(self):
        assert ImmutableList() == ImmutableList()

    def test_eq_not_a_list(self):
        """Comparing with a non-ImmutableList should return NotImplemented / False."""
        lst = ImmutableList.from_list(["a"])
        assert (lst == "not a list") is False
        assert (lst == 42) is False

    def test_repr_empty(self):
        r = repr(ImmutableList())
        assert "ImmutableList" in r
        assert "[]" in r

    def test_repr_with_items(self):
        lst = ImmutableList.from_list(["hello", "world"])
        r = repr(lst)
        assert "ImmutableList" in r
        assert "hello" in r
        assert "world" in r


# =========================================================================
# 11. Edge cases
# =========================================================================

class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_empty_to_list(self):
        assert ImmutableList().to_list() == []

    def test_single_element(self):
        lst = ImmutableList().push("only")
        assert lst.len() == 1
        assert lst.get(0) == "only"
        assert lst.to_list() == ["only"]

    def test_push_empty_string(self):
        lst = ImmutableList().push("")
        assert lst.get(0) == ""
        assert lst.len() == 1

    def test_push_unicode(self):
        """Unicode strings should work correctly."""
        lst = ImmutableList().push("hello").push("world")
        assert lst.get(0) == "hello"
        assert lst.get(1) == "world"

    def test_set_preserves_other_elements(self):
        """set() should only change the target element."""
        lst = ImmutableList.from_list(["a", "b", "c", "d", "e"])
        modified = lst.set(2, "C")
        assert modified.to_list() == ["a", "b", "C", "d", "e"]

    def test_is_empty_after_pop_to_empty(self):
        lst = ImmutableList().push("x")
        new_lst, _ = lst.pop()
        assert new_lst.is_empty() is True
        assert new_lst.len() == 0


# =========================================================================
# 12. from_list class method
# =========================================================================

class TestFromList:
    """Test the from_list class method."""

    def test_from_list_single(self):
        lst = ImmutableList.from_list(["solo"])
        assert lst.len() == 1
        assert lst.get(0) == "solo"

    def test_from_list_many(self):
        items = [chr(ord('a') + i) for i in range(26)]
        lst = ImmutableList.from_list(items)
        assert lst.len() == 26
        assert lst.to_list() == items

    def test_from_list_roundtrip(self):
        """from_list -> to_list should be identity."""
        items = ["one", "two", "three"]
        assert ImmutableList.from_list(items).to_list() == items
