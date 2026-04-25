"""Tests for standard-cell-library."""

import pytest

from standard_cell_library import (
    LookupTable,
    build_default_library,
    select_drive,
)

# ---- LookupTable ----


def test_lookup_corner_values():
    lut = LookupTable(
        slew_index=(0.01, 0.10, 1.00),
        load_index=(0.5, 5.0, 50.0),
        values=(
            (1.0, 2.0, 3.0),
            (4.0, 5.0, 6.0),
            (7.0, 8.0, 9.0),
        ),
    )
    # Exact corner queries
    assert lut.lookup(0.01, 0.5) == 1.0
    assert lut.lookup(1.00, 50.0) == 9.0


def test_lookup_clamps_below_range():
    lut = LookupTable(
        slew_index=(0.1, 1.0),
        load_index=(1.0, 10.0),
        values=((1.0, 2.0), (3.0, 4.0)),
    )
    # Below min — clamps to min
    assert lut.lookup(0.001, 0.001) == 1.0


def test_lookup_clamps_above_range():
    lut = LookupTable(
        slew_index=(0.1, 1.0),
        load_index=(1.0, 10.0),
        values=((1.0, 2.0), (3.0, 4.0)),
    )
    assert lut.lookup(100.0, 1000.0) == 4.0


def test_lookup_bilinear_interp():
    lut = LookupTable(
        slew_index=(0.0, 1.0),
        load_index=(0.0, 10.0),
        values=((0.0, 1.0), (10.0, 11.0)),
    )
    # Center: average of corners = (0+1+10+11)/4 = 5.5
    result = lut.lookup(0.5, 5.0)
    assert abs(result - 5.5) < 1e-6


def test_lookup_monotonic_in_load():
    """Default LUTs grow with load."""
    lib = build_default_library()
    cell = lib.get("sky130_fd_sc_hd__nand2_1")
    arc = cell.timing_arcs[0]
    d_low = arc.cell_rise.lookup(slew_ns=0.05, load_ff=0.5)
    d_high = arc.cell_rise.lookup(slew_ns=0.05, load_ff=10.0)
    assert d_high > d_low


# ---- Library / build_default_library ----


def test_library_has_expected_cells():
    lib = build_default_library()
    expected = {
        "sky130_fd_sc_hd__inv_1",
        "sky130_fd_sc_hd__nand2_1",
        "sky130_fd_sc_hd__nor2_1",
        "sky130_fd_sc_hd__xor2_1",
        "sky130_fd_sc_hd__mux2_1",
        "sky130_fd_sc_hd__dfxtp_1",
    }
    assert expected.issubset(set(lib.cells.keys()))


def test_library_get_known_cell():
    lib = build_default_library()
    cell = lib.get("sky130_fd_sc_hd__inv_1")
    assert cell.area > 0
    assert cell.leakage_power > 0
    assert "A" in cell.pin_capacitance
    assert len(cell.timing_arcs) > 0


def test_library_get_unknown_raises():
    lib = build_default_library()
    with pytest.raises(KeyError, match="not in library"):
        lib.get("nonexistent_cell")


def test_library_default_corners():
    lib = build_default_library()
    assert lib.voltage == 1.8
    assert lib.process == "tt"


# ---- list_drives ----


def test_inv_has_four_drives():
    lib = build_default_library()
    drives = lib.list_drives("sky130_fd_sc_hd__inv")
    assert drives == [1, 2, 4, 8]


def test_buf_has_four_drives():
    lib = build_default_library()
    drives = lib.list_drives("sky130_fd_sc_hd__buf")
    assert drives == [1, 2, 4, 8]


def test_nand2_has_two_drives():
    lib = build_default_library()
    drives = lib.list_drives("sky130_fd_sc_hd__nand2")
    assert 1 in drives and 2 in drives


def test_unknown_base_returns_empty():
    lib = build_default_library()
    assert lib.list_drives("sky130_fd_sc_hd__nonexistent") == []


# ---- TimingArc semantics ----


def test_inv_is_negative_unate():
    """INV should be negative_unate."""
    lib = build_default_library()
    cell = lib.get("sky130_fd_sc_hd__inv_1")
    arc = cell.timing_arcs[0]
    assert arc.sense == "negative_unate"
    assert arc.related_pin == "A"
    assert arc.output_pin == "Y"


def test_buf_is_positive_unate():
    """BUF should be positive_unate."""
    lib = build_default_library()
    cell = lib.get("sky130_fd_sc_hd__buf_1")
    arc = cell.timing_arcs[0]
    assert arc.sense == "positive_unate"


def test_xor_is_non_unate():
    lib = build_default_library()
    cell = lib.get("sky130_fd_sc_hd__xor2_1")
    arc = cell.timing_arcs[0]
    assert arc.sense == "non_unate"


# ---- Drive strength selection ----


def test_select_drive_returns_smallest_when_no_target():
    lib = build_default_library()
    chosen = select_drive(lib, "sky130_fd_sc_hd__inv", target_load_ff=1.0)
    assert chosen == "sky130_fd_sc_hd__inv_1"


def test_select_drive_picks_meeting_target():
    lib = build_default_library()
    # With a generous target delay, smallest INV should suffice
    chosen = select_drive(
        lib, "sky130_fd_sc_hd__inv",
        target_load_ff=2.0, target_delay_ns=1.0,
    )
    assert chosen == "sky130_fd_sc_hd__inv_1"


def test_select_drive_unknown_base_raises():
    lib = build_default_library()
    with pytest.raises(KeyError, match="no drives"):
        select_drive(lib, "sky130_fd_sc_hd__missing", target_load_ff=1.0)


def test_select_drive_falls_back_to_largest():
    """When even the largest drive can't meet the target, return the largest."""
    lib = build_default_library()
    chosen = select_drive(
        lib, "sky130_fd_sc_hd__inv",
        target_load_ff=10.0, target_delay_ns=0.0001,
    )
    # Should be the _8 (largest) since none can be fast enough at 0.0001 ns.
    assert chosen == "sky130_fd_sc_hd__inv_8"


# ---- 4-bit adder cell coverage ----


def test_4bit_adder_cells_in_library():
    """The library should include the cells the adder uses after tech-mapping."""
    lib = build_default_library()
    needed = [
        "sky130_fd_sc_hd__xor2_1",
        "sky130_fd_sc_hd__and2_1",
        "sky130_fd_sc_hd__or2_1",
        "sky130_fd_sc_hd__inv_1",
    ]
    for name in needed:
        cell = lib.get(name)
        assert cell.area > 0
        assert len(cell.timing_arcs) > 0
