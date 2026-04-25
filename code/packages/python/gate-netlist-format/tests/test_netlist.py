"""Tests for HNL data model + JSON + validation."""

from pathlib import Path

import pytest

from gate_netlist_format import (
    BUILTIN_CELL_TYPES,
    Direction,
    Instance,
    Level,
    Module,
    Net,
    NetSlice,
    Netlist,
    Port,
)


# ---- Constructors ----


def test_port_round_trip():
    p = Port("a", Direction.INPUT, 4)
    assert Port.from_dict(p.to_dict()) == p


def test_port_zero_width_rejected():
    with pytest.raises(ValueError, match="width"):
        Port("a", Direction.INPUT, 0)


def test_net_round_trip():
    n = Net("c0", 1)
    assert Net.from_dict(n.to_dict()) == n


def test_net_zero_width_rejected():
    with pytest.raises(ValueError, match="width"):
        Net("x", 0)


def test_net_slice_round_trip():
    s = NetSlice("data", (0, 1, 2, 3))
    rt = NetSlice.from_dict(s.to_dict())
    assert rt == s
    assert rt.width() == 4


def test_instance_round_trip():
    inst = Instance(
        name="u_fa",
        cell_type="full_adder",
        connections={"a": NetSlice("x", (0,)), "b": NetSlice("y", (0,))},
        parameters={"WIDTH": 8},
    )
    rt = Instance.from_dict(inst.to_dict())
    assert rt == inst


# ---- Module + Netlist ----


def make_adder4_netlist() -> Netlist:
    fa = Module(
        name="full_adder",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("b", Direction.INPUT, 1),
            Port("cin", Direction.INPUT, 1),
            Port("sum", Direction.OUTPUT, 1),
            Port("cout", Direction.OUTPUT, 1),
        ],
        instances=[
            Instance(
                name="u_xor",
                cell_type="XOR2",
                connections={
                    "A": NetSlice("a", (0,)),
                    "B": NetSlice("b", (0,)),
                    "Y": NetSlice("sum", (0,)),
                },
            )
        ],
    )
    top = Module(
        name="adder4",
        ports=[
            Port("a", Direction.INPUT, 4),
            Port("b", Direction.INPUT, 4),
            Port("cin", Direction.INPUT, 1),
            Port("sum", Direction.OUTPUT, 4),
            Port("cout", Direction.OUTPUT, 1),
        ],
        nets=[Net("c0", 1), Net("c1", 1), Net("c2", 1)],
        instances=[
            Instance(
                name="u_fa0",
                cell_type="full_adder",
                connections={
                    "a": NetSlice("a", (0,)),
                    "b": NetSlice("b", (0,)),
                    "cin": NetSlice("cin", (0,)),
                    "sum": NetSlice("sum", (0,)),
                    "cout": NetSlice("c0", (0,)),
                },
            ),
        ],
    )
    return Netlist(top="adder4", modules={"adder4": top, "full_adder": fa})


def test_module_helpers():
    nl = make_adder4_netlist()
    top = nl.modules["adder4"]
    assert top.port("a") is not None
    assert top.port("nonexistent") is None
    assert top.net("c0") is not None
    assert top.net("nonexistent") is None


def test_netlist_round_trip():
    nl = make_adder4_netlist()
    rt = Netlist.from_dict(nl.to_dict())
    assert rt.top == nl.top
    assert set(rt.modules) == set(nl.modules)


def test_netlist_json_round_trip(tmp_path: Path):
    nl = make_adder4_netlist()
    p = tmp_path / "x.json"
    nl.to_json(p)
    rt = Netlist.from_json(p)
    assert rt.top == nl.top


def test_netlist_format_marker():
    nl = make_adder4_netlist()
    d = nl.to_dict()
    assert d["format"] == "HNL"
    assert d["version"] == "0.1.0"


def test_netlist_rejects_non_hnl():
    with pytest.raises(ValueError, match="not an HNL"):
        Netlist.from_dict({"format": "other", "top": "x", "modules": {}})


def test_netlist_rejects_major_version_mismatch():
    bad = {"format": "HNL", "version": "99.0.0", "top": "x", "modules": {}}
    with pytest.raises(ValueError, match="major version"):
        Netlist.from_dict(bad)


def test_stats():
    nl = make_adder4_netlist()
    s = nl.stats()
    assert s.total_cells == 2  # u_fa0 in adder4 + u_xor in full_adder
    assert "full_adder" in s.cell_counts
    assert "XOR2" in s.cell_counts


# ---- Built-in cell types ----


def test_builtin_cell_types_complete():
    expected = {
        "BUF", "NOT", "AND2", "AND3", "AND4",
        "OR2", "OR3", "OR4",
        "NAND2", "NAND3", "NAND4",
        "NOR2", "NOR3", "NOR4",
        "XOR2", "XOR3", "XNOR2", "XNOR3",
        "MUX2", "DFF", "DFF_R", "DFF_S", "DFF_RS",
        "DLATCH", "TBUF", "CONST_0", "CONST_1",
    }
    assert expected.issubset(set(BUILTIN_CELL_TYPES.keys()))


def test_dff_signature():
    sig = BUILTIN_CELL_TYPES["DFF"]
    assert sig.inputs == ("D", "CLK")
    assert sig.outputs == ("Q",)
    assert sig.has_pin("D")
    assert sig.has_pin("Q")
    assert not sig.has_pin("X")


def test_const_0_signature():
    sig = BUILTIN_CELL_TYPES["CONST_0"]
    assert sig.inputs == ()
    assert sig.outputs == ("Y",)


# ---- Validation R1: top exists ----


def test_r1_top_exists():
    nl = make_adder4_netlist()
    assert nl.validate().ok


def test_r1_missing_top():
    nl = Netlist(top="missing", modules={})
    r = nl.validate()
    assert any("R1" in e for e in r.errors)


# ---- R2: cell type resolves ----


def test_r2_unknown_cell_type():
    bad = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 1), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="MYSTERY_CELL",
                connections={"A": NetSlice("a", (0,)), "Y": NetSlice("y", (0,))},
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": bad})
    r = nl.validate()
    assert any("R2" in e for e in r.errors)


def test_r2_resolves_to_user_module():
    nl = make_adder4_netlist()
    r = nl.validate()
    # full_adder is a user module; should resolve cleanly.
    assert all("R2" not in e for e in r.errors)


# ---- R3: input pins must be connected ----


def test_r3_input_pin_not_connected():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 1), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="AND2",
                connections={"A": NetSlice("a", (0,)), "Y": NetSlice("y", (0,))},
                # Missing B!
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("R3" in e and "B" in e for e in r.errors)


# ---- R4: connection keys are real pins ----


def test_r4_unknown_pin():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 1), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="NOT",
                connections={
                    "A": NetSlice("a", (0,)),
                    "Y": NetSlice("y", (0,)),
                    "FAKE_PIN": NetSlice("a", (0,)),
                },
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("R4" in e and "FAKE_PIN" in e for e in r.errors)


# ---- R5: width match ----


def test_r5_width_mismatch():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 4), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="NOT",
                connections={
                    "A": NetSlice("a", (0, 1, 2, 3)),  # NOT.A is 1-bit; width 4 mismatch
                    "Y": NetSlice("y", (0,)),
                },
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("R5" in e for e in r.errors)


# ---- R6: net referenced exists ----


def test_r6_unknown_net():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 1), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="NOT",
                connections={
                    "A": NetSlice("nonexistent", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("R6" in e for e in r.errors)


# ---- R7: bits in range ----


def test_r7_bit_out_of_range():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 4), Port("y", Direction.OUTPUT, 1)],
        instances=[
            Instance(
                name="u",
                cell_type="NOT",
                connections={
                    "A": NetSlice("a", (10,)),  # bit 10 out of 4-bit range
                    "Y": NetSlice("y", (0,)),
                },
            )
        ],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("R7" in e for e in r.errors)


# ---- R11: no self-instantiation ----


def test_r11_direct_self_loop():
    m = Module(
        name="m",
        instances=[Instance(name="self", cell_type="m", connections={})],
    )
    nl = Netlist(top="m", modules={"m": m})
    r = nl.validate()
    assert any("R11" in e for e in r.errors)


def test_r11_indirect_loop():
    a = Module(name="a", instances=[Instance(name="ub", cell_type="b", connections={})])
    b = Module(name="b", instances=[Instance(name="ua", cell_type="a", connections={})])
    nl = Netlist(top="a", modules={"a": a, "b": b})
    r = nl.validate()
    assert any("R11" in e for e in r.errors)


# ---- Duplicate names ----


def test_duplicate_port_names():
    m = Module(
        name="x",
        ports=[Port("a", Direction.INPUT, 1), Port("a", Direction.OUTPUT, 1)],
    )
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("duplicate port" in e for e in r.errors)


def test_duplicate_net_names():
    m = Module(name="x", nets=[Net("c", 1), Net("c", 1)])
    nl = Netlist(top="x", modules={"x": m})
    r = nl.validate()
    assert any("duplicate net" in e for e in r.errors)
