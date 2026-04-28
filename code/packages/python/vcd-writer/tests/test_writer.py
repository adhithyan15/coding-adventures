"""Tests for VcdWriter."""

from pathlib import Path

import pytest

from vcd_writer import VcdWriter, attach_to_callback_emitter


# ---- Header generation ----


def test_basic_header(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p, timescale="1ns") as w:
        w.end_definitions()
    text = p.read_text()
    assert "$date" in text
    assert "$timescale 1ns $end" in text
    assert "$enddefinitions $end" in text


def test_scope_hierarchy(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        w.open_scope("u_dut", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.close_scope()
        w.end_definitions()
    text = p.read_text()
    assert "$scope module top $end" in text
    assert "$scope module u_dut $end" in text
    assert text.count("$upscope $end") == 2
    assert f"$var wire 1 {a} a $end" in text


def test_multi_bit_var_format(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        sum_id = w.declare("sum", 4)
        w.close_scope()
        w.end_definitions()
    text = p.read_text()
    assert f"$var wire 4 {sum_id} sum [3:0] $end" in text


def test_declare_after_end_raises(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.end_definitions()
        with pytest.raises(RuntimeError, match="end_definitions"):
            w.declare("a", 1)


def test_declare_zero_width_raises(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        with pytest.raises(ValueError, match="width"):
            w.declare("a", 0)


def test_close_scope_without_open_raises(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        with pytest.raises(RuntimeError, match="without matching"):
            w.close_scope()


def test_open_scope_after_end_raises(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.end_definitions()
        with pytest.raises(RuntimeError, match="end_definitions"):
            w.open_scope("top")


# ---- Identifier compaction ----


def test_id_uniqueness_first_100(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        ids = [w.declare(f"v{i}", 1) for i in range(100)]
        w.end_definitions()
    assert len(set(ids)) == 100  # all unique


def test_id_compaction_first_94_single_char(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        for i in range(94):
            id_ = w.declare(f"v{i}", 1)
            assert len(id_) == 1
        # 95th should still be 1 char (with carry overflow into 2nd)
        # Actually our impl rolls over to 2-char at index 94
        id95 = w.declare("v94", 1)
        assert len(id95) == 2
        w.end_definitions()


# ---- Value changes ----


def test_scalar_value_change(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, a, 1)
    text = p.read_text()
    assert "#10" in text
    assert f"1{a}" in text


def test_vector_value_change(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        sum_id = w.declare("sum", 4)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, sum_id, 0xA)  # 1010
    text = p.read_text()
    assert "#10" in text
    assert f"b1010 {sum_id}" in text


def test_repeated_value_no_emit(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, a, 1)
        w.value_change(20, a, 1)  # unchanged - should not emit value line
    text = p.read_text()
    # We should see #10 and the value, but #20 may or may not be emitted
    # Either way, the value line should appear only once.
    assert text.count(f"1{a}") == 1


def test_time_monotonic(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, a, 1)
        with pytest.raises(ValueError, match="must not decrease"):
            w.value_change(5, a, 0)


def test_unknown_var_id(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.end_definitions()
        with pytest.raises(KeyError, match="unknown var_id"):
            w.value_change(10, "!", 1)


# ---- Initial dump ----


def test_dump_initial(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        b = w.declare("b", 4)
        w.close_scope()
        w.end_definitions()
        w.dump_initial({a: 1, b: 0xC})
    text = p.read_text()
    assert "$dumpvars" in text
    assert "$end" in text
    assert f"1{a}" in text
    assert f"b1100 {b}" in text


def test_dump_initial_default_values(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.end_definitions()
        w.dump_initial({})
    text = p.read_text()
    assert f"0{a}" in text  # default 0


# ---- 4-state x/z values ----


def test_x_value_scalar(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, a, "x")
    text = p.read_text()
    assert f"x{a}" in text


def test_xz_string_in_vector(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        b = w.declare("b", 4)
        w.close_scope()
        w.end_definitions()
        w.value_change(10, b, "10xz")
    text = p.read_text()
    assert f"b10xz {b}" in text


# ---- Real value ----


def test_real_value(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        v = w.declare("voltage", 64, kind="real")
        w.close_scope()
        w.end_definitions()
        w.value_change(10, v, 1.5)
    text = p.read_text()
    assert f"r1.5 {v}" in text


# ---- attach_to_callback_emitter ----


class FakeEvent:
    def __init__(self, time: int, signal: str, new_value: int):
        self.time = time
        self.signal = signal
        self.new_value = new_value


def test_attach_to_callback_emitter(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        a = w.declare("a", 1)
        sum_id = w.declare("sum", 4)
        w.close_scope()
        w.end_definitions()

        cb = attach_to_callback_emitter(w, name_to_var_id={"a": a, "sum": sum_id})
        cb(FakeEvent(time=10, signal="a", new_value=1))
        cb(FakeEvent(time=20, signal="sum", new_value=0xA))
        # Unknown signal should be silently dropped
        cb(FakeEvent(time=30, signal="not_declared", new_value=99))

    text = p.read_text()
    assert "#10" in text
    assert "#20" in text
    assert f"1{a}" in text
    assert f"b1010 {sum_id}" in text


def test_attach_handles_non_event_objects(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.end_definitions()
        cb = attach_to_callback_emitter(w, name_to_var_id={})
        # No exception when called with random object
        cb(object())


# ---- Auto-close scopes on end ----


def test_auto_close_scopes_on_end_definitions(tmp_path: Path):
    p = tmp_path / "x.vcd"
    with VcdWriter(p) as w:
        w.open_scope("top", "module")
        w.declare("a", 1)
        # Don't close_scope explicitly; end_definitions does it for us.
        w.end_definitions()
    text = p.read_text()
    assert "$upscope $end" in text


# ---- Errors ----


def test_writing_when_not_open_raises(tmp_path: Path):
    p = tmp_path / "x.vcd"
    w = VcdWriter(p)
    # haven't called .open()
    with pytest.raises(RuntimeError, match="not open"):
        w._w("test")
