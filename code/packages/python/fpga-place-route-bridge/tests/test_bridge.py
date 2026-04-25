"""Tests for fpga-place-route-bridge."""

from gate_netlist_format import (
    Direction,
    Instance,
    Level,
    Module,
    Net,
    NetSlice,
    Netlist,
    Port,
)
from fpga_place_route_bridge import (
    TRUTH_TABLES,
    FpgaBridgeOptions,
    hnl_to_fpga_json,
)


def make_inverter_netlist() -> Netlist:
    m = Module(
        name="inv_top",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        instances=[
            Instance(
                name="u_inv",
                cell_type="NOT",
                connections={
                    "A": NetSlice("a", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            ),
        ],
    )
    return Netlist(top="inv_top", modules={"inv_top": m}, level=Level.GENERIC)


def make_and2_netlist() -> Netlist:
    m = Module(
        name="and_top",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("b", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        instances=[
            Instance(
                name="u",
                cell_type="AND2",
                connections={
                    "A": NetSlice("a", (0,)),
                    "B": NetSlice("b", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            ),
        ],
    )
    return Netlist(top="and_top", modules={"and_top": m}, level=Level.GENERIC)


# ---- Truth tables ----


def test_truth_tables_present():
    expected = {
        "BUF", "NOT", "AND2", "OR2", "NAND2", "NOR2", "XOR2",
        "AND3", "OR3", "NAND3", "NOR3", "XOR3",
        "AND4", "OR4", "NAND4", "NOR4",
        "MUX2", "CONST_0", "CONST_1",
    }
    assert expected.issubset(set(TRUTH_TABLES.keys()))


def test_and2_truth_table_correct():
    pins, table = TRUTH_TABLES["AND2"]
    assert pins == ["A", "B"]
    # AND truth: 00->0, 01->0, 10->0, 11->1
    assert table == [0, 0, 0, 1]


def test_xor2_truth_table_correct():
    pins, table = TRUTH_TABLES["XOR2"]
    assert table == [0, 1, 1, 0]


def test_const_0_table():
    pins, table = TRUTH_TABLES["CONST_0"]
    assert pins == []
    assert table == [0]


def test_const_1_table():
    pins, table = TRUTH_TABLES["CONST_1"]
    assert table == [1]


# ---- hnl_to_fpga_json ----


def test_inverter_packs_one_clb():
    nl = make_inverter_netlist()
    cfg, report = hnl_to_fpga_json(nl)
    assert report.cells_packed == 1
    assert len(cfg["clbs"]) == 1


def test_and2_packs_one_clb():
    nl = make_and2_netlist()
    cfg, report = hnl_to_fpga_json(nl)
    assert report.cells_packed == 1
    # Truth table expanded to 4-input (16 entries).
    clb_0_0 = cfg["clbs"]["clb_0_0"]
    assert len(clb_0_0["lut_a"]["truth_table"]) == 16


def test_unmapped_cells_reported():
    m = Module(
        name="x",
        instances=[
            Instance(name="u", cell_type="MYSTERY", connections={}),
        ],
    )
    nl = Netlist(top="x", modules={"x": m}, level=Level.GENERIC)
    _, report = hnl_to_fpga_json(nl)
    assert "MYSTERY" in report.cells_unmapped
    assert report.cells_packed == 0


def test_io_pins_emitted():
    nl = make_and2_netlist()
    cfg, _ = hnl_to_fpga_json(nl)
    pin_names = {pin["name"] for pin in cfg["io"].values()}
    assert "a" in pin_names
    assert "b" in pin_names
    assert "y" in pin_names


def test_routes_emitted():
    nl = make_and2_netlist()
    cfg, report = hnl_to_fpga_json(nl)
    assert report.routes_emitted >= 2  # at least A and B inputs
    # At least one route from net source to LUT input
    sources = {r["from"] for r in cfg["routing"]}
    assert any("net_a" in s or "io_pin_a" in s or "a" in s for s in sources)


def test_truth_table_expansion_size():
    """4-input LUT should always have 16 entries regardless of cell input count."""
    nl = make_inverter_netlist()  # 1-input cell
    cfg, _ = hnl_to_fpga_json(nl, options=FpgaBridgeOptions(lut_inputs=4))
    assert len(cfg["clbs"]["clb_0_0"]["lut_a"]["truth_table"]) == 16


def test_options_passed_through():
    nl = make_and2_netlist()
    cfg, _ = hnl_to_fpga_json(
        nl, options=FpgaBridgeOptions(rows=8, cols=8, lut_inputs=6),
    )
    assert cfg["device"]["rows"] == 8
    assert cfg["device"]["cols"] == 8
    assert cfg["device"]["lut_inputs"] == 6


# ---- Multiple cells ----


def test_multiple_cells_get_different_clbs():
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("b", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        nets=[Net("mid", 1)],
        instances=[
            Instance(
                name="u1", cell_type="AND2",
                connections={"A": NetSlice("a", (0,)), "B": NetSlice("b", (0,)),
                             "Y": NetSlice("mid", (0,))},
            ),
            Instance(
                name="u2", cell_type="NOT",
                connections={"A": NetSlice("mid", (0,)), "Y": NetSlice("y", (0,))},
            ),
        ],
    )
    nl = Netlist(top="x", modules={"x": m}, level=Level.GENERIC)
    cfg, report = hnl_to_fpga_json(nl)
    assert report.cells_packed == 2
    assert len(cfg["clbs"]) == 2
    assert "clb_0_0" in cfg["clbs"]
    assert "clb_0_1" in cfg["clbs"]


# ---- 4-bit adder smoke ----


def test_4bit_adder_packs():
    """A 4-bit adder synthesizes to ~20 cells; verify they all pack."""
    # Build a simplified version with 8 XOR2 + 8 AND2 + 4 OR2
    m = Module(
        name="adder4",
        ports=[
            Port("a", Direction.INPUT, 4),
            Port("b", Direction.INPUT, 4),
            Port("sum", Direction.OUTPUT, 4),
            Port("cout", Direction.OUTPUT, 1),
        ],
        instances=[
            *[Instance(
                f"x{i}", "XOR2",
                {"A": NetSlice("a", (i % 4,)), "B": NetSlice("b", (i % 4,)),
                 "Y": NetSlice("sum", (i % 4,))},
            ) for i in range(8)],
            *[Instance(
                f"and{i}", "AND2",
                {"A": NetSlice("a", (i % 4,)), "B": NetSlice("b", (i % 4,)),
                 "Y": NetSlice("cout", (0,))},
            ) for i in range(8)],
            *[Instance(
                f"or{i}", "OR2",
                {"A": NetSlice("a", (i,)), "B": NetSlice("b", (i,)),
                 "Y": NetSlice("cout", (0,))},
            ) for i in range(4)],
        ],
    )
    nl = Netlist(top="adder4", modules={"adder4": m}, level=Level.GENERIC)
    cfg, report = hnl_to_fpga_json(
        nl, options=FpgaBridgeOptions(rows=8, cols=8),
    )
    assert report.cells_packed == 20
    assert len(cfg["clbs"]) == 20
