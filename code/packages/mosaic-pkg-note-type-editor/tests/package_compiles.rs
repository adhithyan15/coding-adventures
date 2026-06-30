use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["NoteTypeEditor"];

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

fn compiled_note_type_editor() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("NoteTypeEditor.mil"))
        .expect("NoteTypeEditor.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("NoteTypeEditor.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("NoteTypeEditor.mll should compile against NoteTypeEditor.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("NoteTypeEditor.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("NoteTypeEditor.dark.msl should compile against NoteTypeEditor.mll parts");

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
    assert_eq!(name, "mosaic-pkg-note-type-editor");

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
fn note_type_editor_frontend_sources_compile() {
    let (component, layout, style) = compiled_note_type_editor();

    assert_eq!(component.component, "NoteTypeEditor");
    assert_eq!(layout.component_name, "NoteTypeEditor");
    assert_eq!(style.component_name, "NoteTypeEditor");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "editor-label",
            "note-types-label",
            "note-type-names",
            "selected-note-type-index",
            "note-type-id-label",
            "note-type-id-value",
            "name-label",
            "name-value",
            "name-placeholder",
            "fields-label",
            "field-labels",
            "selected-field-index",
            "field-name-label",
            "field-name-value",
            "field-name-placeholder",
            "field-required-label",
            "field-required-value",
            "templates-label",
            "template-labels",
            "stylesheet-label",
            "stylesheet-value",
            "stylesheet-placeholder",
            "new-label",
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
            "onSelectField",
            "onNameChange",
            "onFieldNameChange",
            "onFieldRequiredChange",
            "onStylesheetChange",
            "onNewNoteType",
            "onSaveNoteType",
            "onDeleteNoteType",
            "onCancel",
        ]
    );
}

#[test]
fn note_type_editor_layout_wires_model_editor() {
    let source = read_source("NoteTypeEditor.mll");

    assert!(source.contains("For ( each: slot: note-type-names"));
    assert!(source.contains("HostButton [ note-type-option ]"));
    assert!(source.contains("onClick : emit: onSelectNoteType"));
    assert!(source.contains("HostInput [ note-type-name-input ]"));
    assert!(source.contains("value : slot: name-value"));
    assert!(source.contains("onChange : emit: onNameChange"));
    assert!(source.contains("For ( each: slot: field-labels"));
    assert!(source.contains("HostButton [ note-type-field-option ]"));
    assert!(source.contains("onClick : emit: onSelectField"));
    assert!(source.contains("HostInput [ note-type-field-name-input ]"));
    assert!(source.contains("value : slot: field-name-value"));
    assert!(source.contains("onChange : emit: onFieldNameChange"));
    assert!(source.contains("HostCheckbox [ note-type-field-required-checkbox ]"));
    assert!(source.contains("checked : slot: field-required-value"));
    assert!(source.contains("onToggle : emit: onFieldRequiredChange"));
    assert!(source.contains("For ( each: slot: template-labels"));
    assert!(source.contains("HostInput [ note-type-stylesheet-input ]"));
    assert!(source.contains("value : slot: stylesheet-value"));
    assert!(source.contains("onChange : emit: onStylesheetChange"));

    for (label, emit) in [
        ("new-label", "onNewNoteType"),
        ("save-label", "onSaveNoteType"),
        ("delete-label", "onDeleteNoteType"),
        ("cancel-label", "onCancel"),
    ] {
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "NoteTypeEditor.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "NoteTypeEditor.mll must wire {emit}"
        );
    }

    let style = read_source("NoteTypeEditor.dark.msl");
    for part in [
        "note-type-editor",
        "note-type-list-column",
        "note-type-option",
        "note-type-detail-column",
        "note-type-name-input",
        "note-type-schema-summary",
        "note-type-field-list",
        "note-type-field-option",
        "note-type-field-name-input",
        "note-type-field-required-checkbox",
        "note-type-template-list",
        "note-type-stylesheet-input",
        "note-type-new-button",
        "note-type-save-button",
        "note-type-delete-button",
        "note-type-cancel-button",
    ] {
        assert!(
            style.contains(&format!("part {part}")),
            "NoteTypeEditor.dark.msl must style {part}"
        );
    }
}

#[test]
fn note_type_editor_package_emitters_accept_model_editor_surface() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/NoteTypeEditor.html"),
        (Backend::React, "react/NoteTypeEditor.tsx"),
        (Backend::SwiftUI, "swiftui/NoteTypeEditor.swift"),
        (Backend::Qt, "qt/NoteTypeEditor.qml"),
        (Backend::Xaml, "xaml/NoteTypeEditor.xaml"),
        (Backend::Flutter, "flutter/NoteTypeEditor.dart"),
        (Backend::Compose, "compose/NoteTypeEditor.kt"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build NoteTypeEditor: {e}"));

        assert_eq!(result.components_built, vec!["NoteTypeEditor"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("NoteTypeEditor.html"))
        .expect("NoteTypeEditor HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"NoteTypeEditor\""));
    assert!(html.contains("data-on-change=\"onNameChange\""));
    assert!(html.contains("data-on-change=\"onFieldNameChange\""));
    assert!(html.contains("data-on-toggle=\"onFieldRequiredChange\""));
    assert!(html.contains("data-on-change=\"onStylesheetChange\""));
    assert!(html.contains("data-on-click=\"onSaveNoteType\""));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "NoteTypeEditor.mil",
        "NoteTypeEditor.mll",
        "NoteTypeEditor.dark.msl",
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
