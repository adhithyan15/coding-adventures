use tape_out::{
    render_manifest, render_readme, validate_for_chipignite, PadLocation, Shuttle,
    TapeoutBundle, TapeoutMetadata,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn full_bundle() -> TapeoutBundle {
    let meta = TapeoutMetadata {
        project_name: "adder4".into(),
        designer: "Alice".into(),
        email: "alice@example.com".into(),
        top_module: "adder4".into(),
        ..TapeoutMetadata::default()
    };
    let mut b = TapeoutBundle::new(meta);
    b.signoff.insert("drc".into(), "clean".into());
    b.signoff.insert("lvs".into(), "clean".into());
    for f in ["gds", "lef", "def", "verilog", "drc_report", "lvs_report"] {
        b.files.insert(f.into(), format!("adder4.{f}"));
    }
    b
}

// ---------------------------------------------------------------------------
// Validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_valid_bundle_passes() {
    let b = full_bundle();
    let r = validate_for_chipignite(&b);
    assert!(r.passed, "errors: {:?}", r.errors);
    assert!(r.errors.is_empty());
}

#[test]
fn test_missing_project_name_fails() {
    let meta = TapeoutMetadata { project_name: "".into(), ..TapeoutMetadata::default() };
    let b = TapeoutBundle::new(meta);
    let r = validate_for_chipignite(&b);
    assert!(!r.passed);
    assert!(r.errors.iter().any(|e| e.contains("project_name")));
}

#[test]
fn test_missing_designer_fails() {
    let meta = TapeoutMetadata {
        project_name: "x".into(), email: "e@e.com".into(), top_module: "x".into(),
        ..TapeoutMetadata::default()
    };
    let b = TapeoutBundle::new(meta);
    let r = validate_for_chipignite(&b);
    assert!(r.errors.iter().any(|e| e.contains("designer")));
}

#[test]
fn test_missing_required_file_fails() {
    let mut b = full_bundle();
    b.files.remove("gds");
    let r = validate_for_chipignite(&b);
    assert!(!r.passed);
    assert!(r.errors.iter().any(|e| e.contains("gds")));
}

#[test]
fn test_dirty_drc_fails() {
    let mut b = full_bundle();
    b.signoff.insert("drc".into(), "5 errors".into());
    let r = validate_for_chipignite(&b);
    assert!(!r.passed);
    assert!(r.errors.iter().any(|e| e.contains("DRC")));
}

#[test]
fn test_missing_lvs_fails() {
    let mut b = full_bundle();
    b.signoff.remove("lvs");
    let r = validate_for_chipignite(&b);
    assert!(!r.passed);
    assert!(r.errors.iter().any(|e| e.contains("LVS")));
}

#[test]
fn test_no_pad_locations_warning_for_open_mpw() {
    let b = full_bundle(); // default shuttle = OpenMpw, no pads
    let r = validate_for_chipignite(&b);
    // Still passes (warning, not error).
    assert!(r.passed);
    assert!(!r.warnings.is_empty());
}

#[test]
fn test_tiny_tapeout_no_pad_warning() {
    let meta = TapeoutMetadata {
        project_name: "x".into(), designer: "B".into(), email: "b@b.com".into(),
        top_module: "x".into(), shuttle: Shuttle::TinyTapeout,
        ..TapeoutMetadata::default()
    };
    let mut b = TapeoutBundle::new(meta);
    b.signoff.insert("drc".into(), "clean".into());
    b.signoff.insert("lvs".into(), "clean".into());
    for f in ["gds", "lef", "def", "verilog", "drc_report", "lvs_report"] {
        b.files.insert(f.into(), format!("x.{f}"));
    }
    let r = validate_for_chipignite(&b);
    assert!(r.passed);
    assert!(r.warnings.is_empty()); // TinyTapeout does not require pad_locations
}

// ---------------------------------------------------------------------------
// render_manifest tests
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_contains_project_name() {
    let b = full_bundle();
    let m = render_manifest(&b);
    assert!(m.contains("project_name: adder4"), "got:\n{m}");
}

#[test]
fn test_manifest_contains_shuttle() {
    let b = full_bundle();
    let m = render_manifest(&b);
    assert!(m.contains("shuttle: chipignite_open_mpw"));
}

#[test]
fn test_manifest_contains_signoff() {
    let b = full_bundle();
    let m = render_manifest(&b);
    assert!(m.contains("signoff:"));
    assert!(m.contains("drc: clean"));
    assert!(m.contains("lvs: clean"));
}

#[test]
fn test_manifest_contains_pads_when_present() {
    let mut b = full_bundle();
    b.pad_locations.push(PadLocation { name: "clk".into(), direction: "input".into(), x: 10.0, y: 0.0 });
    let m = render_manifest(&b);
    assert!(m.contains("pads:"));
    assert!(m.contains("clk"));
}

#[test]
fn test_manifest_pdk_version_optional() {
    let mut b = full_bundle();
    b.metadata.pdk_version = Some("1.0.0".into());
    let m = render_manifest(&b);
    assert!(m.contains("pdk_version: 1.0.0"));
}

// ---------------------------------------------------------------------------
// render_readme tests
// ---------------------------------------------------------------------------

#[test]
fn test_readme_contains_project_name() {
    let b = full_bundle();
    let r = render_readme(&b);
    assert!(r.starts_with("# adder4"));
}

#[test]
fn test_readme_contains_files_section() {
    let b = full_bundle();
    let r = render_readme(&b);
    assert!(r.contains("## Files"));
    assert!(r.contains("gds"));
}
