use sky130_pdk::{load_sky130, PdkError, PdkProfile, LAYER_MAP, TEACHING_CELLS};

#[test]
fn test_teaching_load_succeeds_without_root() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    assert!(pdk.cells.len() >= 33);
}

#[test]
fn test_teaching_has_key_cells() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    for name in &[
        "sky130_fd_sc_hd__inv_1",
        "sky130_fd_sc_hd__buf_1",
        "sky130_fd_sc_hd__and2_1",
        "sky130_fd_sc_hd__or2_1",
        "sky130_fd_sc_hd__xor2_1",
        "sky130_fd_sc_hd__nand2_1",
        "sky130_fd_sc_hd__nor2_1",
        "sky130_fd_sc_hd__dfxtp_1",
        "sky130_fd_sc_hd__conb_1",
    ] {
        assert!(pdk.cells.contains_key(*name), "missing: {name}");
    }
}

#[test]
fn test_process_metadata_values() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    let m = &pdk.process;
    assert_eq!(m.feature_size_nm, 130);
    assert!((m.vdd_nominal - 1.8).abs() < 1e-9);
    assert_eq!(m.metal_layers, 6);
    assert!((m.cell_row_height_um - 2.72).abs() < 1e-6);
}

#[test]
fn test_layer_map_has_key_layers() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    for key in &["met1.drawing", "met1.pin", "poly.drawing", "li1.drawing", "met5.pin"] {
        assert!(pdk.layers.contains_key(*key), "missing layer: {key}");
    }
}

#[test]
fn test_met1_drawing_layer_number() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    let l = pdk.get_layer("met1.drawing").unwrap();
    assert_eq!(l.layer_number, 68);
    assert_eq!(l.datatype, 20);
}

#[test]
fn test_get_cell_returns_correct_entry() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    let c = pdk.get_cell("sky130_fd_sc_hd__inv_1").unwrap();
    assert_eq!(c.drive_strength, 1);
    assert!(c.function.contains("!A"));
}

#[test]
fn test_cell_names_sorted() {
    let pdk = load_sky130(PdkProfile::Teaching, None::<&str>).unwrap();
    let names = pdk.cell_names();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn test_full_profile_missing_root_errors() {
    let result = load_sky130(PdkProfile::Full, None::<&str>);
    assert!(matches!(result, Err(PdkError::MissingRoot)));
}

#[test]
fn test_full_profile_nonexistent_root_errors() {
    let result = load_sky130(PdkProfile::Full, Some("/nonexistent/sky130A/path/xyz"));
    assert!(matches!(result, Err(PdkError::InstallNotFound(_))));
}

#[test]
fn test_teaching_cells_static_count() {
    assert!(TEACHING_CELLS.len() >= 33);
}

#[test]
fn test_layer_map_static_count() {
    assert!(LAYER_MAP.len() >= 20);
}

#[test]
fn test_dfxtp_height_tracks() {
    let c = &TEACHING_CELLS["sky130_fd_sc_hd__dfxtp_1"];
    assert_eq!(c.height_tracks, 9);
}
