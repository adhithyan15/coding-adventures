"""Tests for LEF/DEF emission."""

from pathlib import Path

from lef_def import (
    CellLef,
    Component,
    Def,
    DefPin,
    Direction,
    LayerDef,
    Net,
    PinDef,
    PinPort,
    Rect,
    Row,
    Segment,
    SiteDef,
    TechLef,
    Use,
    ViaDef,
    ViaLayer,
    write_cells_lef,
    write_cells_lef_str,
    write_def,
    write_def_str,
    write_tech_lef,
    write_tech_lef_str,
)


# ---- Tech LEF ----


def test_tech_lef_basic_header():
    tech = TechLef()
    s = write_tech_lef_str(tech)
    assert "VERSION 5.8 ;" in s
    assert "DATABASE MICRONS 1000" in s
    assert "END LIBRARY" in s


def test_tech_lef_with_layer():
    tech = TechLef()
    tech.layers.append(LayerDef("met1", "ROUTING", direction="HORIZONTAL",
                                 pitch=0.34, width=0.14, spacing=0.14))
    s = write_tech_lef_str(tech)
    assert "LAYER met1" in s
    assert "TYPE ROUTING" in s
    assert "DIRECTION HORIZONTAL" in s
    assert "PITCH 0.34" in s
    assert "END met1" in s


def test_tech_lef_with_via():
    tech = TechLef()
    tech.vias.append(ViaDef(
        name="via0",
        is_default=True,
        layers=(
            ViaLayer("met1", Rect(-0.085, -0.085, 0.085, 0.085)),
            ViaLayer("met2", Rect(-0.085, -0.085, 0.085, 0.085)),
        ),
    ))
    s = write_tech_lef_str(tech)
    assert "VIA via0 DEFAULT" in s
    assert "LAYER met1" in s
    assert "RECT -0.085 -0.085 0.085 0.085" in s


def test_tech_lef_with_site():
    tech = TechLef()
    tech.sites.append(SiteDef("unithd", "CORE", width=0.46, height=2.72))
    s = write_tech_lef_str(tech)
    assert "SITE unithd" in s
    assert "CLASS CORE" in s
    assert "SIZE 0.46 BY 2.72" in s


def test_tech_lef_to_file(tmp_path: Path):
    tech = TechLef()
    p = tmp_path / "x.lef"
    write_tech_lef(tech, p)
    assert "VERSION 5.8" in p.read_text()


# ---- Cells LEF ----


def test_cells_lef_basic():
    cell = CellLef(
        name="nand2_1",
        class_="CORE",
        width=1.38,
        height=2.72,
        site="unithd",
        pins=[
            PinDef(
                "A", Direction.INPUT, Use.SIGNAL,
                ports=(PinPort("li1", Rect(0.1, 0.1, 0.3, 0.3)),),
            ),
            PinDef(
                "Y", Direction.OUTPUT, Use.SIGNAL,
                ports=(PinPort("li1", Rect(1.0, 1.0, 1.2, 1.2)),),
            ),
        ],
    )
    s = write_cells_lef_str([cell])
    assert "MACRO nand2_1" in s
    assert "CLASS CORE" in s
    assert "SIZE 1.38 BY 2.72" in s
    assert "PIN A" in s
    assert "DIRECTION INPUT" in s
    assert "USE SIGNAL" in s
    assert "PIN Y" in s
    assert "DIRECTION OUTPUT" in s
    assert "END nand2_1" in s


def test_cells_lef_with_obs():
    cell = CellLef(
        name="nand2_1",
        width=1.38,
        height=2.72,
        site="unithd",
        obs=[("li1", Rect(0.4, 0.5, 0.6, 0.7))],
    )
    s = write_cells_lef_str([cell])
    assert "OBS" in s
    assert "RECT 0.4 0.5 0.6 0.7" in s


def test_cells_lef_with_foreign():
    cell = CellLef(name="x", foreign="external_x")
    s = write_cells_lef_str([cell])
    assert "FOREIGN external_x" in s


def test_cells_lef_to_file(tmp_path: Path):
    cell = CellLef(name="x")
    p = tmp_path / "cells.lef"
    write_cells_lef([cell], p)
    assert "MACRO x" in p.read_text()


# ---- DEF ----


def test_def_minimal():
    d = Def(design="adder4")
    s = write_def_str(d)
    assert "VERSION 5.8 ;" in s
    assert "DESIGN adder4 ;" in s
    assert "UNITS DISTANCE MICRONS 1000" in s
    assert "END DESIGN" in s


def test_def_with_diearea():
    d = Def(design="adder4", die_area=Rect(0, 0, 100, 50))
    s = write_def_str(d)
    assert "DIEAREA ( 0 0 ) ( 100 50 ) ;" in s


def test_def_with_row():
    d = Def(design="adder4")
    d.rows.append(Row("row1", "unithd", 0, 0, "N", 217, 1, 0.46, 0))
    s = write_def_str(d)
    assert "ROW row1 unithd 0 0 N DO 217 BY 1 STEP 0.46 0" in s


def test_def_with_placed_component():
    d = Def(design="adder4")
    d.components.append(
        Component("u_fa0", "nand2_1", placed=True, location_x=10, location_y=2.72)
    )
    s = write_def_str(d)
    assert "COMPONENTS 1 ;" in s
    assert "u_fa0 nand2_1 + PLACED ( 10 2.72 ) N" in s
    assert "END COMPONENTS" in s


def test_def_with_unplaced_component():
    d = Def(design="adder4")
    d.components.append(Component("u", "cell"))
    s = write_def_str(d)
    assert "u cell ;" in s
    assert "PLACED" not in s


def test_def_with_pin():
    d = Def(design="adder4")
    d.pins.append(
        DefPin("a[0]", "a[0]", Direction.INPUT, Use.SIGNAL,
               layer="met2", rect=Rect(-0.1, 1.0, 0.0, 1.2))
    )
    s = write_def_str(d)
    assert "PINS 1 ;" in s
    assert "a[0] + NET a[0] + DIRECTION INPUT" in s
    assert "+ USE SIGNAL" in s
    assert "+ LAYER met2" in s
    assert "( -0.1 1.0 ) ( 0.0 1.2 )" in s
    assert "END PINS" in s


def test_def_with_pin_no_layer():
    d = Def(design="adder4")
    d.pins.append(
        DefPin("clk", "clk", Direction.INPUT, Use.SIGNAL)
    )
    s = write_def_str(d)
    assert "DIRECTION INPUT" in s
    assert "LAYER" not in s


def test_def_with_net():
    d = Def(design="adder4")
    d.nets.append(Net("c0", connections=[("u_fa0", "Y"), ("u_fa1", "A")]))
    s = write_def_str(d)
    assert "NETS 1 ;" in s
    assert "( u_fa0 Y )" in s
    assert "( u_fa1 A )" in s


def test_def_with_routed_net():
    d = Def(design="adder4")
    d.nets.append(Net(
        "c0",
        connections=[("u_fa0", "Y")],
        routed_segments=[Segment("met1", ((1.0, 1.0), (2.0, 1.0)))],
    ))
    s = write_def_str(d)
    assert "ROUTED met1 ( 1.0 1.0 ) ( 2.0 1.0 )" in s


def test_def_to_file(tmp_path: Path):
    d = Def(design="adder4")
    p = tmp_path / "x.def"
    write_def(d, p)
    assert "DESIGN adder4" in p.read_text()


# ---- Round-trip-ish: emitted LEF/DEF have sensible structure ----


def test_full_lef_def_workflow_for_adder4(tmp_path: Path):
    """End-to-end: emit a tech LEF, cells LEF, and DEF for the 4-bit adder
    smoke test."""
    # Tech
    tech = TechLef()
    tech.layers.append(LayerDef("met1", "ROUTING", direction="HORIZONTAL",
                                 pitch=0.34, width=0.14, spacing=0.14))
    tech.sites.append(SiteDef("unithd", "CORE", width=0.46, height=2.72))
    write_tech_lef(tech, tmp_path / "tech.lef")

    # Cells
    cells = [
        CellLef(
            name="nand2_1", width=1.38, height=2.72, site="unithd",
            pins=[
                PinDef("A", Direction.INPUT, Use.SIGNAL,
                       ports=(PinPort("li1", Rect(0.1, 1.0, 0.3, 1.3)),)),
                PinDef("Y", Direction.OUTPUT, Use.SIGNAL,
                       ports=(PinPort("li1", Rect(0.8, 1.0, 1.2, 1.3)),)),
            ],
        ),
    ]
    write_cells_lef(cells, tmp_path / "cells.lef")

    # DEF
    d = Def(design="adder4", die_area=Rect(0, 0, 100, 50))
    d.rows.append(Row("row1", "unithd", 0, 0, "N", 217, 1, 0.46, 0))
    for i in range(4):
        d.components.append(
            Component(f"u_fa{i}", "nand2_1",
                      placed=True, location_x=i * 1.5, location_y=0)
        )
    write_def(d, tmp_path / "adder4.def")

    # All three files were written
    assert (tmp_path / "tech.lef").exists()
    assert (tmp_path / "cells.lef").exists()
    assert (tmp_path / "adder4.def").exists()
