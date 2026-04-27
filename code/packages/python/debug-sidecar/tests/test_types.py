"""Tests for debug_sidecar.types — SourceLocation and Variable."""

import pytest

from debug_sidecar.types import SourceLocation, Variable


class TestSourceLocation:
    def test_str_representation(self):
        loc = SourceLocation(file="foo.tetrad", line=10, col=3)
        assert str(loc) == "foo.tetrad:10:3"

    def test_frozen(self):
        loc = SourceLocation(file="foo.tetrad", line=1, col=1)
        with pytest.raises(Exception):  # FrozenInstanceError
            loc.line = 99  # type: ignore[misc]

    def test_hashable(self):
        loc1 = SourceLocation(file="a.tetrad", line=5, col=1)
        loc2 = SourceLocation(file="a.tetrad", line=5, col=1)
        assert loc1 == loc2
        assert hash(loc1) == hash(loc2)

    def test_different_locations_not_equal(self):
        loc1 = SourceLocation(file="a.tetrad", line=1, col=1)
        loc2 = SourceLocation(file="a.tetrad", line=2, col=1)
        assert loc1 != loc2

    def test_usable_in_set(self):
        locs = {
            SourceLocation("a.tetrad", 1, 1),
            SourceLocation("a.tetrad", 1, 1),
            SourceLocation("b.tetrad", 2, 3),
        }
        assert len(locs) == 2


class TestVariable:
    def test_is_live_at_start(self):
        v = Variable(reg_index=0, name="x", type_hint="u8", live_start=3, live_end=10)
        assert v.is_live_at(3)

    def test_is_live_at_middle(self):
        v = Variable(reg_index=0, name="x", type_hint="u8", live_start=3, live_end=10)
        assert v.is_live_at(7)

    def test_not_live_at_end(self):
        # live_end is exclusive
        v = Variable(reg_index=0, name="x", type_hint="u8", live_start=3, live_end=10)
        assert not v.is_live_at(10)

    def test_not_live_before_start(self):
        v = Variable(reg_index=0, name="x", type_hint="u8", live_start=3, live_end=10)
        assert not v.is_live_at(2)

    def test_empty_type_hint(self):
        v = Variable(reg_index=1, name="n", type_hint="", live_start=0, live_end=5)
        assert v.type_hint == ""

    def test_frozen(self):
        v = Variable(reg_index=0, name="x", type_hint="any", live_start=0, live_end=5)
        with pytest.raises(Exception):
            v.name = "y"  # type: ignore[misc]

    def test_single_instruction_range(self):
        v = Variable(reg_index=0, name="tmp", type_hint="any", live_start=5, live_end=6)
        assert v.is_live_at(5)
        assert not v.is_live_at(6)
        assert not v.is_live_at(4)
