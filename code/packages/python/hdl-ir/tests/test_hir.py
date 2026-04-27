"""Tests for top-level HIR container, the canonical 4-bit-adder example, and JSON round-trip."""

import json
from pathlib import Path

import pytest

from hdl_ir import (
    HIR,
    BinaryOp,
    Concat,
    ContAssign,
    Direction,
    Library,
    Module,
    Net,
    NetKind,
    Port,
    PortRef,
    Process,
    ProcessKind,
    Provenance,
    SourceLang,
    SourceLocation,
    TyLogic,
    TyVector,
)


def make_adder4() -> HIR:
    """Construct the canonical 4-bit adder HIR.

    Mirrors `hdl-ir.md` Worked Example 1 (Verilog-flavored)."""
    adder = Module(
        name="adder4",
        ports=[
            Port("a",   Direction.IN,  TyVector(TyLogic(), 4)),
            Port("b",   Direction.IN,  TyVector(TyLogic(), 4)),
            Port("cin", Direction.IN,  TyLogic()),
            Port("sum", Direction.OUT, TyVector(TyLogic(), 4)),
            Port("cout", Direction.OUT, TyLogic()),
        ],
        cont_assigns=[
            ContAssign(
                target=Concat((PortRef("cout"), PortRef("sum"))),
                rhs=BinaryOp(
                    "+",
                    BinaryOp("+", PortRef("a"), PortRef("b")),
                    PortRef("cin"),
                ),
            )
        ],
        provenance=Provenance(
            SourceLang.VERILOG, SourceLocation("adder4.v", 1, 1)
        ),
    )
    return HIR(top="adder4", modules={"adder4": adder})


# ---- Round-trip ----


def test_adder4_dict_round_trip():
    hir = make_adder4()
    rt = HIR.from_dict(hir.to_dict())
    assert rt.top == hir.top
    assert set(rt.modules) == {"adder4"}
    rt_mod = rt.modules["adder4"]
    src_mod = hir.modules["adder4"]
    assert rt_mod.name == src_mod.name
    assert len(rt_mod.ports) == len(src_mod.ports)
    assert rt_mod.cont_assigns == src_mod.cont_assigns


def test_adder4_json_string_round_trip():
    hir = make_adder4()
    rt = HIR.from_json_str(hir.to_json_str())
    assert rt.modules["adder4"].cont_assigns == hir.modules["adder4"].cont_assigns


def test_adder4_json_file_round_trip(tmp_path: Path):
    hir = make_adder4()
    p = tmp_path / "adder4.json"
    hir.to_json(p)
    rt = HIR.from_json(p)
    assert rt.top == hir.top


def test_json_format_marker_is_HIR():
    hir = make_adder4()
    d = hir.to_dict()
    assert d["format"] == "HIR"


def test_from_dict_rejects_non_hir():
    with pytest.raises(ValueError, match="not an HIR"):
        HIR.from_dict({"format": "something_else", "top": "x", "modules": {}})


def test_from_dict_rejects_major_version_mismatch():
    bad = {"format": "HIR", "version": "99.0.0", "top": "x", "modules": {"x": {"name": "x"}}}
    with pytest.raises(ValueError, match="major version"):
        HIR.from_dict(bad)


# ---- Stats ----


def test_stats_counts():
    hir = make_adder4()
    s = hir.stats()
    assert s.module_count == 1
    assert s.cont_assign_count == 1
    assert s.process_count == 0
    assert s.instance_count == 0


def test_stats_with_process():
    proc = Process(
        kind=ProcessKind.PROCESS,
        body=(),
    )
    mod = Module(name="m", processes=[proc])
    hir = HIR(top="m", modules={"m": mod})
    assert hir.stats().process_count == 1


# ---- Net + Variable ----


def test_net_round_trip_with_kind():
    n = Net("clk", TyLogic(), kind=NetKind.WIRE)
    rt = Net.from_dict(n.to_dict())
    assert rt == n


def test_net_with_initial_round_trip():
    from hdl_ir.expr import Lit

    n = Net("count", TyVector(TyLogic(), 4), kind=NetKind.REG, initial=Lit(0, TyLogic()))
    rt = Net.from_dict(n.to_dict())
    assert rt == n


# ---- Library ----


def test_library_round_trip():
    mod = Module(name="m")
    lib = Library(name="ieee", modules={"m": mod})
    hir = HIR(top="m", modules={"m": mod}, libraries={"ieee": lib})
    rt = HIR.from_dict(hir.to_dict())
    assert "ieee" in rt.libraries
    assert "m" in rt.libraries["ieee"].modules


# ---- Provenance ----


def test_provenance_preserved_through_json():
    hir = make_adder4()
    rt = HIR.from_json_str(hir.to_json_str())
    assert rt.modules["adder4"].provenance is not None
    assert rt.modules["adder4"].provenance.lang == SourceLang.VERILOG
    assert rt.modules["adder4"].provenance.location is not None
    assert rt.modules["adder4"].provenance.location.file == "adder4.v"


def test_source_location_invalid_line():
    with pytest.raises(ValueError, match="line"):
        SourceLocation("a.v", 0, 1)


def test_source_location_invalid_column():
    with pytest.raises(ValueError, match="column"):
        SourceLocation("a.v", 1, 0)


# ---- Module helpers ----


def test_module_find_port():
    hir = make_adder4()
    mod = hir.modules["adder4"]
    assert mod.find_port("a") is not None
    assert mod.find_port("not_a_port") is None


def test_module_find_net_when_present():
    mod = Module(name="m", nets=[Net("clk", TyLogic())])
    assert mod.find_net("clk") is not None
    assert mod.find_net("xyz") is None


# ---- JSON output is human-readable ----


def test_json_output_is_indented(tmp_path: Path):
    hir = make_adder4()
    p = tmp_path / "x.json"
    hir.to_json(p)
    text = p.read_text()
    # Should have at least one line break per top-level field; trivially true for indent=2
    assert "\n" in text
    parsed = json.loads(text)
    assert parsed["format"] == "HIR"
