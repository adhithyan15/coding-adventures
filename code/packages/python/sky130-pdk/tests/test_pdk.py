"""Tests for sky130-pdk."""

from pathlib import Path

import pytest

from sky130_pdk import (
    LAYER_MAP,
    TEACHING_CELLS,
    Pdk,
    PdkProfile,
    ProcessMetadata,
    load_sky130,
)


# ---- ProcessMetadata defaults ----


def test_process_metadata_defaults():
    p = ProcessMetadata()
    assert p.feature_size_nm == 130
    assert p.vdd_nominal == 1.8
    assert p.gate_oxide_thickness_nm == 4.2
    assert p.metal_layers == 6


def test_process_vt_signs():
    p = ProcessMetadata()
    assert p.nmos_vt_typical > 0
    assert p.pmos_vt_typical < 0


# ---- TEACHING_CELLS ----


def test_teaching_cells_present():
    expected_subset = {
        "sky130_fd_sc_hd__inv_1",
        "sky130_fd_sc_hd__nand2_1",
        "sky130_fd_sc_hd__nor2_1",
        "sky130_fd_sc_hd__xor2_1",
        "sky130_fd_sc_hd__mux2_1",
        "sky130_fd_sc_hd__dfxtp_1",
        "sky130_fd_sc_hd__dfrtp_1",
        "sky130_fd_sc_hd__clkbuf_1",
    }
    assert expected_subset.issubset(set(TEACHING_CELLS.keys()))


def test_teaching_cells_have_inv_at_multiple_drives():
    drives = sorted(
        c.drive_strength
        for c in TEACHING_CELLS.values()
        if c.name.startswith("sky130_fd_sc_hd__inv_")
    )
    assert drives == [1, 2, 4, 8]


def test_nand2_function_string():
    cell = TEACHING_CELLS["sky130_fd_sc_hd__nand2_1"]
    assert "A" in cell.function
    assert "B" in cell.function


# ---- LAYER_MAP ----


def test_layer_map_includes_metals():
    expected = {f"met{i}.drawing" for i in range(1, 6)}
    assert expected.issubset(set(LAYER_MAP.keys()))


def test_met1_drawing_layer_number():
    layer = LAYER_MAP["met1.drawing"]
    assert layer.layer_number == 68
    assert layer.datatype == 20


def test_met1_pin_layer():
    layer = LAYER_MAP["met1.pin"]
    assert layer.purpose == "pin"


# ---- load_sky130 (TEACHING) ----


def test_load_teaching_profile():
    pdk = load_sky130()
    assert pdk.profile == PdkProfile.TEACHING
    assert pdk.root is None
    assert len(pdk.cells) >= 30


def test_teaching_pdk_get_cell():
    pdk = load_sky130()
    cell = pdk.get_cell("sky130_fd_sc_hd__inv_1")
    assert cell.drive_strength == 1


def test_teaching_pdk_get_unknown_cell_raises():
    pdk = load_sky130()
    with pytest.raises(KeyError, match="not in PDK"):
        pdk.get_cell("nonexistent_cell")


def test_teaching_pdk_get_layer():
    pdk = load_sky130()
    layer = pdk.get_layer("poly.drawing")
    assert layer.name == "poly"


def test_teaching_pdk_get_unknown_layer_raises():
    pdk = load_sky130()
    with pytest.raises(KeyError, match="not in PDK"):
        pdk.get_layer("xyz.drawing")


def test_pdk_cell_names_sorted():
    pdk = load_sky130()
    names = pdk.cell_names
    assert names == sorted(names)


# ---- load_sky130 (FULL) ----


def test_full_profile_requires_root():
    with pytest.raises(ValueError, match="root"):
        load_sky130(profile=PdkProfile.FULL)


def test_full_profile_validates_path_exists(tmp_path: Path):
    nonexistent = tmp_path / "no_such_pdk"
    with pytest.raises(FileNotFoundError, match="not found"):
        load_sky130(root=str(nonexistent), profile=PdkProfile.FULL)


def test_full_profile_with_existing_path(tmp_path: Path):
    """Validate doesn't actually parse — just confirms the path exists."""
    pdk = load_sky130(root=str(tmp_path), profile=PdkProfile.FULL)
    assert pdk.profile == PdkProfile.FULL
    assert pdk.root == tmp_path


# ---- 4-bit adder cell coverage ----


def test_4bit_adder_cells_available():
    """The PDK should include all cells needed to map a 4-bit adder."""
    pdk = load_sky130()
    needed = [
        "sky130_fd_sc_hd__xor2_1",
        "sky130_fd_sc_hd__and2_1",
        "sky130_fd_sc_hd__or2_1",
        "sky130_fd_sc_hd__inv_1",
    ]
    for name in needed:
        cell = pdk.get_cell(name)
        assert cell.drive_strength >= 1
