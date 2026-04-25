"""Tests for MOSFET models."""

import pytest

from mosfet_models import (
    MOSFET,
    Level1Model,
    Level1Params,
    MosfetType,
    evaluate_level1,
)


def make_default_nmos() -> MOSFET:
    return MOSFET(type=MosfetType.NMOS, model=Level1Model(Level1Params()))


# ---- Region detection ----


def test_cutoff_region_returns_zero_id():
    """V_GS < V_t with subthreshold disabled -> Id ≈ 0."""
    p = Level1Params(VT0=0.5, subthreshold_enable=False)
    r = evaluate_level1(p, V_GS=0.2, V_DS=1.8, V_BS=0.0)
    assert r.Id == 0.0
    assert r.region == "cutoff"


def test_subthreshold_region_returns_small_id():
    """V_GS just below V_t with subthreshold enabled -> small Id > 0."""
    p = Level1Params(VT0=0.5, subthreshold_enable=True)
    r = evaluate_level1(p, V_GS=0.4, V_DS=1.8, V_BS=0.0)
    assert r.region == "subthreshold"
    assert 0 < r.Id < 1e-6  # small but nonzero


def test_triode_region():
    """V_GS > V_t but V_DS small -> triode."""
    p = Level1Params(VT0=0.42)
    r = evaluate_level1(p, V_GS=1.8, V_DS=0.1, V_BS=0.0)
    assert r.region == "triode"
    assert r.Id > 0


def test_saturation_region():
    """V_GS > V_t and V_DS > V_GS - V_t -> saturation."""
    p = Level1Params(VT0=0.42)
    r = evaluate_level1(p, V_GS=1.8, V_DS=1.8, V_BS=0.0)
    assert r.region == "saturation"
    assert r.Id > 0


# ---- Quantitative ----


def test_saturation_id_formula():
    """In saturation: Id = (KP/2) × (W/L) × V_OV² × (1 + λ V_DS)."""
    p = Level1Params(
        VT0=0.5, KP=200e-6, W=1e-6, L=100e-9, LAMBDA=0.0, GAMMA=0.0,
        subthreshold_enable=False,
    )
    V_OV = 1.5 - 0.5  # = 1.0
    expected = (200e-6 / 2) * (1e-6 / 100e-9) * 1.0 * 1.0
    r = evaluate_level1(p, V_GS=1.5, V_DS=1.5, V_BS=0.0)
    assert abs(r.Id - expected) / expected < 0.01


def test_triode_id_formula():
    """Triode: Id = β × (V_OV V_DS - V_DS²/2) × (1 + λ V_DS)."""
    p = Level1Params(
        VT0=0.5, KP=200e-6, W=1e-6, L=100e-9, LAMBDA=0.0, GAMMA=0.0,
        subthreshold_enable=False,
    )
    V_OV = 1.5 - 0.5  # = 1.0
    V_DS = 0.2
    beta = 200e-6 * (1e-6 / 100e-9)
    expected = beta * (V_OV * V_DS - V_DS * V_DS / 2)
    r = evaluate_level1(p, V_GS=1.5, V_DS=V_DS, V_BS=0.0)
    assert abs(r.Id - expected) / expected < 0.01


def test_id_increases_with_vgs_in_saturation():
    p = Level1Params()
    r1 = evaluate_level1(p, V_GS=0.8, V_DS=1.8)
    r2 = evaluate_level1(p, V_GS=1.4, V_DS=1.8)
    assert r2.Id > r1.Id


def test_lambda_increases_id_with_vds():
    p = Level1Params(LAMBDA=0.1, subthreshold_enable=False)
    r1 = evaluate_level1(p, V_GS=1.8, V_DS=1.0)
    r2 = evaluate_level1(p, V_GS=1.8, V_DS=1.8)
    assert r2.Id > r1.Id


# ---- Body effect ----


def test_body_effect_raises_threshold():
    p_no_body = Level1Params(VT0=0.42, GAMMA=0.0, subthreshold_enable=False)
    p_with_body = Level1Params(VT0=0.42, GAMMA=0.4, subthreshold_enable=False)
    r1 = evaluate_level1(p_no_body, V_GS=1.0, V_DS=1.8, V_BS=-1.0)
    r2 = evaluate_level1(p_with_body, V_GS=1.0, V_DS=1.8, V_BS=-1.0)
    # With body effect, V_BS=-1V raises V_t -> smaller Id
    assert r2.Id < r1.Id


# ---- Small-signal parameters ----


def test_gm_in_saturation():
    """gm = β × V_OV × (1 + λ V_DS)."""
    p = Level1Params(VT0=0.5, KP=200e-6, W=1e-6, L=100e-9, LAMBDA=0.0,
                     GAMMA=0.0, subthreshold_enable=False)
    r = evaluate_level1(p, V_GS=1.5, V_DS=1.5, V_BS=0.0)
    expected_gm = 200e-6 * (1e-6 / 100e-9) * 1.0
    assert abs(r.gm - expected_gm) / expected_gm < 0.01


def test_gds_increases_with_lambda():
    p_no = Level1Params(LAMBDA=0.0, subthreshold_enable=False)
    p_yes = Level1Params(LAMBDA=0.1, subthreshold_enable=False)
    r_no = evaluate_level1(p_no, V_GS=1.8, V_DS=1.8)
    r_yes = evaluate_level1(p_yes, V_GS=1.8, V_DS=1.8)
    assert r_yes.gds > r_no.gds


# ---- MOSFET wrapper ----


def test_nmos_wrapper_passes_through():
    nmos = make_default_nmos()
    r = nmos.dc(V_GS=1.8, V_DS=1.8)
    assert r.Id > 0


def test_pmos_wrapper_flips_sign():
    pmos = MOSFET(type=MosfetType.PMOS, model=Level1Model(Level1Params()))
    # Apply PMOS-style operating point: gate below source.
    r = pmos.dc(V_GS=-1.8, V_DS=-1.8)
    # PMOS with V_GS=-1.8 should produce negative Id (flowing source-to-drain).
    assert r.Id < 0
    assert r.region == "saturation"


def test_pmos_wrapper_in_cutoff():
    pmos = MOSFET(type=MosfetType.PMOS, model=Level1Model(Level1Params(
        VT0=0.42, subthreshold_enable=False
    )))
    r = pmos.dc(V_GS=-0.2, V_DS=-1.8)
    assert r.Id == 0.0
    assert r.region == "cutoff"


# ---- Edge cases ----


def test_zero_vds_returns_zero_in_triode():
    p = Level1Params(subthreshold_enable=False)
    r = evaluate_level1(p, V_GS=1.8, V_DS=0.0, V_BS=0.0)
    assert r.Id == 0.0


def test_subthreshold_id_grows_exponentially_with_vgs():
    p = Level1Params(VT0=0.5, subthreshold_enable=True, N_SUB=1.0)
    r1 = evaluate_level1(p, V_GS=0.30, V_DS=1.8, V_BS=0.0)
    r2 = evaluate_level1(p, V_GS=0.35, V_DS=1.8, V_BS=0.0)
    # 50 mV / V_T(0.0259) = 1.93; exp(1.93) ≈ 6.9
    ratio = r2.Id / r1.Id
    assert 5.0 < ratio < 10.0


def test_default_params_match_spec():
    p = Level1Params()
    assert p.VT0 == 0.42
    assert p.KP == 220e-6
    assert p.W == 1e-6
    assert p.L == 130e-9
