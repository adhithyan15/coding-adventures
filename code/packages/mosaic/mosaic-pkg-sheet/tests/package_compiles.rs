//! package_compiles — smoke test for the mosaic-pkg-sheet package.
//!
//! Mirrors mosaic-pkg-grid's own `package_compiles.rs` (same four-step
//! shape): manifest parses and declares the expected export; Sheet.mil
//! compiles via mosmodel; Sheet.mll compiles against that interface via
//! moslayout; Sheet.dark.msl / Sheet.light.msl each compile against the
//! resulting part map via mosstyle.
//!
//! Cross-package references stay unresolved at this layer (UI34 §5 —
//! `pkg::P::C` substitution is `mosaic-compile`'s job, not
//! `moslayout_compiler::compile`'s), so `pkg::mosaic-pkg-grid::Grid`
//! and `pkg::mosaic-pkg-toolkit::Select` are validated here only as
//! opaque tags whose props reference Sheet's OWN slots/emits — exactly
//! the same lenient layer Grid.mll's own `Cell` reference is validated
//! at. Full end-to-end package resolution is exercised transitively
//! once task-app depends on this package and its own build script runs
//! `mosaic-compile pkg` (see task-app's `tests/package_compiles.rs`
//! and `scripts/build-web.sh`).

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
fn manifest_declares_expected_exports_and_deps() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as valid TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-sheet", "[package].name mismatch");

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Sheet"],
        "[components].exports must list exactly Sheet"
    );

    let deps = value
        .get("dependencies")
        .and_then(|d| d.as_table())
        .expect("[dependencies] must be a table");
    assert!(
        deps.contains_key("mosaic-pkg-grid") && deps.contains_key("mosaic-pkg-toolkit"),
        "Sheet must declare its dependency on mosaic-pkg-grid and mosaic-pkg-toolkit, got: {:?}",
        deps.keys().collect::<Vec<_>>()
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
fn sheet_mil_compiles() {
    let src = read_source("Sheet.mil");
    let out = mosmodel_compiler::compile(&src)
        .unwrap_or_else(|e| panic!("Sheet.mil failed to compile via mosmodel_compiler:\n{:#?}", e));
    assert_eq!(out.component.component, "Sheet");

    let slot_names: Vec<&str> = out.component.slots.iter().map(|s| s.name.as_str()).collect();
    for expected in [
        "viewport-rows",
        "column-headers",
        "column-widths",
        "selected-row",
        "selected-col",
        "edit-row",
        "edit-col",
        "edit-content",
        "filter-text",
        "sort-field",
        "sort-options",
        "sort-open",
        "sort-ascending",
    ] {
        assert!(
            slot_names.contains(&expected),
            "Sheet.mil must declare slot `{}`, got: {:?}",
            expected,
            slot_names
        );
    }

    let emit_names: Vec<&str> = out.component.emits.iter().map(|e| e.name.as_str()).collect();
    for expected in [
        "onNavigate",
        "onFormulaChange",
        "onEditCommit",
        "onEditCancel",
        "onFilterChange",
        "onSortFieldChange",
        "onToggleSortOpen",
        "onToggleSortDirection",
    ] {
        assert!(
            emit_names.contains(&expected),
            "Sheet.mil must declare emit `{}`, got: {:?}",
            expected,
            emit_names
        );
    }
}

// ---------------------------------------------------------------------------
// 3. moslayout — .mll compilation
// ---------------------------------------------------------------------------

#[test]
fn sheet_mll_compiles_against_its_interface() {
    let mil_src = read_source("Sheet.mil");
    let mil_out =
        mosmodel_compiler::compile(&mil_src).unwrap_or_else(|e| panic!("Sheet.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Sheet.mll");
    moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Sheet.mll failed to compile via moslayout_compiler:\n{:#?}", e));
}

#[test]
fn sheet_mll_references_grid_and_select_via_qualified_pkg_tags() {
    // Pins the UI34 cross-package reference shape this package depends on —
    // if a future refactor accidentally inlines Grid/Select source instead
    // of referencing them, this is the regression guard.
    let src = read_source("Sheet.mll");
    assert!(
        src.contains("pkg::mosaic-pkg-grid::Grid"),
        "Sheet.mll must reference Grid via the qualified pkg::P::C form"
    );
    assert!(
        src.contains("pkg::mosaic-pkg-toolkit::Select"),
        "Sheet.mll must reference Select via the qualified pkg::P::C form"
    );
}

// ---------------------------------------------------------------------------
// 4. mosstyle — .msl compilation
// ---------------------------------------------------------------------------

#[test]
fn each_msl_theme_compiles_against_the_part_map() {
    let mll_src = read_source("Sheet.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Sheet.mll precompile failed: {:#?}", e));

    for theme in ["Sheet.dark.msl", "Sheet.light.msl"] {
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
        "Sheet.mil",
        "Sheet.mll",
        "Sheet.dark.msl",
        "Sheet.light.msl",
    ] {
        let path = src_path(name);
        assert!(path.exists(), "expected package source file missing: {}", path.display());
    }
}
