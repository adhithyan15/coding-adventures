"""Tests for aot_core.link — linker for per-function binaries."""

from __future__ import annotations

from aot_core.link import entry_point_offset, link


class TestLink:
    def test_single_fn(self):
        code, offsets = link([("main", b"\x01\x02\x03")])
        assert code == b"\x01\x02\x03"
        assert offsets["main"] == 0

    def test_two_fns_concatenated(self):
        binaries = [("main", b"\x01\x02"), ("helper", b"\x03\x04\x05")]
        code, offsets = link(binaries)
        assert code == b"\x01\x02\x03\x04\x05"
        assert offsets["main"] == 0
        assert offsets["helper"] == 2

    def test_three_fns(self):
        binaries = [("a", b"\xAA"), ("b", b"\xBB\xBB"), ("c", b"\xCC\xCC\xCC")]
        code, offsets = link(binaries)
        assert code == b"\xAA\xBB\xBB\xCC\xCC\xCC"
        assert offsets["a"] == 0
        assert offsets["b"] == 1
        assert offsets["c"] == 3

    def test_empty_list(self):
        code, offsets = link([])
        assert code == b""
        assert offsets == {}

    def test_empty_binary_fn(self):
        code, offsets = link([("main", b""), ("helper", b"\xFF")])
        assert code == b"\xFF"
        assert offsets["main"] == 0
        assert offsets["helper"] == 0

    def test_offset_values_match_cumulative_sizes(self):
        sizes = [3, 5, 2]
        binaries = [(str(i), bytes(n)) for i, n in enumerate(sizes)]
        code, offsets = link(binaries)
        assert len(code) == sum(sizes)
        expected_offsets = [0, 3, 8]
        for i, off in enumerate(expected_offsets):
            assert offsets[str(i)] == off


class TestEntryPointOffset:
    def test_main_present(self):
        _, offsets = link([("helper", b"\xAA\xBB"), ("main", b"\xCC")])
        assert entry_point_offset(offsets) == 2

    def test_main_absent_returns_zero(self):
        _, offsets = link([("helper", b"\xAA")])
        assert entry_point_offset(offsets) == 0

    def test_custom_entry(self):
        _, offsets = link([("init", b"\x01"), ("start", b"\x02")])
        assert entry_point_offset(offsets, entry="start") == 1

    def test_empty_offsets(self):
        assert entry_point_offset({}) == 0
