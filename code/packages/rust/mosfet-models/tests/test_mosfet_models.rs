use mosfet_models::{evaluate_level1, Level1Model, Level1Params, Mosfet, MosfetType, Region};

// ---------------------------------------------------------------------------
// Level1Params defaults
// ---------------------------------------------------------------------------

#[test]
fn test_default_params() {
    let p = Level1Params::default();
    assert_eq!(p.vt0, 0.42);
    assert!((p.kp - 220e-6).abs() < 1e-9);
    assert_eq!(p.lambda, 0.05);
    assert_eq!(p.gamma, 0.27);
    assert_eq!(p.phi, 0.84);
    assert_eq!(p.w, 1e-6);
    assert!((p.l - 130e-9).abs() < 1e-15);
    assert_eq!(p.kf, 0.0);
    assert_eq!(p.af, 1.0);
    assert!(p.subthreshold_enable);
}

// ---------------------------------------------------------------------------
// Region detection
// ---------------------------------------------------------------------------

#[test]
fn test_saturation_region() {
    // V_GS = 1.8 V, V_DS = 1.8 V → V_OV = 1.38 V, V_DS ≥ V_OV → saturation
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    assert_eq!(r.region, Region::Saturation, "should be saturation");
    assert!(r.id > 0.0, "Id must be positive in saturation");
}

#[test]
fn test_triode_region() {
    // V_GS = 1.8, V_DS = 0.1 → V_OV ≈ 1.38, V_DS < V_OV → triode
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 1.8, 0.1, 0.0, 300.15);
    assert_eq!(r.region, Region::Triode, "should be triode");
    assert!(r.id > 0.0);
}

#[test]
fn test_cutoff_region() {
    // V_GS = 0 V → V_OV = -0.42 V → cutoff (subthreshold disabled)
    let p = Level1Params {
        subthreshold_enable: false,
        ..Level1Params::default()
    };
    let r = evaluate_level1(&p, 0.0, 1.0, 0.0, 300.15);
    assert_eq!(r.region, Region::Cutoff);
    assert_eq!(r.id, 0.0);
    assert_eq!(r.gm, 0.0);
}

#[test]
fn test_subthreshold_region() {
    // V_GS = 0.2 V < VT0 = 0.42 V → subthreshold when enabled
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 0.2, 1.0, 0.0, 300.15);
    assert_eq!(r.region, Region::Subthreshold);
    // Subthreshold current should be tiny but positive
    assert!(r.id > 0.0 && r.id < 1e-5, "sub-vt current={}", r.id);
}

// ---------------------------------------------------------------------------
// Transconductance and output conductance
// ---------------------------------------------------------------------------

#[test]
fn test_gm_positive_in_saturation() {
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    assert!(r.gm > 0.0, "gm must be positive");
}

#[test]
fn test_gds_positive_in_saturation() {
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    assert!(
        r.gds > 0.0,
        "gds must be positive (channel-length modulation)"
    );
}

#[test]
fn test_gds_without_clm() {
    // With lambda = 0, gds should be near 0 in saturation
    let p = Level1Params {
        lambda: 0.0,
        ..Level1Params::default()
    };
    let r = evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    assert!(r.gds.abs() < 1e-12, "gds≈0 when lambda=0");
}

// ---------------------------------------------------------------------------
// Body effect
// ---------------------------------------------------------------------------

#[test]
fn test_body_effect_raises_vt() {
    let p = Level1Params::default();
    // With V_BS = -1 (body below source), V_t should increase → Id drops
    let r0 = evaluate_level1(&p, 1.2, 1.2, 0.0, 300.15);
    let r1 = evaluate_level1(&p, 1.2, 1.2, -1.0, 300.15);
    assert!(r1.id < r0.id, "body effect should reduce Id");
}

#[test]
fn test_gmb_nonzero_with_body_bias() {
    let p = Level1Params::default();
    let r = evaluate_level1(&p, 1.8, 1.8, -0.5, 300.15);
    assert!(r.gmb > 0.0, "gmb must be positive with reverse body bias");
}

#[test]
fn test_bulk_junction_capacitance_uses_continuous_forward_bias_transition() {
    let params = Level1Params {
        cbs: 4.0e-12,
        pb: 1.0,
        mj: 0.5,
        fc: 0.4,
        subthreshold_enable: false,
        ..Level1Params::default()
    };
    let below = evaluate_level1(&params, 0.0, 0.0, 0.4 - 1.0e-9, 300.15).cbs;
    let above = evaluate_level1(&params, 0.0, 0.0, 0.4 + 1.0e-9, 300.15).cbs;
    let later_transition = evaluate_level1(
        &Level1Params {
            fc: 0.8,
            ..params.clone()
        },
        0.0,
        0.0,
        0.7,
        300.15,
    )
    .cbs;
    let early_transition = evaluate_level1(&params, 0.0, 0.0, 0.7, 300.15).cbs;

    assert!((below - above).abs() < 1.0e-19);
    assert!(later_transition > early_transition);
}

// ---------------------------------------------------------------------------
// PMOS wrapper sign conventions
// ---------------------------------------------------------------------------

#[test]
fn test_pmos_dc_negative_id() {
    // PMOS with V_GS = -1.8 V should give negative Id
    let p = Level1Params {
        vt0: 0.42, // PMOS threshold (magnitude)
        ..Level1Params::default()
    };
    let m = Mosfet::new(MosfetType::Pmos, p);
    let r = m.dc(-1.8, -1.8, 0.0, 300.15);
    assert!(r.id < 0.0, "PMOS Id should be negative: {}", r.id);
    assert_eq!(r.region, Region::Saturation);
}

#[test]
fn test_nmos_pmos_magnitude_match() {
    // |Id_NMOS(V_GS, V_DS)| = |Id_PMOS(-V_GS, -V_DS)| for same parameters
    let p = Level1Params::default();
    let nmos = Mosfet::new(MosfetType::Nmos, p.clone());
    let pmos = Mosfet::new(MosfetType::Pmos, p);
    let rn = nmos.dc(1.8, 1.8, 0.0, 300.15);
    let rp = pmos.dc(-1.8, -1.8, 0.0, 300.15);
    assert!(
        (rn.id + rp.id).abs() < 1e-12,
        "NMOS Id={} and -PMOS Id={} should match",
        rn.id,
        -rp.id
    );
}

// ---------------------------------------------------------------------------
// Level1Model struct
// ---------------------------------------------------------------------------

#[test]
fn test_level1_model_dc() {
    let model = Level1Model::new(Level1Params::default());
    let r = model.dc(1.8, 1.8, 0.0, 300.15);
    assert_eq!(r.region, Region::Saturation);
    assert!(r.id > 0.0);
}

// ---------------------------------------------------------------------------
// Region::as_str
// ---------------------------------------------------------------------------

#[test]
fn test_region_as_str() {
    assert_eq!(Region::Cutoff.as_str(), "cutoff");
    assert_eq!(Region::Subthreshold.as_str(), "subthreshold");
    assert_eq!(Region::Triode.as_str(), "triode");
    assert_eq!(Region::Saturation.as_str(), "saturation");
}

// ---------------------------------------------------------------------------
// Capacitance model
// ---------------------------------------------------------------------------

#[test]
fn test_capacitances_nonnegative() {
    let p = Level1Params::default();
    for (vgs, vds) in [(0.0, 0.0), (0.2, 0.5), (1.2, 0.1), (1.8, 1.8)] {
        let r = evaluate_level1(&p, vgs, vds, 0.0, 300.15);
        assert!(r.cgs >= 0.0, "Cgs < 0 at vgs={vgs}, vds={vds}");
        assert!(r.cgd >= 0.0, "Cgd < 0 at vgs={vgs}, vds={vds}");
        assert!(r.cgb >= 0.0, "Cgb < 0 at vgs={vgs}, vds={vds}");
    }
}

#[test]
fn test_overlap_caps_scale_with_w() {
    let p = Level1Params {
        cgso: 5e-10, // 0.5 nF/m gate-source overlap cap
        w: 2e-6,     // 2 µm wide
        ..Level1Params::default()
    };
    let r = evaluate_level1(&p, 1.8, 1.8, 0.0, 300.15);
    // Overlap alone: Cgs_overlap = cgso * W = 5e-10 * 2e-6 = 1e-15 F
    assert!(r.cgs > 5e-10 * 2e-6, "Cgs should include overlap cap");
}

// ---------------------------------------------------------------------------
// Saturation Id formula cross-check
// ---------------------------------------------------------------------------

#[test]
fn test_saturation_id_formula() {
    // With lambda = 0, Id = (KP/2)(W/L) V_OV²
    let p = Level1Params {
        lambda: 0.0,
        ..Level1Params::default()
    };
    let v_gs = 1.8_f64;
    let v_t = p.vt0; // V_BS=0 → V_t = VT0
    let v_ov = v_gs - v_t;
    let beta = p.kp * (p.w / p.l);
    let expected = (beta / 2.0) * v_ov * v_ov;
    let r = evaluate_level1(&p, v_gs, v_gs, 0.0, 300.15);
    let rel_err = (r.id - expected).abs() / expected;
    assert!(rel_err < 1e-9, "Id={} expected={expected}", r.id);
}
