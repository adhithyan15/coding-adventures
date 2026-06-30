use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["CardBrowser"];

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

fn compiled_card_browser() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("CardBrowser.mil"))
        .expect("CardBrowser.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("CardBrowser.mll"), Some(&mil.descriptor_json))
            .expect("CardBrowser.mll should compile against CardBrowser.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("CardBrowser.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("CardBrowser.dark.msl should compile against CardBrowser.mll parts");

    (mil.component, mll.def, msl.def)
}

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("mosaic-package.toml must exist");
    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-card-browser");

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(export_names, COMPONENTS);

    let toolkit_version = value
        .get("dependencies")
        .and_then(|d| d.get("mosaic-pkg-toolkit"))
        .and_then(|v| v.as_str())
        .expect("[dependencies].mosaic-pkg-toolkit must be set");
    assert_eq!(toolkit_version, "0.11.0");

    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1");
}

#[test]
fn card_browser_frontend_sources_compile() {
    let (component, layout, style) = compiled_card_browser();

    assert_eq!(component.component, "CardBrowser");
    assert_eq!(layout.component_name, "CardBrowser");
    assert_eq!(style.component_name, "CardBrowser");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "browser-label",
            "query-label",
            "query",
            "query-placeholder",
            "filter-label",
            "filter-value",
            "filter-options",
            "filter-placeholder",
            "filter-open",
            "search-label",
            "results-label",
            "results-summary",
            "results",
            "result-card-ids",
            "result-note-ids",
            "result-template-ids",
            "result-states",
            "result-flags",
            "selected-index",
            "selected-card-id",
            "selected-note-id",
            "selected-template-id",
            "selected-state",
            "selected-flag",
            "open-label",
            "edit-label",
            "suspend-label",
            "mark-label",
            "flag-label",
            "flag-value",
            "flag-options",
            "flag-placeholder",
            "flag-open",
            "tag-edit-label",
            "tag-edit",
            "tag-edit-placeholder",
            "add-tag-label",
            "remove-tag-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onQueryChange",
            "onToggleFilter",
            "onSetFilter",
            "onSearch",
            "onSelectResult",
            "onOpenSelected",
            "onEditSelected",
            "onToggleSuspendSelected",
            "onToggleMarkSelected",
            "onToggleFlagPicker",
            "onSetFlagSelected",
            "onTagEditChange",
            "onAddTagSelected",
            "onRemoveTagSelected",
        ]
    );
}

#[test]
fn card_browser_layout_wires_search_results_and_actions() {
    let source = read_source("CardBrowser.mll");

    assert!(source.contains("pkg::mosaic-pkg-toolkit::Input"));
    assert!(source.contains("value : slot: query"));
    assert!(source.contains("onChange : emit: onQueryChange"));
    assert!(source.contains("onCommit : emit: onSearch"));
    assert!(source.contains("Column [ filter-column ]"));
    assert!(source.contains("label : slot: filter-value"));
    assert!(source.contains("onClick : emit: onToggleFilter"));
    assert!(source.contains("For ( each: slot: filter-options"));
    assert!(source.contains("label : filter-option"));
    assert!(source.contains("onClick : emit: onSetFilter"));
    assert!(source.contains("pkg::mosaic-pkg-toolkit::ListGroup"));
    assert!(source.contains("items : slot: results"));
    assert!(source.contains("selected-index : slot: selected-index"));
    assert!(source.contains("onSelect : emit: onSelectResult"));
    assert!(source.contains("Row [ browser-flag-row ]"));
    assert!(source.contains("pkg::mosaic-pkg-toolkit::Select"));
    assert!(source.contains("value : slot: flag-value"));
    assert!(source.contains("options : slot: flag-options"));
    assert!(source.contains("placeholder : slot: flag-placeholder"));
    assert!(source.contains("open : slot: flag-open"));
    assert!(source.contains("onToggle : emit: onToggleFlagPicker"));
    assert!(source.contains("onChange : emit: onSetFlagSelected"));
    assert!(source.contains("Row [ browser-tag-row ]"));
    assert!(source.contains("HostInput [ tag-edit-input ]"));
    assert!(source.contains("value : slot: tag-edit"));
    assert!(source.contains("placeholder : slot: tag-edit-placeholder"));
    assert!(source.contains("onChange : emit: onTagEditChange"));

    for (label, emit) in [
        ("open-label", "onOpenSelected"),
        ("edit-label", "onEditSelected"),
        ("suspend-label", "onToggleSuspendSelected"),
        ("mark-label", "onToggleMarkSelected"),
        ("add-tag-label", "onAddTagSelected"),
        ("remove-tag-label", "onRemoveTagSelected"),
    ] {
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "CardBrowser.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "CardBrowser.mll must wire {emit}"
        );
    }

    let style = read_source("CardBrowser.dark.msl");
    for part in [
        "browser-tag-row",
        "filter-column",
        "filter-label",
        "filter-toggle-button",
        "filter-options-list",
        "filter-option-button",
        "browser-flag-row",
        "flag-picker-column",
        "flag-label",
        "selected-flag-label",
        "tag-edit-column",
        "tag-edit-label",
        "tag-edit-input",
        "add-tag-button",
        "remove-tag-button",
    ] {
        assert!(
            style.contains(&format!("part {part}")),
            "CardBrowser.dark.msl must style {part}"
        );
    }
}

#[test]
fn card_browser_package_emitters_all_accept_nested_toolkit_controls() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/CardBrowser.html"),
        (Backend::React, "react/CardBrowser.tsx"),
        (Backend::SwiftUI, "swiftui/CardBrowser.swift"),
        (Backend::Qt, "qt/CardBrowser.qml"),
        (Backend::Xaml, "xaml/CardBrowser.xaml"),
        (Backend::Flutter, "flutter/CardBrowser.dart"),
        (Backend::Compose, "compose/CardBrowser.kt"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build CardBrowser: {e}"));

        assert_eq!(result.components_built, vec!["CardBrowser"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("CardBrowser.html"))
        .expect("CardBrowser HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"CardBrowser\""));
    assert!(
        html.contains("#1e293b"),
        "nested toolkit Input/ListGroup styles should reach CardBrowser HTML"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = ["CardBrowser.mil", "CardBrowser.mll", "CardBrowser.dark.msl"];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }
}
