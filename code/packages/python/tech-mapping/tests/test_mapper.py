"""Tests for tech-mapping."""

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
from tech_mapping import DEFAULT_MAP, TechMapper, map_to_stdcell, push_bubbles


def make_simple_netlist() -> Netlist:
    """An AND2 + INV chain: a, b -> AND2.Y -> INV.A -> y."""
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("b", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        nets=[Net("ab", 1)],
        instances=[
            Instance(
                name="u_and",
                cell_type="AND2",
                connections={
                    "A": NetSlice("a", (0,)),
                    "B": NetSlice("b", (0,)),
                    "Y": NetSlice("ab", (0,)),
                },
            ),
            Instance(
                name="u_inv",
                cell_type="NOT",
                connections={
                    "A": NetSlice("ab", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            ),
        ],
    )
    return Netlist(top="x", modules={"x": m}, level=Level.GENERIC)


# ---- Cell-type renaming ----


def test_map_renames_cells():
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    types = [i.cell_type for i in mapped.modules["x"].instances]
    assert "and2_1" in types
    assert "inv_1" in types


def test_map_changes_level():
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    assert mapped.level == Level.STDCELL


def test_map_preserves_top():
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    assert mapped.top == nl.top


def test_map_preserves_ports_and_nets():
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    assert len(mapped.modules["x"].ports) == 3
    assert len(mapped.modules["x"].nets) == 1


# ---- Pin remap ----


def test_pin_remap_y_to_x_for_combinational():
    """HNL `Y` -> Sky130 `X` for combinational cells."""
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    and_inst = next(i for i in mapped.modules["x"].instances if i.cell_type == "and2_1")
    assert "X" in and_inst.connections
    assert "Y" not in and_inst.connections


def test_pin_remap_y_kept_for_inverting():
    """HNL `Y` -> Sky130 `Y` for inverting cells (NOT, NAND, etc.)."""
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    inv_inst = next(i for i in mapped.modules["x"].instances if i.cell_type == "inv_1")
    assert "Y" in inv_inst.connections


# ---- Mapping report ----


def test_report_counts():
    nl = make_simple_netlist()
    _, report = map_to_stdcell(nl)
    assert report.cells_before == 2
    assert report.cells_after == 2
    assert report.unmapped == []


def test_unmapped_cells_pass_through():
    """A user-defined module type stays unchanged."""
    m = Module(
        name="top",
        instances=[
            Instance(
                name="u",
                cell_type="my_user_module",
                connections={},
            ),
        ],
    )
    nl = Netlist(top="top", modules={"top": m}, level=Level.GENERIC)
    mapped, report = map_to_stdcell(nl)
    assert mapped.modules["top"].instances[0].cell_type == "my_user_module"
    assert "my_user_module" in report.unmapped


# ---- DEFAULT_MAP coverage ----


def test_default_map_covers_all_hnl_builtins():
    expected = {
        "BUF", "NOT", "AND2", "AND3", "AND4",
        "OR2", "OR3", "OR4",
        "NAND2", "NAND3", "NAND4",
        "NOR2", "NOR3", "NOR4",
        "XOR2", "XOR3", "XNOR2", "XNOR3",
        "MUX2", "DFF", "DFF_R", "DFF_S", "DFF_RS",
        "DLATCH", "TBUF", "CONST_0", "CONST_1",
    }
    assert expected.issubset(set(DEFAULT_MAP.keys()))


# ---- Custom mapping ----


def test_custom_cell_map():
    custom = {"AND2": ("custom_and", {"A": "in0", "B": "in1", "Y": "out"})}
    nl = make_simple_netlist()
    mapped, report = TechMapper(cell_map=custom).map(nl)
    and_inst = next(i for i in mapped.modules["x"].instances if i.name == "u_and")
    assert and_inst.cell_type == "custom_and"
    assert "in0" in and_inst.connections
    assert "out" in and_inst.connections
    # NOT was not in custom map; passes through as unmapped.
    assert "NOT" in report.unmapped


# ---- push_bubbles ----


def test_push_bubbles_cancels_inv_inv():
    """A -> INV -> INV -> Y should reduce to A -> Y (both INVs gone)."""
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        nets=[Net("mid", 1)],
        instances=[
            Instance(
                name="u_inv1",
                cell_type="inv_1",
                connections={
                    "A": NetSlice("a", (0,)),
                    "Y": NetSlice("mid", (0,)),
                },
            ),
            Instance(
                name="u_inv2",
                cell_type="inv_1",
                connections={
                    "A": NetSlice("mid", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            ),
            # A consumer of inv2's output, reading net "y"
            Instance(
                name="u_consumer",
                cell_type="buf_1",
                connections={
                    "A": NetSlice("y", (0,)),
                    "X": NetSlice("a", (0,)),  # arbitrary
                },
            ),
        ],
    )
    nl = Netlist(top="x", modules={"x": m}, level=Level.STDCELL)
    optimized, count = push_bubbles(nl)
    # Both INVs cancelled
    inv_count = sum(1 for i in optimized.modules["x"].instances if i.cell_type == "inv_1")
    assert inv_count == 0
    assert count == 1


def test_push_bubbles_no_cancellation():
    """A single INV shouldn't cancel."""
    m = Module(
        name="x",
        ports=[
            Port("a", Direction.INPUT, 1),
            Port("y", Direction.OUTPUT, 1),
        ],
        instances=[
            Instance(
                name="u_inv",
                cell_type="inv_1",
                connections={
                    "A": NetSlice("a", (0,)),
                    "Y": NetSlice("y", (0,)),
                },
            ),
        ],
    )
    nl = Netlist(top="x", modules={"x": m}, level=Level.STDCELL)
    _, count = push_bubbles(nl)
    assert count == 0


def test_push_bubbles_preserves_other_cells():
    """Non-INV cells are untouched."""
    nl = make_simple_netlist()
    mapped, _ = map_to_stdcell(nl)
    optimized, _ = push_bubbles(mapped)
    types = {i.cell_type for i in optimized.modules["x"].instances}
    # AND2 still there
    assert "and2_1" in types


# ---- 4-bit-adder smoke ----


def test_4bit_adder_mapping_smoke():
    """Synthesized adder has 8 XOR2 + 8 AND2 + 4 OR2 cells; verify mapping
    rewrites all of them."""
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
                f"a{i}", "AND2",
                {"A": NetSlice("a", (i % 4,)), "B": NetSlice("b", (i % 4,)),
                 "Y": NetSlice("cout", (0,))},
            ) for i in range(8)],
            *[Instance(
                f"o{i}", "OR2",
                {"A": NetSlice("a", (i,)), "B": NetSlice("b", (i,)),
                 "Y": NetSlice("cout", (0,))},
            ) for i in range(4)],
        ],
    )
    nl = Netlist(top="adder4", modules={"adder4": m}, level=Level.GENERIC)

    mapped, report = map_to_stdcell(nl)
    types = [i.cell_type for i in mapped.modules["adder4"].instances]
    assert sum(1 for t in types if t == "xor2_1") == 8
    assert sum(1 for t in types if t == "and2_1") == 8
    assert sum(1 for t in types if t == "or2_1") == 4
    assert mapped.level == Level.STDCELL
    assert report.unmapped == []
