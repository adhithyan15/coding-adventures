use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["DeckStatsPanel"];

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

fn compiled_deck_stats_panel() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("DeckStatsPanel.mil"))
        .expect("DeckStatsPanel.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("DeckStatsPanel.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("DeckStatsPanel.mll should compile against DeckStatsPanel.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("DeckStatsPanel.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("DeckStatsPanel.dark.msl should compile against DeckStatsPanel.mll parts");

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
    assert_eq!(name, "mosaic-pkg-deck-stats");

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(export_names, COMPONENTS);

    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1");
}

#[test]
fn deck_stats_frontend_sources_compile() {
    let (component, layout, style) = compiled_deck_stats_panel();

    assert_eq!(component.component, "DeckStatsPanel");
    assert_eq!(layout.component_name, "DeckStatsPanel");
    assert_eq!(style.component_name, "DeckStatsPanel");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "deck-label",
            "deck-name",
            "deck-list-label",
            "deck-names",
            "total-label",
            "total-value",
            "new-label",
            "new-value",
            "due-label",
            "due-value",
            "learning-label",
            "learning-value",
            "hidden-label",
            "hidden-value",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onSelectDeck"]);
}

#[test]
fn deck_stats_layout_binds_all_stat_slots() {
    let source = read_source("DeckStatsPanel.mll");

    assert!(source.contains("content : slot: deck-list-label"));
    assert!(source.contains("Row [ deck-list-row ]"));
    assert!(source.contains("For ( each: slot: deck-names"));
    assert!(source.contains("HostButton [ deck-option-button ]"));
    assert!(source.contains("label : deck-option"));
    assert!(source.contains("onClick : emit: onSelectDeck"));

    for (part, label, value) in [
        ("deck-stat-total", "total-label", "total-value"),
        ("deck-stat-new", "new-label", "new-value"),
        ("deck-stat-due", "due-label", "due-value"),
        ("deck-stat-learning", "learning-label", "learning-value"),
        ("deck-stat-hidden", "hidden-label", "hidden-value"),
    ] {
        assert!(
            source.contains(&format!("Box [ {part} ]")),
            "DeckStatsPanel.mll must expose {part}"
        );
        assert!(
            source.contains(&format!("content : slot: {label}")),
            "DeckStatsPanel.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("content : slot: {value}")),
            "DeckStatsPanel.mll must bind {value}"
        );
    }
}

#[test]
fn deck_stats_package_emitters_all_accept_deck_selector_controls() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/DeckStatsPanel.html"),
        (Backend::React, "react/DeckStatsPanel.tsx"),
        (Backend::SwiftUI, "swiftui/DeckStatsPanel.swift"),
        (Backend::Qt, "qt/DeckStatsPanel.qml"),
        (Backend::Xaml, "xaml/DeckStatsPanel.xaml"),
        (Backend::Flutter, "flutter/DeckStatsPanel.dart"),
        (Backend::Compose, "compose/DeckStatsPanel.kt"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build DeckStatsPanel: {e}"));

        assert_eq!(result.components_built, vec!["DeckStatsPanel"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("DeckStatsPanel.html"))
        .expect("DeckStatsPanel HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"DeckStatsPanel\""));
    assert!(html.contains("#2563eb"));
    assert!(html.contains("#0f766e"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "DeckStatsPanel.mil",
        "DeckStatsPanel.mll",
        "DeckStatsPanel.dark.msl",
    ];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }
}
