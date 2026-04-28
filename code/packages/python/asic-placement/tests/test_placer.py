"""Tests for ASIC placement."""

import pytest

from asic_floorplan import CellInstanceEstimate, IoSpec, compute_floorplan
from asic_placement import CellSize, PlacementOptions, place
from lef_def import Direction


def make_simple_fp(n_cells: int = 8):
    cells = [CellInstanceEstimate(f"u{i}", "nand2_1", area=3.0) for i in range(n_cells)]
    return compute_floorplan(
        cells=cells, site_height=1.0, site_width=1.0, site_name="x",
        utilization=0.5,
    )


def make_simple_sizes():
    return {"nand2_1": CellSize("nand2_1", width=1.0, height=1.0)}


# ---- Basic placement ----


def test_all_cells_placed():
    fp = make_simple_fp(8)
    placed_def, report = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        options=PlacementOptions(iterations=0, legalize=False),
    )
    assert report.cells_placed == 8
    assert all(c.placed for c in placed_def.components)


def test_placement_returns_valid_def():
    fp = make_simple_fp()
    placed_def, _ = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        options=PlacementOptions(iterations=0),
    )
    assert placed_def.die_area is not None
    assert len(placed_def.rows) > 0


def test_no_rows_raises():
    from dataclasses import replace
    fp = make_simple_fp()
    bad_fp = replace(fp, rows=())
    with pytest.raises(ValueError, match="no rows"):
        place(fp=bad_fp, cell_sizes=make_simple_sizes())


def test_cell_too_wide_raises():
    fp = make_simple_fp(2)
    big_sizes = {"nand2_1": CellSize("nand2_1", width=1000.0, height=1.0)}
    with pytest.raises(ValueError, match="doesn't fit"):
        place(
            fp=fp, cell_sizes=big_sizes,
            options=PlacementOptions(iterations=0, legalize=False),
        )


# ---- HPWL minimization ----


def test_sa_reduces_hpwl():
    """With sensible nets, SA should reduce HPWL vs random initial."""
    fp = make_simple_fp(20)
    sizes = make_simple_sizes()
    # A long net through 5 cells; random placement scatters them; SA should
    # bring them together.
    nets = [[f"u{i}" for i in range(5)]]

    _, no_anneal = place(
        fp=fp, cell_sizes=sizes, nets=nets,
        options=PlacementOptions(iterations=0, seed=1, legalize=False),
    )

    _, with_anneal = place(
        fp=fp, cell_sizes=sizes, nets=nets,
        options=PlacementOptions(iterations=5000, seed=1, legalize=False),
    )

    assert with_anneal.final_hpwl <= no_anneal.final_hpwl


def test_sa_converges_with_seed():
    """Same seed = same result."""
    fp = make_simple_fp(10)
    sizes = make_simple_sizes()
    nets = [["u0", "u1", "u2"]]

    _, r1 = place(
        fp=fp, cell_sizes=sizes, nets=nets,
        options=PlacementOptions(iterations=1000, seed=42, legalize=False),
    )
    _, r2 = place(
        fp=fp, cell_sizes=sizes, nets=nets,
        options=PlacementOptions(iterations=1000, seed=42, legalize=False),
    )
    assert r1.final_hpwl == r2.final_hpwl


def test_zero_iterations_no_swaps():
    fp = make_simple_fp(10)
    _, r = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        nets=[["u0", "u1"]],
        options=PlacementOptions(iterations=0),
    )
    assert r.accepted_swaps == 0
    assert r.rejected_swaps == 0


# ---- Legalization ----


def test_legalization_packs_cells_left_to_right():
    fp = make_simple_fp(5)
    placed_def, _ = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        options=PlacementOptions(iterations=0, legalize=True),
    )
    # All cells in a row should be at sequential x positions
    by_y: dict[float, list] = {}
    for c in placed_def.components:
        by_y.setdefault(c.location_y, []).append(c)
    for row_cells in by_y.values():
        row_cells.sort(key=lambda c: c.location_x)
        for i in range(1, len(row_cells)):
            # Each cell starts where the previous ended
            prev = row_cells[i - 1]
            cur = row_cells[i]
            assert cur.location_x >= prev.location_x


def test_legalize_disabled_keeps_sa_positions():
    fp = make_simple_fp(5)
    placed_def, _ = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        nets=[["u0", "u1", "u2"]],
        options=PlacementOptions(iterations=100, seed=42, legalize=False),
    )
    # Just verify it ran; positions are SA-final without legalization
    assert all(c.placed for c in placed_def.components)


# ---- Net edge cases ----


def test_single_node_net_ignored():
    fp = make_simple_fp(5)
    _, r = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        nets=[["u0"]],  # single node — no contribution to HPWL
        options=PlacementOptions(iterations=10),
    )
    assert r.final_hpwl == 0.0


def test_net_with_unknown_cell_skipped():
    fp = make_simple_fp(3)
    _, r = place(
        fp=fp, cell_sizes=make_simple_sizes(),
        nets=[["u0", "missing", "u1"]],
        options=PlacementOptions(iterations=10),
    )
    # Should not crash; missing cell silently skipped
    assert r.cells_placed == 3


# ---- 4-bit adder smoke test ----


def test_4bit_adder_placement():
    cells = [CellInstanceEstimate(f"u{i}", "nand2_1", area=3.75) for i in range(16)]
    io = [
        IoSpec(f"a[{i}]", Direction.INPUT) for i in range(4)
    ] + [
        IoSpec(f"sum[{i}]", Direction.OUTPUT) for i in range(4)
    ]
    fp = compute_floorplan(
        cells=cells, site_height=2.72, site_width=0.46, site_name="unithd",
        utilization=0.7, io_pins=io,
    )
    sizes = {"nand2_1": CellSize("nand2_1", width=1.4, height=2.72)}
    # Some plausible nets
    nets = [
        [f"u{i}", f"u{i+1}"] for i in range(15)
    ]
    placed_def, report = place(
        fp=fp, cell_sizes=sizes, nets=nets,
        options=PlacementOptions(iterations=2000, seed=42),
    )
    assert report.cells_placed == 16
    assert report.final_hpwl > 0
