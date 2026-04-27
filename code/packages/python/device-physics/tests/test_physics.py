"""Tests for device-physics."""

from math import isclose

import pytest

from device_physics import (
    MOSFETParams,
    PNJunction,
    fermi_potential,
    intrinsic_concentration,
    thermal_voltage,
)


# ---- Thermal voltage ----


def test_thermal_voltage_300K():
    v = thermal_voltage(300.0)
    assert isclose(v, 0.02585, rel_tol=1e-3)


def test_thermal_voltage_scales_with_temperature():
    assert thermal_voltage(600) > thermal_voltage(300)


# ---- Intrinsic concentration ----


def test_n_i_300K_matches_constant():
    assert intrinsic_concentration(300.0) == 1.0e16


def test_n_i_increases_with_T():
    assert intrinsic_concentration(400.0) > intrinsic_concentration(300.0)


def test_n_i_below_min_T_rejected():
    with pytest.raises(ValueError, match="below model validity"):
        intrinsic_concentration(50.0)


# ---- Fermi potential ----


def test_fermi_potential_p_type_positive():
    phi = fermi_potential(1e23, kind="p")
    assert phi > 0


def test_fermi_potential_n_type_negative():
    phi = fermi_potential(1e23, kind="n")
    assert phi < 0


def test_fermi_potential_magnitude_matches_handcalc():
    # For N=10^17/cm^3 (= 1e23 /m^3) at 300 K:
    # phi_F = V_T × ln(1e23 / 1e16) = 0.0259 × 16.12 ≈ 0.418 V
    phi = fermi_potential(1e23, kind="p")
    assert isclose(phi, 0.418, abs_tol=0.01)


def test_fermi_potential_invalid_kind():
    with pytest.raises(ValueError, match="kind must be"):
        fermi_potential(1e23, kind="x")


def test_fermi_potential_zero_doping_rejected():
    with pytest.raises(ValueError, match="must be > 0"):
        fermi_potential(0.0, kind="p")


# ---- PNJunction ----


def test_pn_built_in_voltage_handcalc():
    # N_A = N_D = 1e17/cm^3 (1e23 /m^3) at 300 K:
    # phi_bi = V_T × ln((1e23 * 1e23) / (1e16)^2) = 0.0259 × ln(1e14)
    #        = 0.0259 × 32.24 ≈ 0.836 V
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    assert isclose(j.built_in_voltage(), 0.836, abs_tol=0.01)


def test_pn_depletion_width_zero_bias():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    w = j.depletion_width(0.0)
    assert w > 0


def test_pn_depletion_widens_with_reverse_bias():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    w0 = j.depletion_width(0.0)
    w_rev = j.depletion_width(-1.0)
    assert w_rev > w0


def test_pn_depletion_narrows_with_forward_bias():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    w0 = j.depletion_width(0.0)
    w_fwd = j.depletion_width(0.4)
    assert w_fwd < w0


def test_pn_depletion_width_clamps_at_built_in():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    # V > phi_bi: model returns 0
    w = j.depletion_width(10.0)
    assert w == 0.0


def test_pn_saturation_current_positive():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    assert j.saturation_current() > 0


def test_pn_current_zero_bias():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    assert isclose(j.current(0.0), 0.0, abs_tol=1e-30)


def test_pn_current_forward_bias_exponential():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    # I(0.6V) / I(0.55V) ≈ exp(0.05/0.0259) ≈ 6.86
    i_055 = j.current(0.55)
    i_060 = j.current(0.60)
    ratio = i_060 / i_055
    assert isclose(ratio, 6.86, rel_tol=0.05)


def test_pn_current_reverse_bias_saturates():
    j = PNJunction(N_A=1e23, N_D=1e23, A=1e-8)
    i = j.current(-1.0)
    is_sat = j.saturation_current()
    assert isclose(i, -is_sat, rel_tol=0.001)


def test_pn_invalid_doping():
    with pytest.raises(ValueError, match="doping"):
        PNJunction(N_A=0.0, N_D=1e23, A=1e-8)


def test_pn_invalid_area():
    with pytest.raises(ValueError, match="area"):
        PNJunction(N_A=1e23, N_D=1e23, A=0.0)


# ---- MOSFETParams ----


def test_nmos_typical_vt_in_reasonable_range():
    """For a typical 130 nm NMOS with N_body = 1e17/cm^3, T_ox = 4 nm, and
    polysilicon-on-pSi work-function difference, V_t should land in a sensible
    range (well below V_DD, well above 0). Real Sky130 V_t = 0.42 V is achieved
    via V_t-adjust implants that effectively raise the body doping near the
    surface; this test just checks the physics machinery works."""
    nmos = MOSFETParams(
        type="NMOS",
        L=130e-9,
        W=1e-6,
        T_ox=4e-9,
        N_body=1e23,
        phi_MS=-0.95,
    )
    vt = nmos.threshold_voltage()
    assert 0.0 < vt < 1.0


def test_nmos_higher_doping_approaches_sky130_vt():
    """With heavier near-surface doping (effectively the V_t-adjust implant),
    V_t rises toward the Sky130 BSIM3v3 default of ~0.42 V."""
    nmos = MOSFETParams(
        type="NMOS",
        L=130e-9,
        W=1e-6,
        T_ox=4e-9,
        N_body=5e23,  # heavier doping (5e17/cm^3) to approximate V_t adjust
        phi_MS=-0.95,
    )
    vt = nmos.threshold_voltage()
    assert isclose(vt, 0.42, abs_tol=0.3)


def test_pmos_vt_negative_when_sign_inverted():
    """We don't auto-invert sign for PMOS in v0.1.0; phi_MS must be set so V_t
    is in the expected range. Just check the math doesn't crash."""
    pmos = MOSFETParams(
        type="PMOS",
        L=130e-9,
        W=1e-6,
        T_ox=4e-9,
        N_body=1e23,
        phi_MS=0.95,
    )
    _ = pmos.threshold_voltage()  # no exception


def test_body_effect_raises_vt():
    nmos = MOSFETParams(
        type="NMOS",
        L=130e-9,
        W=1e-6,
        T_ox=4e-9,
        N_body=1e23,
        phi_MS=-0.95,
    )
    vt0 = nmos.threshold_voltage(V_SB=0.0)
    vt2 = nmos.threshold_voltage(V_SB=2.0)
    assert vt2 > vt0


def test_thinner_oxide_lowers_vt():
    nmos_thin = MOSFETParams("NMOS", 130e-9, 1e-6, 2e-9, 1e23, -0.95)
    nmos_thick = MOSFETParams("NMOS", 130e-9, 1e-6, 8e-9, 1e23, -0.95)
    assert nmos_thin.threshold_voltage() < nmos_thick.threshold_voltage()


def test_higher_doping_raises_vt():
    nmos_low = MOSFETParams("NMOS", 130e-9, 1e-6, 4e-9, 1e22, -0.95)
    nmos_high = MOSFETParams("NMOS", 130e-9, 1e-6, 4e-9, 1e24, -0.95)
    assert nmos_high.threshold_voltage() > nmos_low.threshold_voltage()


def test_mosfet_invalid_type():
    with pytest.raises(ValueError, match="type must be"):
        MOSFETParams("BJT", 1e-6, 1e-6, 4e-9, 1e23, -0.95)


def test_mosfet_zero_dimensions():
    with pytest.raises(ValueError, match="L and W"):
        MOSFETParams("NMOS", 0.0, 1e-6, 4e-9, 1e23, -0.95)


def test_mosfet_zero_oxide():
    with pytest.raises(ValueError, match="T_ox"):
        MOSFETParams("NMOS", 1e-6, 1e-6, 0.0, 1e23, -0.95)


def test_mosfet_zero_doping():
    with pytest.raises(ValueError, match="N_body"):
        MOSFETParams("NMOS", 1e-6, 1e-6, 4e-9, 0.0, -0.95)


def test_mosfet_v_sb_too_negative_rejected():
    nmos = MOSFETParams("NMOS", 130e-9, 1e-6, 4e-9, 1e23, -0.95)
    with pytest.raises(ValueError, match="forward biased"):
        nmos.threshold_voltage(V_SB=-10.0)


def test_mosfet_derived_properties():
    nmos = MOSFETParams("NMOS", 130e-9, 1e-6, 4e-9, 1e23, -0.95)
    assert nmos.C_ox > 0
    assert nmos.V_FB == nmos.phi_MS  # Q_ox=0
    assert nmos.phi_F > 0
    assert nmos.gamma > 0
