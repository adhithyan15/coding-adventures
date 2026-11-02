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
    // 0.2.0: the focused field is DraftEditor's body (see the tests below).
    assert!(source.contains("body-value : slot: selected-field-value"));
    assert!(source.contains("onBodyChange : emit: onFieldValueChange"));
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

/// NoteEditor's layout with every comment line removed, so a test can ask
/// what the layout *does* without tripping over what its comments explain.
fn layout_code() -> String {
    read_source("NoteEditor.mll")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The package built for one backend, as the single emitted source file.
///
/// The per-file compiles above cannot see a composition: DraftEditor's layout
/// is inlined only by a package build, which is also where an unbound
/// required slot of it is refused (UI13 §1, E6 F1).
fn built_source(backend: Backend, file: &str) -> String {
    let out = tempfile::tempdir().expect("temp dir");
    build_package(&BuildOptions {
        package_root: package_root(),
        output_root: out.path().to_path_buf(),
        backend,
        emit_project: false,
        theme: Some("light".to_string()),
    })
    .unwrap_or_else(|e| panic!("{backend:?} package build failed: {e}"));
    let path = out.path().join(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn the_focused_field_is_a_body_only_draft_editor() {
    // 0.2.0 (E4 of mosaic-editor-convergence-audit-v1.md, probe B): the
    // focused field is DraftEditor, mounted with no title and no actions.
    // NoteEditor keeps its own Save note / Delete / Cancel row, so the
    // parts the mount replaced must not come back beside it.
    let code = layout_code();
    assert_eq!(
        code.matches("pkg::mosaic-pkg-draft-editor::DraftEditor")
            .count(),
        1
    );
    for retired in ["selected-field-input", "Text [ selected-field-label ]"] {
        assert!(
            !code.contains(retired),
            "NoteEditor.mll must not draw `{retired}` itself any more"
        );
    }
    // Every label DraftEditor gates a part on is "", so the title field and
    // DraftEditor's own three buttons are switched off.
    for off in [
        "title-label : \"\"",
        "save-label : \"\"",
        "delete-label : \"\"",
        "cancel-label : \"\"",
    ] {
        assert!(code.contains(off), "the mount must bind `{off}`");
    }
    // The field's name and hint come from NoteEditor's existing slots, so
    // the slot set (and Engram's call site) did not change.
    assert!(code.contains("body-label : slot: selected-field-label"));
    assert!(code.contains("body-placeholder : slot: selected-field-placeholder"));

    // Built, the composition draws exactly one text area and one one-line
    // field (tags), and no button of DraftEditor's: the only `saveNote`
    // dispatches are NoteEditor's own button and the body's chord.
    let tsx = built_source(Backend::React, "react/NoteEditor.tsx");
    assert_eq!(tsx.matches("<textarea").count(), 1, "{tsx}");
    assert_eq!(tsx.matches("<input").count(), 1, "only tags-input:\n{tsx}");
    assert_eq!(
        tsx.matches("dispatch({ type: \"saveNote\" })").count(),
        2,
        "the Save note button and the body's Ctrl/Cmd+Enter:\n{tsx}"
    );
    for absent in ["titleLabel", "dispatch({ type: \"save\" })"] {
        assert!(
            !tsx.contains(absent),
            "`{absent}` must not survive inlining"
        );
    }
}

#[test]
fn note_editor_declares_the_package_it_mounts() {
    // A `pkg::` reference to a package the manifest does not declare is
    // resolved only because the search path happens to contain it.
    let manifest: toml::Value = toml::from_str(
        &fs::read_to_string(package_root().join("mosaic-package.toml")).expect("manifest"),
    )
    .expect("manifest parses");
    assert_eq!(
        manifest["package"]["version"].as_str(),
        Some("0.2.0"),
        "the focused field's behaviour changed, so the version moved"
    );
    let dependency = manifest
        .get("dependencies")
        .and_then(|d| d.get("mosaic-pkg-draft-editor"))
        .and_then(|v| v.as_str());
    // 0.2.0 is the first DraftEditor with `body-placeholder`.
    assert_eq!(dependency, Some("0.2.0"));
}

#[test]
fn the_focused_field_is_multiline_and_saves_on_ctrl_or_cmd_enter() {
    // DraftEditor's body is the kernel text area: Enter is a line break, and
    // Ctrl+Enter (Cmd+Enter on macOS) raises DraftEditor's onSave, which
    // NoteEditor forwards to onSaveNote, as its Save note button does
    // (UI25 §4). Anki's Add dialog commits on the same chord.
    let code = layout_code();
    assert!(code.contains("onSave : emit: onSaveNote"));
    let tsx = built_source(Backend::React, "react/NoteEditor.tsx");
    let textarea = tsx
        .lines()
        .find(|line| line.contains("<textarea"))
        .unwrap_or_else(|| panic!("the focused field must lower to a <textarea>:\n{tsx}"));
    assert!(
        textarea.contains("value={selectedFieldValue}"),
        "{textarea}"
    );
    assert!(
        textarea.contains("placeholder={selectedFieldPlaceholder}"),
        "{textarea}"
    );
    assert!(
        textarea.contains("dispatch({ type: \"fieldValueChange\", value: e.target.value })"),
        "{textarea}"
    );
    assert!(textarea.contains("e.ctrlKey || e.metaKey"), "{textarea}");
    assert!(
        textarea.contains("dispatch({ type: \"saveNote\" })"),
        "{textarea}"
    );
}

#[test]
fn both_fields_are_named_by_their_visible_labels() {
    // DraftEditor names its body by `body-label`, which is the field's name
    // here. The tags field is NoteEditor's own; it used to have no name
    // beyond its placeholder and now takes its visible label as its name.
    let code = layout_code();
    assert!(code.contains("a11y-label : slot: tags-label"));
    assert!(code.contains("content : slot: tags-label"));
    let tsx = built_source(Backend::React, "react/NoteEditor.tsx");
    for name in ["selectedFieldLabel", "tagsLabel"] {
        assert!(
            tsx.contains(&format!("aria-label={{{name}}}")),
            "a field must be named by `{name}`:\n{tsx}"
        );
    }
}

#[test]
fn note_editor_draws_no_english_of_its_own() {
    // Every word on screen is a NoteEditor slot the host binds. The only
    // quoted strings in the layout are the "" that switch DraftEditor's
    // title and actions off.
    let code = layout_code();
    let literals: Vec<&str> = code.split('"').skip(1).step_by(2).collect();
    assert!(
        literals.iter().all(|literal| literal.is_empty()),
        "NoteEditor.mll must take every label as a slot, found {literals:?}"
    );
}

#[test]
fn stories_set_every_slot_and_cover_the_focused_field_states() {
    use coding_adventures_bounded_json::{parse, JsonValue};

    fn field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
        match value {
            JsonValue::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn text<'a>(value: Option<&'a JsonValue>, what: &str) -> &'a str {
        match value {
            Some(JsonValue::String(s)) => s,
            other => panic!("{what}: expected a string, found {other:?}"),
        }
    }

    let (component, _, _) = compiled_note_editor();
    let mut declared: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    declared.sort_unstable();

    let doc = parse(&read_source("NoteEditor.stories.json")).expect("stories file is JSON");
    let Some(JsonValue::Array(stories)) = field(&doc, "stories") else {
        panic!("a stories array");
    };
    let (mut empty_with_hint, mut multi_line) = (false, false);
    for story in stories {
        let name = text(field(story, "name"), "story name");
        let Some(fixtures @ JsonValue::Object(members)) = field(story, "fixtures") else {
            panic!("{name}: a fixtures object");
        };
        let mut keys: Vec<&str> = members.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(
            keys, declared,
            "{name}: fixtures must set exactly the declared slots"
        );
        // The field's label is also its accessible name, so no story may
        // show it empty, any more than the host may send it empty.
        assert!(
            !text(field(fixtures, "selected-field-label"), name).is_empty(),
            "{name}: the focused field must have a name"
        );
        let value = text(field(fixtures, "selected-field-value"), name);
        let hint = text(field(fixtures, "selected-field-placeholder"), name);
        empty_with_hint |= value.is_empty() && !hint.is_empty();
        multi_line |= value.contains('\n');
    }
    assert!(
        empty_with_hint,
        "a story shows the placeholder on an empty field"
    );
    assert!(multi_line, "a story shows a multi-line field value");
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
