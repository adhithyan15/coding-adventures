//! package_compiles — smoke test for the mosaic-pkg-calendar package.
//!
//! Mirrors mosaic-pkg-sheet's own `package_compiles.rs`: manifest parses and
//! declares the expected export; Calendar.mil compiles via mosmodel;
//! Calendar.mll compiles against that interface via moslayout;
//! Calendar.dark.msl / Calendar.light.msl each compile against the resulting
//! part map via mosstyle. Unlike Sheet, Calendar has no cross-package
//! dependency to pin (it's built entirely from kernel primitives, like
//! mosaic-pkg-grid), so there's no `pkg::` qualified-reference regression
//! test here.

use std::fs;
use std::path::PathBuf;

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as valid TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-calendar", "[package].name mismatch");

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Calendar"],
        "[components].exports must list exactly Calendar"
    );

    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1", "[kernel].version must target UI29 kernel v1");
}

// ---------------------------------------------------------------------------
// 2. mosmodel — .mil compilation
// ---------------------------------------------------------------------------

#[test]
fn calendar_mil_compiles() {
    let src = read_source("Calendar.mil");
    let out = mosmodel_compiler::compile(&src)
        .unwrap_or_else(|e| panic!("Calendar.mil failed to compile via mosmodel_compiler:\n{:#?}", e));
    assert_eq!(out.component.component, "Calendar");

    let slot_names: Vec<&str> = out.component.slots.iter().map(|s| s.name.as_str()).collect();
    for expected in ["calendar-title", "calendar-cells", "calendar-events"] {
        assert!(
            slot_names.contains(&expected),
            "Calendar.mil must declare slot `{}`, got: {:?}",
            expected,
            slot_names
        );
    }

    let emit_names: Vec<&str> = out.component.emits.iter().map(|e| e.name.as_str()).collect();
    for expected in ["onPrev", "onNext", "onEventDropped"] {
        assert!(
            emit_names.contains(&expected),
            "Calendar.mil must declare emit `{}`, got: {:?}",
            expected,
            emit_names
        );
    }
}

// ---------------------------------------------------------------------------
// 3. moslayout — .mll compilation
// ---------------------------------------------------------------------------

#[test]
fn calendar_mll_compiles_against_its_interface() {
    let mil_src = read_source("Calendar.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|e| panic!("Calendar.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Calendar.mll");
    moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Calendar.mll failed to compile via moslayout_compiler:\n{:#?}", e));
}

#[test]
fn calendar_mll_uses_the_drag_kernel() {
    // Pins the UI35 drag-kernel usage this package depends on — a regression
    // guard against accidentally degrading drag-to-move into a plain static
    // container.
    let src = read_source("Calendar.mll");
    assert!(src.contains("HostDropTarget"), "Calendar.mll must use HostDropTarget");
    assert!(src.contains("HostDraggable"), "Calendar.mll must use HostDraggable");
}

// ---------------------------------------------------------------------------
// 4. mosstyle — .msl compilation
// ---------------------------------------------------------------------------

#[test]
fn each_msl_theme_compiles_against_the_part_map() {
    let mll_src = read_source("Calendar.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Calendar.mll precompile failed: {:#?}", e));

    for theme in ["Calendar.dark.msl", "Calendar.light.msl"] {
        let msl_path = src_path(theme);
        assert!(msl_path.exists(), "{} must exist", msl_path.display());

        let msl_src = read_source(theme);
        mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json))
            .unwrap_or_else(|e| panic!("{} failed to compile via mosstyle_compiler:\n{:#?}", theme, e));
    }
}

// ---------------------------------------------------------------------------
// 5. Source-tree sanity
// ---------------------------------------------------------------------------

#[test]
fn source_tree_has_expected_shape() {
    for name in [
        "Calendar.mil",
        "Calendar.mll",
        "Calendar.dark.msl",
        "Calendar.light.msl",
    ] {
        let path = src_path(name);
        assert!(path.exists(), "expected package source file missing: {}", path.display());
    }
}
