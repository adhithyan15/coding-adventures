use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["NoteEditor"];

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

fn compiled_note_editor() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("NoteEditor.mil"))
        .expect("NoteEditor.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("NoteEditor.mll"), Some(&mil.descriptor_json))
            .expect("NoteEditor.mll should compile against NoteEditor.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("NoteEditor.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("NoteEditor.dark.msl should compile against NoteEditor.mll parts");

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
    assert_eq!(name, "mosaic-pkg-note-editor");

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
fn note_editor_frontend_sources_compile() {
    let (component, layout, style) = compiled_note_editor();

    assert_eq!(component.component, "NoteEditor");
    assert_eq!(layout.component_name, "NoteEditor");
    assert_eq!(style.component_name, "NoteEditor");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "editor-label",
            "note-id-label",
            "note-id-value",
            "note-type-label",
            "note-type-value",
            "note-type-options-label",
            "note-type-names",
            "selected-note-type-index",
            "deck-label",
            "deck-value",
            "deck-options-label",
            "deck-names",
            "selected-deck-index",
            "fields-label",
            "field-labels",
            "selected-field-index",
            "selected-field-label",
            "selected-field-value",
            "selected-field-placeholder",
            "tags-label",
            "tags-value",
            "tags-placeholder",
            "save-label",
            "delete-label",
            "cancel-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onSelectNoteType",
            "onSelectDeck",
            "onSelectField",
            "onFieldValueChange",
            "onTagsChange",
            "onSaveNote",
            "onDeleteNote",
            "onCancel",
        ]
    );
}

#[test]
fn note_editor_layout_wires_focused_field_editor() {
    let source = read_source("NoteEditor.mll");

    assert!(source.contains("For ( each: slot: note-type-names"));
    assert!(source.contains("HostButton [ note-editor-note-type-option ]"));
    assert!(source.contains("onClick : emit: onSelectNoteType"));
    assert!(source.contains("For ( each: slot: deck-names"));
    assert!(source.contains("HostButton [ note-editor-deck-option ]"));
    assert!(source.contains("onClick : emit: onSelectDeck"));
    assert!(source.contains("For ( each: slot: field-labels"));
    assert!(source.contains("HostButton [ note-field-option ]"));
    assert!(source.contains("onClick : emit: onSelectField"));
    assert!(source.contains("HostInput [ selected-field-input ]"));
    assert!(source.contains("value : slot: selected-field-value"));
    assert!(source.contains("onChange : emit: onFieldValueChange"));
    assert!(source.contains("HostInput [ tags-input ]"));
    assert!(source.contains("value : slot: tags-value"));
    assert!(source.contains("onChange : emit: onTagsChange"));

    for (label, emit) in [
        ("save-label", "onSaveNote"),
        ("delete-label", "onDeleteNote"),
        ("cancel-label", "onCancel"),
    ] {
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "NoteEditor.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "NoteEditor.mll must wire {emit}"
        );
    }

    let style = read_source("NoteEditor.dark.msl");
    for part in [
        "note-editor",
        "note-metadata",
        "note-choice-row",
        "note-editor-note-type-list-column",
        "note-editor-note-type-list",
        "note-editor-note-type-option",
        "note-editor-deck-list-column",
        "note-editor-deck-list",
        "note-editor-deck-option",
        "field-list-column",
        "note-field-list",
        "note-field-option",
        "focused-field-column",
        "selected-field-input",
        "tags-input",
        "save-button",
        "delete-button",
        "cancel-button",
    ] {
        assert!(
            style.contains(&format!("part {part}")),
            "NoteEditor.dark.msl must style {part}"
        );
    }
}

#[test]
fn note_editor_package_emitters_accept_focused_editor_surface() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/NoteEditor.html"),
        (Backend::React, "react/NoteEditor.tsx"),
        (Backend::SwiftUI, "swiftui/NoteEditor.swift"),
        (Backend::Qt, "qt/NoteEditor.qml"),
        (Backend::Xaml, "xaml/NoteEditor.xaml"),
        (Backend::Flutter, "flutter/NoteEditor.dart"),
        (Backend::Compose, "compose/NoteEditor.kt"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build NoteEditor: {e}"));

        assert_eq!(result.components_built, vec!["NoteEditor"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("NoteEditor.html"))
        .expect("NoteEditor HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"NoteEditor\""));
    assert!(html.contains("data-on-change=\"onFieldValueChange\""));
    assert!(html.contains("data-on-click=\"onSaveNote\""));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "NoteEditor.mil",
        "NoteEditor.mll",
        "NoteEditor.dark.msl",
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
