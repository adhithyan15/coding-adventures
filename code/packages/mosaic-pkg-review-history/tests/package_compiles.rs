use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["ReviewHistoryPanel"];

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

fn compiled_review_history_panel() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("ReviewHistoryPanel.mil"))
        .expect("ReviewHistoryPanel.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("ReviewHistoryPanel.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("ReviewHistoryPanel.mll should compile against ReviewHistoryPanel.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("ReviewHistoryPanel.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("ReviewHistoryPanel.dark.msl should compile against ReviewHistoryPanel.mll parts");

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
    assert_eq!(name, "mosaic-pkg-review-history");

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
fn review_history_frontend_sources_compile() {
    let (component, layout, style) = compiled_review_history_panel();

    assert_eq!(component.component, "ReviewHistoryPanel");
    assert_eq!(layout.component_name, "ReviewHistoryPanel");
    assert_eq!(style.component_name, "ReviewHistoryPanel");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "history-label",
            "window-label",
            "total-label",
            "total-value",
            "correct-label",
            "correct-value",
            "unique-label",
            "unique-value",
            "accuracy-label",
            "accuracy-value",
            "again-label",
            "again-value",
            "hard-label",
            "hard-value",
            "good-label",
            "good-value",
            "easy-label",
            "easy-value",
            "first-label",
            "first-value",
            "last-label",
            "last-value",
        ]
    );

    assert!(component.emits.is_empty());
}

#[test]
fn review_history_layout_binds_summary_slots() {
    let source = read_source("ReviewHistoryPanel.mll");

    for (part, label, value) in [
        ("history-total", "total-label", "total-value"),
        ("history-correct", "correct-label", "correct-value"),
        ("history-unique", "unique-label", "unique-value"),
        ("history-accuracy", "accuracy-label", "accuracy-value"),
        ("history-rating-again", "again-label", "again-value"),
        ("history-rating-hard", "hard-label", "hard-value"),
        ("history-rating-good", "good-label", "good-value"),
        ("history-rating-easy", "easy-label", "easy-value"),
        ("history-first", "first-label", "first-value"),
        ("history-last", "last-label", "last-value"),
    ] {
        assert!(
            source.contains(&format!("Box [ {part} ]")),
            "ReviewHistoryPanel.mll must expose {part}"
        );
        assert!(
            source.contains(&format!("content : slot: {label}")),
            "ReviewHistoryPanel.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("content : slot: {value}")),
            "ReviewHistoryPanel.mll must bind {value}"
        );
    }

    assert!(source.contains("content : slot: history-label"));
    assert!(source.contains("content : slot: window-label"));
}

#[test]
fn review_history_package_emitters_accept_summary_surface() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/ReviewHistoryPanel.html"),
        (Backend::React, "react/ReviewHistoryPanel.tsx"),
        (Backend::SwiftUI, "swiftui/ReviewHistoryPanel.swift"),
        (Backend::Qt, "qt/ReviewHistoryPanel.qml"),
        (Backend::Xaml, "xaml/ReviewHistoryPanel.xaml"),
        (Backend::Flutter, "flutter/ReviewHistoryPanel.dart"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build ReviewHistoryPanel: {e}"));

        assert_eq!(result.components_built, vec!["ReviewHistoryPanel"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("ReviewHistoryPanel.html"))
        .expect("ReviewHistoryPanel HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"ReviewHistoryPanel\""));
    assert!(
        html.contains("#ca8a04"),
        "review history style should reach HTML artifact"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "ReviewHistoryPanel.mil",
        "ReviewHistoryPanel.mll",
        "ReviewHistoryPanel.dark.msl",
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
