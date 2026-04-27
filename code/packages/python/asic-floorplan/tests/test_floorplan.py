"""Tests for ASIC floorplanning."""

import pytest

from asic_floorplan import (
    CellInstanceEstimate,
    IoSpec,
    compute_floorplan,
    floorplan_to_def,
)
from lef_def import Direction


# ---- compute_floorplan ----


def test_basic_floorplan():
    cells = [
        CellInstanceEstimate(f"u{i}", "nand2_1", area=3.75) for i in range(20)
    ]
    fp = compute_floorplan(
        cells=cells, site_height=2.72, site_width=0.46, site_name="unithd",
    )
    # Core area = sum / 0.7 = 75 / 0.7 ≈ 107
    # core_height² = 107, core_height ≈ 10.4 → snapped to multiple of 2.72 = 4 rows
    # core_height = 4 × 2.72 = 10.88
    assert len(fp.rows) >= 1
    assert fp.die.x2 - fp.die.x1 > 0
    assert fp.die.y2 - fp.die.y1 > 0


def test_die_includes_io_ring():
    cells = [CellInstanceEstimate("u", "x", area=10.0)]
    fp = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        io_ring_width=5.0,
    )
    # Die surrounds core by exactly io_ring_width on each side.
    assert fp.die.x1 == 0.0
    assert fp.die.y1 == 0.0
    assert fp.core.x1 == 5.0
    assert fp.core.y1 == 5.0
    assert fp.die.x2 - fp.core.x2 == pytest.approx(5.0)
    assert fp.die.y2 - fp.core.y2 == pytest.approx(5.0)


def test_rows_alternate_orientation():
    cells = [
        CellInstanceEstimate(f"u{i}", "x", area=2.0) for i in range(20)
    ]
    fp = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        utilization=0.5,
    )
    if len(fp.rows) >= 2:
        # Even rows are N, odd rows are FS
        assert fp.rows[0].orientation == "N"
        assert fp.rows[1].orientation == "FS"


def test_aspect_wide():
    cells = [CellInstanceEstimate(f"u{i}", "x", area=4.0) for i in range(10)]
    fp_square = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        aspect=1.0,
    )
    fp_wide = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        aspect=4.0,
    )
    # Wider aspect -> larger horizontal dimension, smaller vertical dimension
    assert (fp_wide.core.x2 - fp_wide.core.x1) > (fp_square.core.x2 - fp_square.core.x1)
    assert (fp_wide.core.y2 - fp_wide.core.y1) <= (fp_square.core.y2 - fp_square.core.y1)


def test_components_in_floorplan():
    cells = [CellInstanceEstimate(f"u{i}", f"cell_{i % 3}", area=1.0) for i in range(5)]
    fp = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
    )
    assert len(fp.components) == 5
    for c in fp.components:
        assert not c.placed  # placement happens later


# ---- IO pin placement ----


def test_io_pins_placed_on_edges():
    cells = [CellInstanceEstimate("u", "x", area=10.0)]
    io = [
        IoSpec("a", Direction.INPUT),
        IoSpec("b", Direction.INPUT),
        IoSpec("y", Direction.OUTPUT),
    ]
    fp = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        io_pins=io,
    )
    pin_names = {p.name for p in fp.pins}
    assert "a" in pin_names
    assert "b" in pin_names
    assert "y" in pin_names

    # Input pins on left edge (x near die.x1)
    a_pin = next(p for p in fp.pins if p.name == "a")
    assert a_pin.rect is not None
    assert a_pin.rect.x2 <= fp.die.x1 + 0.001  # at or just past left edge

    # Output pin on right edge
    y_pin = next(p for p in fp.pins if p.name == "y")
    assert y_pin.rect is not None
    assert y_pin.rect.x1 >= fp.die.x2 - 0.001


def test_io_pins_inout_on_bottom():
    cells = [CellInstanceEstimate("u", "x", area=10.0)]
    io = [IoSpec("clk", Direction.INOUT)]
    fp = compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        io_pins=io,
    )
    clk_pin = next(p for p in fp.pins if p.name == "clk")
    assert clk_pin.rect is not None
    # On the bottom edge
    assert clk_pin.rect.y2 <= fp.die.y1 + 0.001


def test_no_io_pins():
    fp = compute_floorplan(
        cells=[CellInstanceEstimate("u", "x", area=1.0)],
        site_height=1.0, site_width=1.0, site_name="x",
    )
    assert fp.pins == ()


# ---- Validation ----


def test_zero_utilization_rejected():
    with pytest.raises(ValueError, match="utilization"):
        compute_floorplan(
            cells=[CellInstanceEstimate("u", "x", area=1.0)],
            site_height=1.0, site_width=1.0, site_name="x",
            utilization=0.0,
        )


def test_utilization_above_1_rejected():
    with pytest.raises(ValueError, match="utilization"):
        compute_floorplan(
            cells=[CellInstanceEstimate("u", "x", area=1.0)],
            site_height=1.0, site_width=1.0, site_name="x",
            utilization=1.5,
        )


def test_zero_aspect_rejected():
    with pytest.raises(ValueError, match="aspect"):
        compute_floorplan(
            cells=[CellInstanceEstimate("u", "x", area=1.0)],
            site_height=1.0, site_width=1.0, site_name="x",
            aspect=0.0,
        )


def test_zero_total_area_rejected():
    with pytest.raises(ValueError, match="total cell area"):
        compute_floorplan(
            cells=[],
            site_height=1.0, site_width=1.0, site_name="x",
        )


# ---- DEF emission ----


def test_floorplan_to_def(tmp_path):
    from lef_def import write_def

    cells = [CellInstanceEstimate(f"u{i}", "nand2_1", area=3.75) for i in range(20)]
    io = [IoSpec("a", Direction.INPUT), IoSpec("y", Direction.OUTPUT)]

    fp = compute_floorplan(
        cells=cells, site_height=2.72, site_width=0.46, site_name="unithd",
        io_pins=io,
    )

    def_obj = floorplan_to_def(fp, design_name="adder4")
    p = tmp_path / "x.def"
    write_def(def_obj, p)

    text = p.read_text()
    assert "DESIGN adder4" in text
    assert "DIEAREA" in text
    assert "ROW row_0" in text
    assert "COMPONENTS 20" in text
    assert "PINS 2" in text


def test_4bit_adder_floorplan_smoke():
    """Floorplan for the canonical adder4 example."""
    # ~16 cells of ~3.75 sq µm each = 60 sq µm total cell area
    cells = [CellInstanceEstimate(f"u{i}", "nand2_1", area=3.75) for i in range(16)]
    io = [
        *(IoSpec(f"a[{i}]", Direction.INPUT) for i in range(4)),
        *(IoSpec(f"b[{i}]", Direction.INPUT) for i in range(4)),
        IoSpec("cin", Direction.INPUT),
        *(IoSpec(f"sum[{i}]", Direction.OUTPUT) for i in range(4)),
        IoSpec("cout", Direction.OUTPUT),
    ]
    fp = compute_floorplan(
        cells=cells, site_height=2.72, site_width=0.46, site_name="unithd",
        utilization=0.7, io_ring_width=10.0, io_pins=io,
    )
    assert len(fp.components) == 16
    assert len(fp.pins) == 14
    # core area ≥ total cell area
    core_area = (fp.core.x2 - fp.core.x1) * (fp.core.y2 - fp.core.y1)
    assert core_area > 60.0 / 0.7 * 0.95  # within 5% (rounding to row/site grid)
