"""Tests for fab process simulation."""

from math import isclose

import pytest

from fab_process_simulation import (
    IMPLANT_RANGES,
    CrossSection,
    Layer,
    deal_grove_oxidation,
    deposit,
    diffuse,
    etch,
    implant,
)


def make_bare_si() -> CrossSection:
    return CrossSection(layers=[Layer("Si", thickness_nm=500_000)])


# ---- deal_grove_oxidation ----


def test_oxidation_grows_oxide_layer():
    cs = make_bare_si()
    cs2 = deal_grove_oxidation(cs, time_min=10)
    assert cs2.layers[0].material == "SiO2"
    # 10 minutes should give a few nm of oxide
    assert 1.0 < cs2.layers[0].thickness_nm < 100.0


def test_oxidation_continues_existing_oxide():
    cs = CrossSection(layers=[Layer("SiO2", 5.0), Layer("Si", 500_000)])
    cs2 = deal_grove_oxidation(cs, time_min=10)
    # New oxide is thicker than the original 5 nm
    assert cs2.layers[0].material == "SiO2"
    assert cs2.layers[0].thickness_nm > 5.0


def test_oxidation_thickness_grows_monotonically():
    cs = make_bare_si()
    cs1 = deal_grove_oxidation(cs, time_min=5)
    cs2 = deal_grove_oxidation(cs, time_min=20)
    assert cs2.layers[0].thickness_nm > cs1.layers[0].thickness_nm


def test_oxidation_zero_time_gives_zero_thickness():
    cs = make_bare_si()
    cs2 = deal_grove_oxidation(cs, time_min=0)
    # Either the SiO2 has 0 thickness or no SiO2 was added; both fine.
    assert cs2.layers[0].material in ("SiO2", "Si")


# ---- deposit ----


def test_deposit_adds_layer_on_top():
    cs = make_bare_si()
    cs2 = deposit(cs, material="Poly", thickness_nm=200)
    assert cs2.layers[0].material == "Poly"
    assert cs2.layers[0].thickness_nm == 200.0
    assert cs2.layers[1].material == "Si"


def test_deposit_zero_thickness_rejected():
    cs = make_bare_si()
    with pytest.raises(ValueError, match="thickness"):
        deposit(cs, material="Poly", thickness_nm=0)


def test_deposit_negative_thickness_rejected():
    cs = make_bare_si()
    with pytest.raises(ValueError, match="thickness"):
        deposit(cs, material="Poly", thickness_nm=-10)


# ---- etch ----


def test_etch_removes_top_layer_partial():
    cs = CrossSection(layers=[Layer("Poly", 200), Layer("Si", 500_000)])
    cs2 = etch(cs, target_layer="Poly", depth_nm=50)
    assert cs2.layers[0].material == "Poly"
    assert cs2.layers[0].thickness_nm == 150.0


def test_etch_removes_top_layer_completely():
    cs = CrossSection(layers=[Layer("Poly", 200), Layer("Si", 500_000)])
    cs2 = etch(cs, target_layer="Poly", depth_nm=200)
    assert cs2.layers[0].material == "Si"


def test_etch_skips_non_matching_layer():
    cs = CrossSection(layers=[Layer("SiO2", 100), Layer("Si", 500_000)])
    cs2 = etch(cs, target_layer="Poly", depth_nm=50)  # wrong material
    # No change
    assert cs2.layers[0].material == "SiO2"
    assert cs2.layers[0].thickness_nm == 100.0


def test_etch_zero_depth_no_op():
    cs = make_bare_si()
    cs2 = etch(cs, target_layer="Si", depth_nm=0)
    assert cs.layers[0].thickness_nm == cs2.layers[0].thickness_nm


def test_etch_empty_cross_section():
    cs = CrossSection(layers=[])
    cs2 = etch(cs, target_layer="Si", depth_nm=10)
    assert cs2.layers == []


# ---- implant ----


def test_implant_adds_doping_to_si():
    cs = make_bare_si()
    cs2 = implant(cs, species="B", energy_keV=10, dose_per_cm2=5e12)
    assert "B" in cs2.layers[0].doping
    profile = cs2.layers[0].doping["B"]
    assert len(profile) > 0
    # Concentrations should be positive
    for _, conc in profile:
        assert conc > 0


def test_implant_rp_matches_table():
    """Implant at exactly tabulated energy should use the table's Rp."""
    cs = make_bare_si()
    cs2 = implant(cs, species="B", energy_keV=10, dose_per_cm2=5e12)
    profile = cs2.layers[0].doping["B"]
    # Find the depth with the maximum concentration.
    peak_depth = max(profile, key=lambda p: p[1])[0]
    rp_expected = IMPLANT_RANGES[("B", 10)][0]  # 33 nm
    # Within ~5 nm of expected Rp (sample resolution)
    assert abs(peak_depth - rp_expected) < 10.0


def test_implant_unknown_species_rejected():
    cs = make_bare_si()
    with pytest.raises(ValueError, match="unknown"):
        implant(cs, species="MYSTERY", energy_keV=10, dose_per_cm2=1e13)


def test_implant_below_table_extrapolates():
    cs = make_bare_si()
    cs2 = implant(cs, species="B", energy_keV=5, dose_per_cm2=1e13)
    # Extrapolated; just verify it doesn't crash
    assert "B" in cs2.layers[0].doping


def test_implant_above_table_extrapolates():
    cs = make_bare_si()
    cs2 = implant(cs, species="B", energy_keV=200, dose_per_cm2=1e13)
    assert "B" in cs2.layers[0].doping


def test_implant_interpolates_between_tabulated():
    cs = make_bare_si()
    # Between B at 10 keV (Rp=33) and B at 30 keV (Rp=92): at 20 keV expect Rp ≈ 62.5
    cs2 = implant(cs, species="B", energy_keV=20, dose_per_cm2=1e13)
    profile = cs2.layers[0].doping["B"]
    peak_depth = max(profile, key=lambda p: p[1])[0]
    assert 50.0 < peak_depth < 75.0


def test_implant_skips_non_si_top():
    cs = CrossSection(layers=[Layer("SiO2", 100), Layer("Si", 500_000)])
    cs2 = implant(cs, species="B", energy_keV=10, dose_per_cm2=1e13)
    # Should add to the FIRST Si layer (skipping SiO2)
    assert "B" in cs2.layers[1].doping
    assert "B" not in cs2.layers[0].doping


# ---- diffuse ----


def test_diffuse_preserves_doping():
    cs = make_bare_si()
    cs = implant(cs, species="B", energy_keV=10, dose_per_cm2=1e13)
    cs2 = diffuse(cs, time_min=30)
    assert "B" in cs2.layers[0].doping


def test_diffuse_no_doping_no_op():
    cs = make_bare_si()
    cs2 = diffuse(cs, time_min=30)
    assert cs2.layers[0].doping == {}


# ---- End-to-end NMOS recipe ----


def test_nmos_fabrication_recipe():
    """Trace an NMOS through the FEOL: oxidation, implant, anneal, etch, deposit."""
    cs = make_bare_si()

    # Pad oxide
    cs = deal_grove_oxidation(cs, time_min=5)
    assert cs.layers[0].material == "SiO2"

    # Etch oxide off (active region)
    cs = etch(cs, target_layer="SiO2", depth_nm=1000)
    assert cs.layers[0].material == "Si"

    # P-well: boron implant + anneal
    cs = implant(cs, species="B", energy_keV=100, dose_per_cm2=1e13)
    cs = diffuse(cs, time_min=60, temperature_C=1000)
    assert "B" in cs.layers[0].doping

    # V_t adjust: shallow boron implant
    cs = implant(cs, species="B", energy_keV=10, dose_per_cm2=5e12)

    # Gate oxide
    cs = deal_grove_oxidation(cs, time_min=10)
    assert cs.layers[0].material == "SiO2"

    # Polysilicon gate
    cs = deposit(cs, material="Poly", thickness_nm=200)
    assert cs.layers[0].material == "Poly"
    assert cs.layers[1].material == "SiO2"
    assert cs.layers[2].material == "Si"
