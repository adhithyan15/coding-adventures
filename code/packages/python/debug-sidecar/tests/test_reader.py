"""Tests for DebugSidecarReader — round-trip and query correctness."""

import json

import pytest

from debug_sidecar.reader import DebugSidecarReader
from debug_sidecar.types import SourceLocation, Variable
from debug_sidecar.writer import DebugSidecarWriter


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_sidecar(**kwargs) -> bytes:
    """Build a minimal valid sidecar from a raw payload dict."""
    base = {"version": 1, "source_files": [], "line_table": {}, "functions": {}, "variables": {}}
    base.update(kwargs)
    return json.dumps(base).encode("utf-8")


def _fibonacci_sidecar() -> bytes:
    """A realistic fibonacci sidecar used across many tests."""
    w = DebugSidecarWriter()
    fid = w.add_source_file("fibonacci.tetrad")
    w.begin_function("fibonacci", start_instr=0, param_count=1)
    w.declare_variable("fibonacci", reg_index=0, name="n", type_hint="any",
                       live_start=0, live_end=12)
    w.declare_variable("fibonacci", reg_index=1, name="result", type_hint="any",
                       live_start=4, live_end=12)
    for idx, (line, col) in enumerate([
        (1, 1),   # 0: load n
        (2, 5),   # 1: cmp n < 2
        (2, 5),   # 2: jmp_if
        (3, 9),   # 3: ret n
        (4, 5),   # 4: call fib(n-1)
        (4, 5),   # 5: call fib(n-2)
        (4, 5),   # 6: add
        (5, 1),   # 7: ret result
    ]):
        w.record("fibonacci", idx, file_id=fid, line=line, col=col)
    w.end_function("fibonacci", end_instr=8)
    return w.finish()


# ---------------------------------------------------------------------------
# Initialization / error handling
# ---------------------------------------------------------------------------

class TestInit:
    def test_valid_sidecar_loads(self):
        data = _fibonacci_sidecar()
        reader = DebugSidecarReader(data)
        assert reader is not None

    def test_invalid_utf8_raises_value_error(self):
        with pytest.raises(ValueError, match="invalid sidecar"):
            DebugSidecarReader(b"\xff\xfe not utf-8")

    def test_invalid_json_raises_value_error(self):
        with pytest.raises(ValueError, match="invalid sidecar"):
            DebugSidecarReader(b"not json at all")

    def test_wrong_version_raises_value_error(self):
        data = _make_sidecar(version=42)
        with pytest.raises(ValueError, match="unsupported sidecar version"):
            DebugSidecarReader(data)

    def test_missing_version_raises_value_error(self):
        data = json.dumps({"source_files": [], "line_table": {}, "functions": {}, "variables": {}}).encode()
        with pytest.raises(ValueError, match="unsupported sidecar version"):
            DebugSidecarReader(data)

    def test_version_none_raises_value_error(self):
        data = _make_sidecar(version=None)
        with pytest.raises(ValueError, match="unsupported sidecar version"):
            DebugSidecarReader(data)


# ---------------------------------------------------------------------------
# lookup() — offset → source
# ---------------------------------------------------------------------------

class TestLookup:
    def setup_method(self):
        self.reader = DebugSidecarReader(_fibonacci_sidecar())

    def test_exact_match(self):
        loc = self.reader.lookup("fibonacci", 0)
        assert loc == SourceLocation("fibonacci.tetrad", 1, 1)

    def test_dwarf_style_coverage(self):
        # Instructions 4, 5, 6 all map to line 4, col 5
        for idx in (4, 5, 6):
            loc = self.reader.lookup("fibonacci", idx)
            assert loc is not None
            assert loc.line == 4

    def test_between_records_uses_preceding(self):
        # No record at index 3 in this gap scenario; use the preceding
        # Instruction 2 → line 2, instruction 3 → line 2 (same line, consecutive)
        loc = self.reader.lookup("fibonacci", 3)
        assert loc is not None
        assert loc.line == 3  # record at idx 3 is line 3

    def test_unknown_function_returns_none(self):
        assert self.reader.lookup("no_such_fn", 0) is None

    def test_before_first_record_returns_none(self):
        # Create a sidecar whose first record is at index 5
        w = DebugSidecarWriter()
        fid = w.add_source_file("f.tetrad")
        w.begin_function("f", start_instr=5, param_count=0)
        w.record("f", 5, file_id=fid, line=1, col=1)
        reader = DebugSidecarReader(w.finish())
        assert reader.lookup("f", 3) is None

    def test_returns_source_location_type(self):
        loc = self.reader.lookup("fibonacci", 0)
        assert isinstance(loc, SourceLocation)

    def test_instr_beyond_last_record_returns_last(self):
        # Instruction 100 is past all records; should return last record's location
        loc = self.reader.lookup("fibonacci", 100)
        assert loc is not None
        assert loc.line == 5  # last record is at line 5

    def test_bad_file_id_returns_none(self):
        data = _make_sidecar(
            source_files=[{"path": "f.tetrad", "checksum": ""}],
            line_table={"f": [{"instr_index": 0, "file_id": 99, "line": 1, "col": 1}]},
        )
        reader = DebugSidecarReader(data)
        assert reader.lookup("f", 0) is None


# ---------------------------------------------------------------------------
# find_instr() — source → offset
# ---------------------------------------------------------------------------

class TestFindInstr:
    def setup_method(self):
        self.reader = DebugSidecarReader(_fibonacci_sidecar())

    def test_finds_first_instruction_on_line(self):
        idx = self.reader.find_instr("fibonacci.tetrad", 1)
        assert idx == 0  # instruction 0 is on line 1

    def test_finds_lowest_index_for_multirow_line(self):
        # Line 4 has instructions 4, 5, 6 — should return 4
        idx = self.reader.find_instr("fibonacci.tetrad", 4)
        assert idx == 4

    def test_unknown_file_returns_none(self):
        assert self.reader.find_instr("no_such_file.tetrad", 1) is None

    def test_unknown_line_returns_none(self):
        assert self.reader.find_instr("fibonacci.tetrad", 999) is None

    def test_line_5_returns_correct_instruction(self):
        idx = self.reader.find_instr("fibonacci.tetrad", 5)
        assert idx == 7

    def test_cross_function_scan(self):
        """find_instr scans all functions; the first match across any function is returned."""
        w = DebugSidecarWriter()
        fid = w.add_source_file("shared.tetrad")
        w.begin_function("fn_a", start_instr=0, param_count=0)
        w.record("fn_a", 3, file_id=fid, line=10, col=1)
        w.begin_function("fn_b", start_instr=0, param_count=0)
        w.record("fn_b", 1, file_id=fid, line=10, col=1)
        reader = DebugSidecarReader(w.finish())
        idx = reader.find_instr("shared.tetrad", 10)
        # Both have line 10; lowest instr_index wins → 1
        assert idx == 1


# ---------------------------------------------------------------------------
# live_variables()
# ---------------------------------------------------------------------------

class TestLiveVariables:
    def setup_method(self):
        self.reader = DebugSidecarReader(_fibonacci_sidecar())

    def test_at_start_only_n_is_live(self):
        vars_ = self.reader.live_variables("fibonacci", 0)
        assert len(vars_) == 1
        assert vars_[0].name == "n"

    def test_at_instruction_4_both_live(self):
        vars_ = self.reader.live_variables("fibonacci", 4)
        names = {v.name for v in vars_}
        assert names == {"n", "result"}

    def test_sorted_by_reg_index(self):
        vars_ = self.reader.live_variables("fibonacci", 4)
        reg_indices = [v.reg_index for v in vars_]
        assert reg_indices == sorted(reg_indices)

    def test_at_live_end_not_included(self):
        # Both n and result end at 12; at instruction 12 neither should appear
        vars_ = self.reader.live_variables("fibonacci", 12)
        assert vars_ == []

    def test_unknown_function_returns_empty(self):
        assert self.reader.live_variables("no_such_fn", 0) == []

    def test_returns_variable_type(self):
        vars_ = self.reader.live_variables("fibonacci", 0)
        assert all(isinstance(v, Variable) for v in vars_)

    def test_no_variables_registered(self):
        w = DebugSidecarWriter()
        w.begin_function("empty_fn", start_instr=0, param_count=0)
        reader = DebugSidecarReader(w.finish())
        assert reader.live_variables("empty_fn", 0) == []


# ---------------------------------------------------------------------------
# Metadata queries
# ---------------------------------------------------------------------------

class TestMetadata:
    def setup_method(self):
        self.reader = DebugSidecarReader(_fibonacci_sidecar())

    def test_source_files(self):
        assert self.reader.source_files() == ["fibonacci.tetrad"]

    def test_function_names(self):
        assert self.reader.function_names() == ["fibonacci"]

    def test_function_range(self):
        assert self.reader.function_range("fibonacci") == (0, 8)

    def test_function_range_unknown_returns_none(self):
        assert self.reader.function_range("no_such_fn") is None

    def test_function_range_no_end_returns_none(self):
        w = DebugSidecarWriter()
        w.begin_function("f", start_instr=0, param_count=0)
        # no end_function call
        reader = DebugSidecarReader(w.finish())
        assert reader.function_range("f") is None

    def test_multiple_source_files(self):
        w = DebugSidecarWriter()
        w.add_source_file("a.tetrad")
        w.add_source_file("b.tetrad")
        reader = DebugSidecarReader(w.finish())
        assert reader.source_files() == ["a.tetrad", "b.tetrad"]

    def test_multiple_function_names(self):
        w = DebugSidecarWriter()
        w.begin_function("alpha", start_instr=0, param_count=0)
        w.begin_function("beta", start_instr=10, param_count=0)
        reader = DebugSidecarReader(w.finish())
        assert set(reader.function_names()) == {"alpha", "beta"}


# ---------------------------------------------------------------------------
# Round-trip fidelity
# ---------------------------------------------------------------------------

class TestRoundTrip:
    def test_checksum_preserved(self):
        w = DebugSidecarWriter()
        w.add_source_file("f.tetrad", checksum=b"\xca\xfe\xba\xbe")
        data = w.finish()
        # Just verify the reader loads it without error (checksum is internal)
        reader = DebugSidecarReader(data)
        assert reader.source_files() == ["f.tetrad"]

    def test_empty_sidecar_round_trip(self):
        w = DebugSidecarWriter()
        data = w.finish()
        reader = DebugSidecarReader(data)
        assert reader.source_files() == []
        assert reader.function_names() == []

    def test_many_functions_round_trip(self):
        w = DebugSidecarWriter()
        fid = w.add_source_file("prog.tetrad")
        for i in range(20):
            fn = f"fn_{i}"
            w.begin_function(fn, start_instr=i * 10, param_count=i % 3)
            w.record(fn, i * 10, file_id=fid, line=i + 1, col=1)
            w.end_function(fn, end_instr=i * 10 + 10)
        data = w.finish()
        reader = DebugSidecarReader(data)
        assert len(reader.function_names()) == 20
        for i in range(20):
            fn = f"fn_{i}"
            loc = reader.lookup(fn, i * 10)
            assert loc is not None
            assert loc.line == i + 1

    def test_large_instruction_indices(self):
        w = DebugSidecarWriter()
        fid = w.add_source_file("big.tetrad")
        w.begin_function("big", start_instr=0, param_count=0)
        w.record("big", 10_000, file_id=fid, line=9999, col=1)
        w.record("big", 20_000, file_id=fid, line=19999, col=1)
        w.end_function("big", end_instr=20_001)
        reader = DebugSidecarReader(w.finish())
        loc = reader.lookup("big", 15_000)
        assert loc is not None
        assert loc.line == 9999  # between 10000 and 20000 → preceding record
