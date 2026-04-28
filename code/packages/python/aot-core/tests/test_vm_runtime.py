"""Tests for aot_core.vm_runtime — IIR table serialisation."""

from __future__ import annotations

import json

from conftest import make_fn, make_instr
from interpreter_ir.function import FunctionTypeStatus

from aot_core.vm_runtime import VmRuntime


class TestVmRuntime:
    def test_default_is_empty(self):
        rt = VmRuntime()
        assert rt.is_empty

    def test_non_empty(self):
        rt = VmRuntime(b"\x01\x02")
        assert not rt.is_empty
        assert rt.library_bytes == b"\x01\x02"

    def test_serialise_empty_list(self):
        rt = VmRuntime()
        data = rt.serialise_iir_table([])
        records = json.loads(data.decode())
        assert records == []

    def test_serialise_single_fn(self):
        fn = make_fn(
            "foo",
            [("x", "u8")],
            make_instr("ret", srcs=["x"]),
        )
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        assert len(records) == 1
        assert records[0]["name"] == "foo"

    def test_serialise_params_preserved(self):
        fn = make_fn("bar", [("a", "u8"), ("b", "u16")])
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        assert records[0]["params"] == [["a", "u8"], ["b", "u16"]]

    def test_serialise_instructions_preserved(self):
        fn = make_fn(
            "baz", [],
            make_instr("const", "v", [42]),
            make_instr("ret", srcs=["v"]),
        )
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        instrs = records[0]["instructions"]
        assert len(instrs) == 2
        assert instrs[0]["op"] == "const"
        assert instrs[0]["dest"] == "v"
        assert instrs[0]["srcs"] == [42]
        assert instrs[1]["op"] == "ret"

    def test_serialise_multiple_fns(self):
        f1 = make_fn("f1", [])
        f2 = make_fn("f2", [("x", "u8")])
        rt = VmRuntime()
        data = rt.serialise_iir_table([f1, f2])
        records = json.loads(data.decode())
        assert len(records) == 2
        assert records[0]["name"] == "f1"
        assert records[1]["name"] == "f2"

    def test_deserialise_roundtrip(self):
        fn = make_fn(
            "roundtrip",
            [("x", "u8")],
            make_instr("const", "c", [99]),
            make_instr("add", "r", ["x", "c"]),
        )
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = rt.deserialise_iir_table(data)
        assert len(records) == 1
        assert records[0]["name"] == "roundtrip"
        assert len(records[0]["instructions"]) == 2

    def test_type_status_serialised(self):
        fn = make_fn("f", [], type_status=FunctionTypeStatus.UNTYPED)
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        assert records[0]["type_status"] == FunctionTypeStatus.UNTYPED.value

    def test_instr_with_none_dest(self):
        fn = make_fn("f", [("x", "u8")], make_instr("ret", srcs=["x"]))
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        instr = records[0]["instructions"][0]
        assert instr["dest"] is None

    def test_deopt_anchor_preserved(self):
        fn = make_fn(
            "f", [],
            make_instr("add", "r", ["a", "b"], deopt_anchor=3),
        )
        rt = VmRuntime()
        data = rt.serialise_iir_table([fn])
        records = json.loads(data.decode())
        assert records[0]["instructions"][0]["deopt_anchor"] == 3
