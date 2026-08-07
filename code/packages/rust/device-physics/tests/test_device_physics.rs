use device_physics::*;

// ---------------------------------------------------------------------------
// thermal_voltage
// ---------------------------------------------------------------------------

#[test]
fn test_thermal_voltage_300k() {
    let vt = thermal_voltage(300.0);
    // kT/q at 300 K ≈ 0.025852 V
    assert!((vt - 0.025852).abs() < 1e-5, "V_T={vt}");
}

#[test]
fn test_thermal_voltage_scales_linearly() {
    // V_T = kT/q is linear in T.
    let v1 = thermal_voltage(300.0);
    let v2 = thermal_voltage(600.0);
    assert!((v2 - 2.0 * v1).abs() < 1e-10, "V_T should double at 600 K");
}

// ---------------------------------------------------------------------------
// intrinsic_concentration
// ---------------------------------------------------------------------------

#[test]
fn test_intrinsic_concentration_300k() {
    let ni = intrinsic_concentration(300.0).unwrap();
    assert_eq!(ni, N_I_300K, "at 300 K, must return N_I_300K exactly");
}

#[test]
fn test_intrinsic_concentration_increases_with_temperature() {
    let ni_300 = intrinsic_concentration(300.0).unwrap();
    let ni_400 = intrinsic_concentration(400.0).unwrap();
    assert!(ni_400 > ni_300, "n_i must increase with temperature");
}

#[test]
fn test_intrinsic_concentration_low_t_error() {
    assert!(intrinsic_concentration(50.0).is_err(), "below 100 K must fail");
}

// ---------------------------------------------------------------------------
// fermi_potential
// ---------------------------------------------------------------------------

#[test]
fn test_fermi_potential_p_type_positive() {
    // p-type: φ_F should be positive
    let phi = fermi_potential(1e23, "p", 300.0).unwrap();
    assert!(phi > 0.0, "p-type φ_F must be positive");
}

#[test]
fn test_fermi_potential_n_type_negative() {
    // n-type: φ_F should be negative
    let phi = fermi_potential(1e23, "n", 300.0).unwrap();
    assert!(phi < 0.0, "n-type φ_F must be negative");
}

#[test]
fn test_fermi_potential_symmetry() {
    let phi_p = fermi_potential(1e22, "p", 300.0).unwrap();
    let phi_n = fermi_potential(1e22, "n", 300.0).unwrap();
    assert!((phi_p + phi_n).abs() < 1e-10, "magnitudes must match");
}

#[test]
fn test_fermi_potential_bad_kind() {
    assert!(fermi_potential(1e23, "x", 300.0).is_err());
}

#[test]
fn test_fermi_potential_bad_doping() {
    assert!(fermi_potential(-1.0, "p", 300.0).is_err());
    assert!(fermi_potential(0.0, "n", 300.0).is_err());
}

// ---------------------------------------------------------------------------
// PNJunction
// ---------------------------------------------------------------------------

#[test]
fn test_pn_junction_invalid_params() {
    assert!(PNJunction::new(0.0, 1e22, 1e-8, 300.0, 1e-6, 1e-6).is_err());
    assert!(PNJunction::new(1e23, 1e22, -1.0, 300.0, 1e-6, 1e-6).is_err());
}

#[test]
fn test_built_in_voltage_typical() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let vbi = j.built_in_voltage();
    // Typical silicon p-n junction: ~0.7–0.8 V
    assert!(vbi > 0.6 && vbi < 1.1, "V_bi={vbi}");
}

#[test]
fn test_depletion_width_zero_bias() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let w = j.depletion_width(0.0);
    assert!(w > 0.0, "depletion width must be positive at zero bias");
}

#[test]
fn test_depletion_width_reverse_bias_wider() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let w0 = j.depletion_width(0.0);
    let wr = j.depletion_width(-3.0); // reverse bias
    assert!(wr > w0, "reverse bias must widen the depletion region");
}

#[test]
fn test_depletion_width_forward_bias_clamp() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let vbi = j.built_in_voltage();
    let w = j.depletion_width(vbi + 1.0);
    assert_eq!(w, 0.0, "beyond V_bi depletion width must clamp to 0");
}

#[test]
fn test_saturation_current_positive() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    assert!(j.saturation_current() > 0.0);
}

#[test]
fn test_shockley_current_forward_bias() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let i = j.current(0.6);
    assert!(i > 0.0, "forward bias must produce positive current");
}

#[test]
fn test_shockley_current_zero_bias() {
    let j = PNJunction::new(1e23, 1e22, 1e-8, 300.0, 1e-6, 1e-6).unwrap();
    let i = j.current(0.0);
    assert!(i.abs() < 1e-30, "no current at zero bias: I={i}");
}

// ---------------------------------------------------------------------------
// MOSFETParams
// ---------------------------------------------------------------------------

#[test]
fn test_mosfet_params_invalid_type() {
    assert!(MOSFETParams::new("JFET", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).is_err());
}

#[test]
fn test_mosfet_params_c_ox() {
    // T_ox = 2 nm → C_ox = ε_ox / 2e-9
    let p = MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
    let expected = EPS_OX / 2e-9;
    assert!((p.c_ox() - expected).abs() < 1e-3 * expected);
}

#[test]
fn test_threshold_voltage_nmos_reasonable() {
    // 130 nm-style NMOS: expect V_t ~ 0.3–0.6 V at zero body bias
    let p = MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
    let vt = p.threshold_voltage(0.0).unwrap();
    // With N_body = 1e24 /m³ (1e18 /cm³), V_t is ~1.2 V — high p-well doping.
    assert!(vt > 0.5 && vt < 1.6, "V_t={vt}");
}

#[test]
fn test_threshold_voltage_body_effect_raises_vt() {
    let p = MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
    let vt0 = p.threshold_voltage(0.0).unwrap();
    let vt_body = p.threshold_voltage(1.0).unwrap();
    assert!(vt_body > vt0, "V_SB > 0 must raise V_t");
}

#[test]
fn test_threshold_voltage_invalid_vsb() {
    let p = MOSFETParams::new("NMOS", 130e-9, 1e-6, 2e-9, 1e24, -0.05, 0.0, 300.0).unwrap();
    // V_SB < -2*phi_F should fail
    let two_phi_f = 2.0 * p.phi_f();
    assert!(p.threshold_voltage(-(two_phi_f + 1.0)).is_err());
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// These assertions deliberately sanity-check compile-time physical constants;
// clippy flags asserts on constant expressions, but that is exactly the intent
// here (guarding the constant table against accidental edits).
#[allow(clippy::assertions_on_constants)]
#[test]
fn test_constants_sanity() {
    assert!(K_BOLTZMANN > 1e-24 && K_BOLTZMANN < 1e-22);
    assert!(Q_ELECTRON > 1e-20 && Q_ELECTRON < 1e-18);
    assert!(EPS_SI > EPS0, "silicon permittivity > vacuum");
    assert!(EPS_OX > EPS0, "oxide permittivity > vacuum");
    assert!(MU_N_300K > MU_P_300K, "electrons more mobile than holes in Si");
}
