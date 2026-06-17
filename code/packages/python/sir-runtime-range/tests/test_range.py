"""Unit tests for the SIR Python range runtime."""

from __future__ import annotations

from itertools import islice

import pytest

from coding_adventures_sir_runtime_range import (
    Range,
    includes,
    is_range,
    to_list,
)
from coding_adventures_sir_runtime_range import (
    range as sir_range,
)


class TestConstructor:
    def test_range_builds_a_range_with_fields(self) -> None:
        r = sir_range(1, 5, False)
        assert isinstance(r, Range)
        assert r.start == 1
        assert r.stop == 5
        assert r.exclusive is False

    def test_exclusive_flag_is_coerced_to_bool(self) -> None:
        # A None "exclusive" (the inclusive `..` form passes a falsey value)
        # becomes a real False rather than staying None.
        assert sir_range(1, 5, None).exclusive is False
        assert sir_range(1, 5, 1).exclusive is True


class TestIteration:
    def test_inclusive_range_iterates_through_stop(self) -> None:
        assert list(sir_range(1, 5, False)) == [1, 2, 3, 4, 5]

    def test_exclusive_range_stops_before_stop(self) -> None:
        assert list(sir_range(1, 5, True)) == [1, 2, 3, 4]

    def test_single_element_inclusive_range(self) -> None:
        assert list(sir_range(3, 3, False)) == [3]

    def test_empty_exclusive_range(self) -> None:
        assert list(sir_range(3, 3, True)) == []

    def test_endless_range_yields_forever_lazily(self) -> None:
        # An endless range (stop=None) must yield forever; consume it lazily.
        r = sir_range(10, None, False)
        assert list(islice(r, 4)) == [10, 11, 12, 13]

    def test_beginless_range_cannot_be_iterated(self) -> None:
        with pytest.raises(TypeError, match="beginless"):
            list(sir_range(None, 5, False))


class TestMembership:
    def test_inclusive_membership(self) -> None:
        r = sir_range(1, 5, False)
        assert 1 in r
        assert 5 in r
        assert 0 not in r
        assert 6 not in r

    def test_exclusive_membership_excludes_stop(self) -> None:
        r = sir_range(1, 5, True)
        assert 4 in r
        assert 5 not in r

    def test_endless_membership(self) -> None:
        r = sir_range(10, None, False)
        assert 10 in r
        assert 1_000_000 in r
        assert 9 not in r

    def test_beginless_membership(self) -> None:
        r = sir_range(None, 5, False)
        assert -100 in r
        assert 5 in r
        assert 6 not in r

    def test_includes_free_function(self) -> None:
        assert includes(sir_range(1, 5, False), 3) is True
        assert includes(sir_range(1, 5, True), 5) is False


class TestToList:
    def test_to_list_materialises(self) -> None:
        assert to_list(sir_range(1, 4, False)) == [1, 2, 3, 4]
        assert sir_range(1, 4, True).to_list() == [1, 2, 3]

    def test_to_list_on_endless_raises(self) -> None:
        with pytest.raises(TypeError, match="endless"):
            sir_range(1, None, False).to_list()

    def test_to_list_on_beginless_raises(self) -> None:
        with pytest.raises(TypeError, match="beginless"):
            sir_range(None, 5, False).to_list()


class TestIsRange:
    def test_true_for_a_range(self) -> None:
        assert is_range(sir_range(1, 5, False)) is True

    def test_false_for_non_ranges(self) -> None:
        assert is_range(1) is False
        assert is_range(None) is False
        assert is_range([1, 2, 3]) is False


class TestEqualityAndHash:
    def test_equal_ranges_compare_equal_and_hash_equal(self) -> None:
        a = sir_range(1, 5, False)
        b = sir_range(1, 5, False)
        assert a == b
        assert hash(a) == hash(b)

    def test_differing_exclusive_flag_compares_unequal(self) -> None:
        assert sir_range(1, 5, False) != sir_range(1, 5, True)

    def test_range_not_equal_to_non_range(self) -> None:
        assert (sir_range(1, 5, False) == "1..5") is False


class TestRepr:
    def test_inclusive_repr(self) -> None:
        assert repr(sir_range(1, 5, False)) == "1..5"

    def test_exclusive_repr(self) -> None:
        assert repr(sir_range(1, 5, True)) == "1...5"

    def test_endless_repr(self) -> None:
        assert repr(sir_range(1, None, False)) == "1.."

    def test_beginless_repr(self) -> None:
        assert repr(sir_range(None, 5, False)) == "..5"
