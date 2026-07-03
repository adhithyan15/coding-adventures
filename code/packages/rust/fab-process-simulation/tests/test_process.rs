use fab_process_simulation::*;

fn bare_silicon(thickness_nm: f64) -> CrossSection {
    CrossSection {
        layers: vec![Layer::new("Si", thickness_nm)],
    }
}

// ---------------------------------------------------------------------------
// deal_grove_oxidation
// ---------------------------------------------------------------------------

#[test]
fn test_oxidation_creates_sio2_layer() {
    let cs = bare_silicon(500.0);
    let cs2 = deal_grove_oxidation(&cs, 60.0, None, None).unwrap();
    assert_eq!(cs2.layers[0].material, "SiO2");
    assert!(cs2.layers[0].thickness_nm > 0.0);
}

#[test]
fn test_oxidation_preserves_underlying_layers() {
    let cs = bare_silicon(500.0);
    let cs2 = deal_grove_oxidation(&cs, 60.0, None, None).unwrap();
    assert!(cs2.layers.iter().any(|l| l.material == "Si"));
}

#[test]
fn test_oxidation_longer_time_grows_more() {
    let cs = bare_silicon(500.0);
    let t1 = deal_grove_oxidation(&cs, 30.0, None, None).unwrap().layers[0].thickness_nm;
    let t2 = deal_grove_oxidation(&cs, 120.0, None, None).unwrap().layers[0].thickness_nm;
    assert!(t2 > t1, "longer oxidation must produce thicker oxide: {t2} vs {t1}");
}

#[test]
fn test_oxidation_existing_oxide_continues_growth() {
    let cs = bare_silicon(500.0);
    let cs2 = deal_grove_oxidation(&cs, 30.0, None, None).unwrap();
    let t_after_first = cs2.layers[0].thickness_nm;
    let cs3 = deal_grove_oxidation(&cs2, 30.0, None, None).unwrap();
    let t_after_second = cs3.layers[0].thickness_nm;
    assert!(t_after_second > t_after_first, "second oxidation must add more oxide");
    // Should still only have one SiO2 layer (merged with existing).
    assert_eq!(
        cs3.layers.iter().filter(|l| l.material == "SiO2").count(),
        1
    );
}

#[test]
fn test_oxidation_zero_time_error() {
    let cs = bare_silicon(500.0);
    assert!(deal_grove_oxidation(&cs, 0.0, None, None).is_err());
}

// ---------------------------------------------------------------------------
// deposit
// ---------------------------------------------------------------------------

#[test]
fn test_deposit_adds_top_layer() {
    let cs = bare_silicon(500.0);
    let cs2 = deposit(&cs, "Poly", 100.0).unwrap();
    assert_eq!(cs2.layers[0].material, "Poly");
    assert_eq!(cs2.layers[0].thickness_nm, 100.0);
    assert_eq!(cs2.layers[1].material, "Si");
}

#[test]
fn test_deposit_negative_thickness_error() {
    let cs = bare_silicon(500.0);
    assert!(deposit(&cs, "Poly", -1.0).is_err());
    assert!(deposit(&cs, "Poly", 0.0).is_err());
}

#[test]
fn test_deposit_multiple_layers() {
    let cs = bare_silicon(500.0);
    let cs2 = deposit(&cs, "SiO2", 10.0).unwrap();
    let cs3 = deposit(&cs2, "Poly", 50.0).unwrap();
    assert_eq!(cs3.layers[0].material, "Poly");
    assert_eq!(cs3.layers[1].material, "SiO2");
    assert_eq!(cs3.layers[2].material, "Si");
}

// ---------------------------------------------------------------------------
// etch
// ---------------------------------------------------------------------------

#[test]
fn test_etch_partial_layer() {
    let cs = CrossSection {
        layers: vec![Layer::new("SiO2", 20.0), Layer::new("Si", 500.0)],
    };
    let cs2 = etch(&cs, "SiO2", 5.0);
    assert_eq!(cs2.layers[0].material, "SiO2");
    assert!((cs2.layers[0].thickness_nm - 15.0).abs() < 1e-9);
}

#[test]
fn test_etch_complete_layer() {
    let cs = CrossSection {
        layers: vec![Layer::new("SiO2", 20.0), Layer::new("Si", 500.0)],
    };
    let cs2 = etch(&cs, "SiO2", 20.0);
    assert_eq!(cs2.layers[0].material, "Si");
}

#[test]
fn test_etch_stops_at_different_material() {
    let cs = CrossSection {
        layers: vec![Layer::new("SiO2", 5.0), Layer::new("Si", 500.0)],
    };
    // Etching more than the top SiO2 but only targeting SiO2 — Si is preserved.
    let cs2 = etch(&cs, "SiO2", 100.0);
    assert_eq!(cs2.layers[0].material, "Si");
    assert!((cs2.layers[0].thickness_nm - 500.0).abs() < 1e-9);
}

#[test]
fn test_etch_zero_depth_no_change() {
    let cs = bare_silicon(500.0);
    let cs2 = etch(&cs, "Si", 0.0);
    assert_eq!(cs2, cs);
}

#[test]
fn test_etch_wrong_material_no_change() {
    let cs = bare_silicon(500.0);
    let cs2 = etch(&cs, "SiO2", 50.0);
    assert_eq!(cs2.layers[0].material, "Si"); // no oxide to etch
}

// ---------------------------------------------------------------------------
// implant
// ---------------------------------------------------------------------------

#[test]
fn test_implant_boron_into_si() {
    let cs = bare_silicon(500.0);
    let cs2 = implant(&cs, "B", 30.0, 1e13).unwrap();
    let si = &cs2.layers[0];
    assert!(si.doping.contains_key("B"), "Si layer should have B doping");
    let profile = &si.doping["B"];
    assert!(!profile.is_empty(), "doping profile must not be empty");
    // All concentrations should be positive
    for &(_, conc) in profile {
        assert!(conc >= 0.0, "negative concentration");
    }
}

#[test]
fn test_implant_peak_near_rp() {
    let cs = bare_silicon(500.0);
    let cs2 = implant(&cs, "B", 30.0, 1e14).unwrap();
    let profile = &cs2.layers[0].doping["B"];
    let (rp, _) = implant_range("B", 30.0).unwrap();
    // Find the sample closest to Rp.
    let peak_conc = profile
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap()
        .1;
    let at_rp = profile
        .iter()
        .min_by_key(|(x, _)| (((x - rp).abs()) * 1e6) as u64)
        .unwrap()
        .1;
    assert!(
        at_rp / peak_conc > 0.5,
        "sample at Rp should be near the peak"
    );
}

#[test]
fn test_implant_unknown_species_error() {
    let cs = bare_silicon(500.0);
    assert!(implant(&cs, "Xe", 100.0, 1e13).is_err());
}

#[test]
fn test_implant_bad_dose_error() {
    let cs = bare_silicon(500.0);
    assert!(implant(&cs, "B", 30.0, 0.0).is_err());
}

#[test]
fn test_implant_no_si_layer_error() {
    let cs = CrossSection {
        layers: vec![Layer::new("SiO2", 10.0)],
    };
    assert!(implant(&cs, "B", 30.0, 1e13).is_err());
}

// ---------------------------------------------------------------------------
// diffuse
// ---------------------------------------------------------------------------

#[test]
fn test_diffuse_preserves_doping_entry() {
    let cs = bare_silicon(500.0);
    let cs2 = implant(&cs, "B", 30.0, 1e13).unwrap();
    let cs3 = diffuse(&cs2, 30.0, None);
    assert!(cs3.layers[0].doping.contains_key("B"));
}

#[test]
fn test_diffuse_no_doping_unchanged() {
    let cs = bare_silicon(500.0);
    let cs2 = diffuse(&cs, 60.0, None);
    assert!(cs2.layers[0].doping.is_empty());
}

// ---------------------------------------------------------------------------
// implant_range
// ---------------------------------------------------------------------------

#[test]
fn test_implant_range_exact_b_10kev() {
    let (rp, std) = implant_range("B", 10.0).unwrap();
    assert!((rp - 33.0).abs() < 1e-9);
    assert!((std - 18.0).abs() < 1e-9);
}

#[test]
fn test_implant_range_interpolation() {
    // B at 20 keV should interpolate between 10 keV and 30 keV entries.
    let (rp10, _) = implant_range("B", 10.0).unwrap();
    let (rp30, _) = implant_range("B", 30.0).unwrap();
    let (rp20, _) = implant_range("B", 20.0).unwrap();
    assert!(rp20 > rp10 && rp20 < rp30, "interpolation out of range: {rp20}");
}

#[test]
fn test_implant_range_extrapolation_below() {
    let (rp, _) = implant_range("B", 5.0).unwrap();
    assert!(rp > 0.0, "extrapolated range must be positive");
}

#[test]
fn test_implant_range_extrapolation_above() {
    let (rp, _) = implant_range("B", 200.0).unwrap();
    assert!(rp > 260.0, "extrapolated range above 100 keV should exceed 260 nm");
}

// ---------------------------------------------------------------------------
// diffusivity
// ---------------------------------------------------------------------------

#[test]
fn test_diffusivity_known_species() {
    let d_b = diffusivity_cm2_per_s("B", 1000.0);
    let d_p = diffusivity_cm2_per_s("P", 1000.0);
    let d_as = diffusivity_cm2_per_s("As", 1000.0);
    // At 1000 °C: B ~ 1e-14, P ~ 1.2e-14, As ~ 4e-15
    assert!((d_b - 1e-14).abs() < 1e-16, "D_B={d_b}");
    assert!((d_p - 1.2e-14).abs() < 1e-16, "D_P={d_p}");
    assert!((d_as - 4e-15).abs() < 1e-17, "D_As={d_as}");
}

#[test]
fn test_diffusivity_increases_with_temperature() {
    let d1 = diffusivity_cm2_per_s("B", 900.0);
    let d2 = diffusivity_cm2_per_s("B", 1100.0);
    assert!(d2 > d1, "diffusivity must increase with temperature");
}
