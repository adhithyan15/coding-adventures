"""Tests for DebugSidecarWriter."""

import json

import pytest

from debug_sidecar.writer import DebugSidecarWriter


def _parse(data: bytes) -> dict:
    return json.loads(data.decode("utf-8"))


class TestAddSourceFile:
    def test_returns_zero_for_first_file(self):
        w = DebugSidecarWriter()
        assert w.add_source_file("foo.tetrad") == 0

    def test_returns_sequential_ids(self):
        w = DebugSidecarWriter()
        assert w.add_source_file("a.tetrad") == 0
        assert w.add_source_file("b.tetrad") == 1
        assert w.add_source_file("c.tetrad") == 2

    def test_duplicate_path_returns_same_id(self):
        w = DebugSidecarWriter()
        id1 = w.add_source_file("foo.tetrad")
        id2 = w.add_source_file("foo.tetrad")
        assert id1 == id2 == 0

    def test_checksum_stored_as_hex(self):
        w = DebugSidecarWriter()
        w.add_source_file("foo.tetrad", checksum=b"\xde\xad\xbe\xef")
        payload = _parse(w.finish())
        assert payload["source_files"][0]["checksum"] == "deadbeef"

    def test_empty_checksum(self):
        w = DebugSidecarWriter()
        w.add_source_file("foo.tetrad")
        payload = _parse(w.finish())
        assert payload["source_files"][0]["checksum"] == ""

    def test_path_preserved(self):
        w = DebugSidecarWriter()
        w.add_source_file("/abs/path/to/prog.tetrad")
        payload = _parse(w.finish())
        assert payload["source_files"][0]["path"] == "/abs/path/to/prog.tetrad"


class TestRecord:
    def test_record_creates_line_table_entry(self):
        w = DebugSidecarWriter()
        fid = w.add_source_file("fib.tetrad")
        w.begin_function("fib", start_instr=0, param_count=1)
        w.record("fib", 0, file_id=fid, line=1, col=5)
        payload = _parse(w.finish())
        rows = payload["line_table"]["fib"]
        assert len(rows) == 1
        assert rows[0] == {"instr_index": 0, "file_id": 0, "line": 1, "col": 5}

    def test_rows_sorted_by_instr_index(self):
        w = DebugSidecarWriter()
        fid = w.add_source_file("f.tetrad")
        w.begin_function("f", start_instr=0, param_count=0)
        # Record out of order
        w.record("f", 5, file_id=fid, line=6, col=1)
        w.record("f", 2, file_id=fid, line=3, col=1)
        w.record("f", 8, file_id=fid, line=9, col=1)
        w.record("f", 0, file_id=fid, line=1, col=1)
        payload = _parse(w.finish())
        indices = [r["instr_index"] for r in payload["line_table"]["f"]]
        assert indices == sorted(indices)

    def test_multiple_functions(self):
        w = DebugSidecarWriter()
        fid = w.add_source_file("prog.tetrad")
        w.begin_function("main", start_instr=0, param_count=0)
        w.record("main", 0, file_id=fid, line=1, col=1)
        w.begin_function("helper", start_instr=10, param_count=1)
        w.record("helper", 0, file_id=fid, line=5, col=1)
        payload = _parse(w.finish())
        assert "main" in payload["line_table"]
        assert "helper" in payload["line_table"]

    def test_no_record_no_line_table(self):
        w = DebugSidecarWriter()
        payload = _parse(w.finish())
        assert payload["line_table"] == {}


class TestFunctions:
    def test_begin_function_registers_start(self):
        w = DebugSidecarWriter()
        w.begin_function("fib", start_instr=0, param_count=1)
        payload = _parse(w.finish())
        fn = payload["functions"]["fib"]
        assert fn["start_instr"] == 0
        assert fn["param_count"] == 1
        assert fn["end_instr"] is None

    def test_end_function_sets_end(self):
        w = DebugSidecarWriter()
        w.begin_function("fib", start_instr=0, param_count=1)
        w.end_function("fib", end_instr=12)
        payload = _parse(w.finish())
        assert payload["functions"]["fib"]["end_instr"] == 12

    def test_end_function_no_begin_is_noop(self):
        w = DebugSidecarWriter()
        w.end_function("ghost", end_instr=5)
        payload = _parse(w.finish())
        assert "ghost" not in payload["functions"]


class TestVariables:
    def test_declare_variable_stored(self):
        w = DebugSidecarWriter()
        w.begin_function("fib", start_instr=0, param_count=1)
        w.declare_variable("fib", reg_index=0, name="n", type_hint="u8",
                           live_start=0, live_end=12)
        payload = _parse(w.finish())
        vars_ = payload["variables"]["fib"]
        assert len(vars_) == 1
        assert vars_[0] == {
            "reg_index": 0, "name": "n", "type_hint": "u8",
            "live_start": 0, "live_end": 12,
        }

    def test_multiple_variables(self):
        w = DebugSidecarWriter()
        w.begin_function("f", start_instr=0, param_count=2)
        w.declare_variable("f", reg_index=0, name="a", type_hint="any",
                           live_start=0, live_end=5)
        w.declare_variable("f", reg_index=1, name="b", type_hint="any",
                           live_start=0, live_end=5)
        payload = _parse(w.finish())
        assert len(payload["variables"]["f"]) == 2

    def test_empty_type_hint_default(self):
        w = DebugSidecarWriter()
        w.begin_function("f", start_instr=0, param_count=1)
        w.declare_variable("f", reg_index=0, name="x", live_start=0, live_end=3)
        payload = _parse(w.finish())
        assert payload["variables"]["f"][0]["type_hint"] == ""


class TestFinish:
    def test_version_is_one(self):
        w = DebugSidecarWriter()
        payload = _parse(w.finish())
        assert payload["version"] == 1

    def test_returns_bytes(self):
        w = DebugSidecarWriter()
        result = w.finish()
        assert isinstance(result, bytes)

    def test_valid_utf8_json(self):
        w = DebugSidecarWriter()
        data = w.finish()
        parsed = json.loads(data.decode("utf-8"))
        assert isinstance(parsed, dict)

    def test_empty_writer_still_valid(self):
        w = DebugSidecarWriter()
        payload = _parse(w.finish())
        assert payload["source_files"] == []
        assert payload["line_table"] == {}
        assert payload["functions"] == {}
        assert payload["variables"] == {}
