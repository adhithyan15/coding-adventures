use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["CollectionActions"];

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

fn compiled_collection_actions() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("CollectionActions.mil"))
        .expect("CollectionActions.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("CollectionActions.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("CollectionActions.mll should compile against CollectionActions.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("CollectionActions.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("CollectionActions.dark.msl should compile against CollectionActions.mll parts");

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
    assert_eq!(name, "mosaic-pkg-collection-actions");

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
fn collection_actions_frontend_sources_compile() {
    let (component, layout, style) = compiled_collection_actions();

    assert_eq!(component.component, "CollectionActions");
    assert_eq!(layout.component_name, "CollectionActions");
    assert_eq!(style.component_name, "CollectionActions");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "collection-label",
            "note-count-label",
            "note-count-value",
            "note-type-count-label",
            "note-type-count-value",
            "media-count-label",
            "media-count-value",
            "referenced-media-label",
            "referenced-media-value",
            "missing-media-label",
            "missing-media-value",
            "missing-media-filenames",
            "unused-media-label",
            "unused-media-value",
            "unused-media-asset-ids",
            "prune-unused-media-label",
            "import-label",
            "export-label",
            "add-note-label",
            "add-note-type-label",
            "delete-note-label",
            "delete-note-type-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onImportAnki",
            "onExportAnki",
            "onPruneUnusedMedia",
            "onAddNote",
            "onAddNoteType",
            "onDeleteNote",
            "onDeleteNoteType",
        ]
    );
}

#[test]
fn collection_actions_layout_wires_counts_and_actions() {
    let source = read_source("CollectionActions.mll");

    for slot in [
        "note-count-value",
        "note-type-count-value",
        "media-count-value",
        "referenced-media-value",
        "missing-media-value",
        "unused-media-value",
        "note-count-label",
        "note-type-count-label",
        "media-count-label",
        "referenced-media-label",
        "missing-media-label",
        "unused-media-label",
    ] {
        assert!(
            source.contains(&format!("content : slot: {slot}")),
            "CollectionActions.mll must bind {slot}"
        );
    }

    for (label, emit) in [
        ("import-label", "onImportAnki"),
        ("export-label", "onExportAnki"),
        ("prune-unused-media-label", "onPruneUnusedMedia"),
        ("add-note-label", "onAddNote"),
        ("add-note-type-label", "onAddNoteType"),
        ("delete-note-label", "onDeleteNote"),
        ("delete-note-type-label", "onDeleteNoteType"),
    ] {
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "CollectionActions.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "CollectionActions.mll must wire {emit}"
        );
    }
}

#[test]
fn collection_actions_package_emitters_accept_workflow_surface() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (
            Backend::React,
            "react/CollectionActions.tsx",
            "background: \"#2563eb\"",
            "background: \"#9f1239\"",
        ),
        (
            Backend::Electron,
            "electron/CollectionActions.tsx",
            "background: \"#2563eb\"",
            "background: \"#9f1239\"",
        ),
        (
            Backend::SwiftUI,
            "swiftui/CollectionActions.swift",
            "Color(red: 0.145, green: 0.388, blue: 0.922)",
            "Color(red: 0.624, green: 0.071, blue: 0.224)",
        ),
        (
            Backend::Qt,
            "qt/CollectionActions.qml",
            "color: \"#2563eb\"",
            "color: \"#9f1239\"",
        ),
        (
            Backend::WebComponent,
            "webcomponent/CollectionActions.js",
            "background: #2563eb",
            "background: #9f1239",
        ),
        (
            Backend::Html,
            "html/CollectionActions.html",
            "background: #2563eb",
            "background: #9f1239",
        ),
        (
            Backend::Xaml,
            "xaml/CollectionActions.xaml",
            "Background=\"#2563eb\"",
            "Background=\"#9f1239\"",
        ),
        (
            Backend::Flutter,
            "flutter/CollectionActions.dart",
            "Color(0xFF2563EB)",
            "Color(0xFF9F1239)",
        ),
        (
            Backend::Compose,
            "compose/CollectionActions.kt",
            "Color(0xFF2563EB)",
            "Color(0xFF9F1239)",
        ),
    ];

    for (backend, expected_artifact, primary_button_style, destructive_button_style) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build CollectionActions: {e}"));

        assert_eq!(result.components_built, vec!["CollectionActions"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );

        let artifact = fs::read_to_string(tmp.path().join(expected_artifact))
            .unwrap_or_else(|e| panic!("failed to read {expected_artifact}: {e}"));
        assert!(
            artifact.contains(primary_button_style),
            "{backend:?} did not lower the import-button style into {expected_artifact}"
        );
        assert!(
            artifact.contains(destructive_button_style),
            "{backend:?} did not lower the delete-button style into {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("CollectionActions.html"))
        .expect("CollectionActions HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"CollectionActions\""));
    assert!(
        html.contains("#0891b2"),
        "collection action style should reach HTML artifact"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "CollectionActions.mil",
        "CollectionActions.mll",
        "CollectionActions.dark.msl",
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
