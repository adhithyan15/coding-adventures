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
/// lands. v0.1 PR-1: Button, Alert. PR-2: Badge, Spinner, Toast.
/// PR-3: Checkbox, Input, Radio. PR-4 adds: ListGroup, Modal.
///
/// Alphabetical order matches the manifest's `[components].exports`
/// list. Reorder both together if it ever changes.
const COMPONENTS: &[&str] = &[
    "Accordion", "Alert", "Badge", "Breadcrumb", "Button", "ButtonGroup",
    "Checkbox", "DropdownMenu", "Field", "Input", "InputGroup",
    "ListGroup", "Modal", "Nav", "Navbar", "NumberInput", "Pagination",
    "Radio", "Select", "Spinner", "Tabs", "Toast", "Tooltip",
];

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
        version, "0.11.0",
        "[package].version must be 0.11.0 for the Select release"
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

/// Input — text input with onChange (text payload) + onCommit.
#[test]
fn input_interface_matches_spec() {
    let mil_src = read_source("Input.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["value", "placeholder", "disabled", "size"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onChange", "onCommit"]);
}

/// Checkbox v0.3 — native `HostCheckbox` wrapper with optional
/// `indeterminate` slot. Breaking changes from v0.2 documented in
/// CHANGELOG: `onChange` now carries `checked: bool`, and the slot
/// roster grew an `indeterminate` slot.
#[test]
fn checkbox_interface_matches_spec() {
    let mil_src = read_source("Checkbox.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["label", "checked", "disabled", "indeterminate"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onChange"]);
    // v0.3 — onChange now carries a `checked: bool` parameter.
    let on_change = c.emits.iter().find(|e| e.name == "onChange").unwrap();
    let param_names: Vec<&str> = on_change.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, vec!["checked"]);
}

/// Radio v0.3 — native `HostRadio` wrapper. Breaking changes from
/// v0.2: `selected` renamed to `checked` (consistent with HostRadio
/// and Checkbox); new `value` and `group` slots; `onSelect` carries
/// `value: text` payload.
#[test]
fn radio_interface_matches_spec() {
    let mil_src = read_source("Radio.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["label", "checked", "value", "group", "disabled"]
    );
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
    // v0.3 — onSelect now carries a `value: text` parameter.
    let on_select = c.emits.iter().find(|e| e.name == "onSelect").unwrap();
    let param_names: Vec<&str> = on_select.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, vec!["value"]);
}

/// ListGroup — vertical list of selectable text rows. Iterates via For.
#[test]
fn listgroup_interface_matches_spec() {
    let mil_src = read_source("ListGroup.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["items", "selected-index"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// ListGroup should render the selected row through a distinct part,
/// not merely expose an unused selected-index slot.
#[test]
fn listgroup_selected_row_part_compiles_and_is_styled() {
    let mil_src = read_source("ListGroup.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src).unwrap();

    let mll_src = read_source("ListGroup.mll");
    assert!(
        mll_src.contains("i == selectedIndex"),
        "ListGroup.mll should compare the loop index with selectedIndex"
    );
    assert!(
        mll_src.contains("list-group-item-selected"),
        "ListGroup.mll should route the selected row through its own part"
    );

    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .expect("ListGroup.mll should compile against ListGroup.mil");
    assert!(
        mll_out.part_map_json.contains("list-group-item-selected"),
        "ListGroup part map should include the selected row part"
    );

    for theme in THEMES {
        let style_filename = format!("ListGroup.{theme}.msl");
        let style_src = read_source(&style_filename);
        let style_out = mosstyle_compiler::compile(&style_src, Some(&mll_out.part_map_json))
            .unwrap_or_else(|e| panic!("{style_filename} failed to compile:\n{:#?}", e));
        let selected = style_out
            .def
            .parts
            .iter()
            .find(|part| part.name == "list-group-item-selected")
            .unwrap_or_else(|| panic!("{style_filename} missing selected row part"));
        let background = selected
            .base
            .iter()
            .find(|prop| prop.name == "background")
            .unwrap_or_else(|| panic!("{style_filename} selected row missing background"));
        let expected_background = if *theme == "light" {
            "#e7f1ff"
        } else {
            "#1d4ed8"
        };
        assert_eq!(
            background.value, expected_background,
            "{style_filename} selected row background mismatch"
        );
    }
}

/// Modal — wraps HostDialog with title + message slots.
#[test]
fn modal_interface_matches_spec() {
    let mil_src = read_source("Modal.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["title", "message", "open", "close-label"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onClose"]);
}

/// Field — label + HostInput + help/error. The label/help/error
/// slots are text; value/placeholder are text; disabled is bool.
#[test]
fn field_interface_matches_spec() {
    let mil_src = read_source("Field.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["label", "value", "placeholder", "help", "error", "disabled"]
    );
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onChange", "onCommit"]);
}

/// Nav — horizontal list of nav links. Same shape as ListGroup
/// (items + selected-style index + onSelect with index payload),
/// laid out horizontally via Row.
#[test]
fn nav_interface_matches_spec() {
    let mil_src = read_source("Nav.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["items", "active-index"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// Nav should render the active link through a distinct part,
/// not merely expose an unused active-index slot.
#[test]
fn nav_active_link_part_compiles_and_is_styled() {
    let mil_src = read_source("Nav.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src).unwrap();

    let mll_src = read_source("Nav.mll");
    assert!(
        mll_src.contains("i == activeIndex"),
        "Nav.mll should compare the loop index with activeIndex"
    );
    assert!(
        mll_src.contains("nav-link-active"),
        "Nav.mll should route the active link through its own part"
    );

    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .expect("Nav.mll should compile against Nav.mil");
    assert!(
        mll_out.part_map_json.contains("nav-link-active"),
        "Nav part map should include the active link part"
    );

    for theme in THEMES {
        let style_filename = format!("Nav.{theme}.msl");
        let style_src = read_source(&style_filename);
        let style_out = mosstyle_compiler::compile(&style_src, Some(&mll_out.part_map_json))
            .unwrap_or_else(|e| panic!("{style_filename} failed to compile:\n{:#?}", e));
        let active = style_out
            .def
            .parts
            .iter()
            .find(|part| part.name == "nav-link-active")
            .unwrap_or_else(|| panic!("{style_filename} missing active link part"));
        let background = active
            .base
            .iter()
            .find(|prop| prop.name == "background")
            .unwrap_or_else(|| panic!("{style_filename} active link missing background"));
        let expected_background = if *theme == "light" {
            "#e7f1ff"
        } else {
            "#1d4ed8"
        };
        assert_eq!(
            background.value, expected_background,
            "{style_filename} active link background mismatch"
        );
    }
}

/// ButtonGroup — row of related buttons that visually share borders.
#[test]
fn button_group_interface_matches_spec() {
    let mil_src = read_source("ButtonGroup.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["items"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// Breadcrumb — hierarchical nav trail. Same For-over-list pattern.
#[test]
fn breadcrumb_interface_matches_spec() {
    let mil_src = read_source("Breadcrumb.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["crumbs"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// Pagination — « prev | 1 2 3 | next » row of HostLink chips. Three
/// emits: onPrev, onNext (no payload), onPageSelect(index).
#[test]
fn pagination_interface_matches_spec() {
    let mil_src = read_source("Pagination.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["pages", "prev-label", "next-label", "active-index"]
    );
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onPrev", "onNext", "onPageSelect"]);
}

/// InputGroup — text input flanked by optional prefix/suffix addons.
/// No emits beyond the inner field's onChange/onCommit.
#[test]
fn input_group_interface_matches_spec() {
    let mil_src = read_source("InputGroup.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["prefix", "suffix", "value", "placeholder", "disabled"]
    );
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onChange", "onCommit"]);
}

/// Accordion — expand/collapse stack. Parallel headers + bodies
/// lists, host-owned open-index, onToggle(index) emit.
#[test]
fn accordion_interface_matches_spec() {
    let mil_src = read_source("Accordion.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["headers", "bodies", "open-index"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onToggle"]);
}

/// Tabs — horizontal tab bar + body panel. Host owns active-index
/// and supplies the matching active-body each tick.
#[test]
fn tabs_interface_matches_spec() {
    let mil_src = read_source("Tabs.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["headers", "active-body", "active-index"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// Tabs should render the active header through a distinct part,
/// not merely expose an unused active-index slot.
#[test]
fn tabs_active_header_part_compiles_and_is_styled() {
    let mil_src = read_source("Tabs.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src).unwrap();

    let mll_src = read_source("Tabs.mll");
    assert!(
        mll_src.contains("i == activeIndex"),
        "Tabs.mll should compare the loop index with activeIndex"
    );
    assert!(
        mll_src.contains("tabs-tab-active"),
        "Tabs.mll should route the active header through its own part"
    );

    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .expect("Tabs.mll should compile against Tabs.mil");
    assert!(
        mll_out.part_map_json.contains("tabs-tab-active"),
        "Tabs part map should include the active header part"
    );

    for theme in THEMES {
        let style_filename = format!("Tabs.{theme}.msl");
        let style_src = read_source(&style_filename);
        let style_out = mosstyle_compiler::compile(&style_src, Some(&mll_out.part_map_json))
            .unwrap_or_else(|e| panic!("{style_filename} failed to compile:\n{:#?}", e));
        let active = style_out
            .def
            .parts
            .iter()
            .find(|part| part.name == "tabs-tab-active")
            .unwrap_or_else(|| panic!("{style_filename} missing active tab part"));
        let background = active
            .base
            .iter()
            .find(|prop| prop.name == "background")
            .unwrap_or_else(|| panic!("{style_filename} active tab missing background"));
        let expected_background = if *theme == "light" {
            "#e7f1ff"
        } else {
            "#1d4ed8"
        };
        assert_eq!(
            background.value, expected_background,
            "{style_filename} active tab background mismatch"
        );
    }
}

/// DropdownMenu — toggle button + revealed item list. Two emits:
/// onToggle (no payload) and onSelect(index).
#[test]
fn dropdown_menu_interface_matches_spec() {
    let mil_src = read_source("DropdownMenu.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["label", "items", "open"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onToggle", "onSelect"]);
}

/// Navbar — brand on the left, HostLink row after.
#[test]
fn navbar_interface_matches_spec() {
    let mil_src = read_source("Navbar.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(slot_names, vec!["brand", "items", "active-index"]);
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelect"]);
}

/// Select — toggle button + revealed option list. onChange's payload
/// is the option text (not its index, unlike DropdownMenu).
#[test]
fn select_interface_matches_spec() {
    let mil_src = read_source("Select.mil");
    let out = mosmodel_compiler::compile(&mil_src).unwrap();
    let c = &out.component;
    let slot_names: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["value", "options", "placeholder", "open", "disabled"]
    );
    let emit_names: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onToggle", "onChange"]);
}
