//! package_compiles — smoke test for the mosaic-pkg-toolkit package.
//!
//! Asserts that every exported component (`Button`, `Alert` in v0.1
//! PR-1) compiles through the three IR compilers (mosmodel /
//! moslayout / mosstyle) and that the manifest is internally
//! consistent.
//!
//! What this test verifies:
//!
//!   1. The manifest parses, names the package correctly, declares
//!      every exported component in `[components].exports`, lists no
//!      runtime dependencies (the toolkit is kernel-only), and
//!      targets kernel v1.
//!   2. For each exported component:
//!      a. `<Component>.mil` compiles via `mosmodel_compiler::compile`.
//!      b. `<Component>.mll` compiles via `moslayout_compiler::compile`,
//!         validated against the matching `.mil` interface.
//!      c. `<Component>.light.msl` and `<Component>.dark.msl` each
//!         compile via `mosstyle_compiler::compile`, validated
//!         against the matching `.mll` part map.
//!
//! Backend-specific lowering (does each component produce valid
//! React/SwiftUI/Qt/XAML/etc.?) is NOT in scope here — that lives in
//! per-backend integration tests (`mosaic-emit-xaml/tests/`,
//! `mosaic-emit-react/tests/`, …) and lands as each backend gets
//! per-toolkit-component coverage.

use std::fs;
use std::path::PathBuf;

/// The list of exported components. Grows as each Tier-1 component
/// lands. v0.1 PR-1: Button, Alert. v0.1 PR-2 adds: Badge, Spinner,
/// Toast.
///
/// Alphabetical order matches the manifest's `[components].exports`
/// list. Reorder both together if it ever changes.
const COMPONENTS: &[&str] = &["Alert", "Badge", "Button", "Spinner", "Toast"];

/// Themes shipped per component. Both must compile.
const THEMES: &[&str] = &["light", "dark"];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value = toml::from_str(&manifest_src)
        .expect("mosaic-package.toml must parse as valid TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-toolkit", "[package].name mismatch");

    let version = value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[package].version must be set");
    assert_eq!(
        version, "0.1.0",
        "[package].version must be 0.1.0 for the v0.1 PR-1 release"
    );

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        COMPONENTS,
        "[components].exports must match the compile-loop's list"
    );

    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(
        kernel_version, "1",
        "[kernel].version must target UI29 kernel v1"
    );

    assert!(
        value.get("dependencies").is_some(),
        "[dependencies] table must be present (may be empty)"
    );
    let deps = value
        .get("dependencies")
        .and_then(|d| d.as_table())
        .expect("[dependencies] must be a TOML table");
    assert!(
        deps.is_empty(),
        "[dependencies] must be empty — toolkit is built only from UI29 kernel primitives; got {:?}",
        deps
    );
}

// ---------------------------------------------------------------------------
// 2. Per-component round-trip
// ---------------------------------------------------------------------------

/// Compile one component's .mil + .mll + each .msl. Asserts everything
/// round-trips and the .mll's name matches the .mil's.
fn compile_component(name: &str) {
    // .mil
    let mil_src = read_source(&format!("{name}.mil"));
    let mil_out = mosmodel_compiler::compile(&mil_src).unwrap_or_else(|e| {
        panic!("{name}.mil failed to compile:\n{:#?}", e)
    });
    assert_eq!(
        mil_out.component.component, name,
        "{name}.mil must declare component named {name:?}, got {:?}",
        mil_out.component.component
    );

    // .mll, validated against the .mil's descriptor.
    let mll_src = read_source(&format!("{name}.mll"));
    let mll_out =
        moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
            .unwrap_or_else(|e| panic!("{name}.mll failed to compile:\n{:#?}", e));
    assert_eq!(
        mll_out.def.component_name, name,
        "{name}.mll must declare layout for {name:?}"
    );

    // Both .msl files, validated against the .mll's part map.
    for theme in THEMES {
        let msl_filename = format!("{name}.{theme}.msl");
        let msl_src = read_source(&msl_filename);
        mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json))
            .unwrap_or_else(|e| {
                panic!("{msl_filename} failed to compile:\n{:#?}", e)
            });
    }
}

#[test]
fn every_exported_component_round_trips() {
    for name in COMPONENTS {
        compile_component(name);
    }
}

// ---------------------------------------------------------------------------
// 3. Per-component sanity checks — light bound on the surface
// ---------------------------------------------------------------------------

/// Button must have the documented slot/emit surface.
#[test]
fn button_interface_matches_spec() {
    let mil_src = read_source("Button.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;

    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["label", "variant", "size", "disabled"]);

    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onClick"]);
}

/// Alert must have the documented slot/emit surface.
#[test]
fn alert_interface_matches_spec() {
    let mil_src = read_source("Alert.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;

    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["message", "variant", "dismissible"]);

    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onClose"]);
}

/// Badge — pill label, slot-driven variant. No emits.
#[test]
fn badge_interface_matches_spec() {
    let mil_src = read_source("Badge.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["label", "variant"]);
    assert!(c.emits.is_empty(), "Badge has no emits");
}

/// Spinner — display-only loading indicator. No emits.
#[test]
fn spinner_interface_matches_spec() {
    let mil_src = read_source("Spinner.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["size", "variant", "aria-label"]);
    assert!(c.emits.is_empty(), "Spinner has no emits");
}

/// Toast — bottom-anchored notification with title/message/open/variant
/// slots and onClose emit. The `open` slot is a bool that drives an
/// `If` block in the .mll.
#[test]
fn toast_interface_matches_spec() {
    let mil_src = read_source("Toast.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["title", "message", "variant", "open"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onClose"]);
}
