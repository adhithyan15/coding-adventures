//! package_compiles — smoke test for the mosaic-pkg-notes package.
//!
//! Mirrors mosaic-pkg-calendar's own `package_compiles.rs`: manifest parses
//! and declares the expected export; Notes.mil compiles via mosmodel;
//! Notes.mll compiles against that interface via moslayout; Notes.dark.msl /
//! Notes.light.msl each compile against the resulting part map via mosstyle.
//! Since 0.3.0 Notes mounts the toolkit's RecordList and DraftEditor, so a
//! few tests also build the package, which is where those mounts are
//! inlined and their required slots checked.

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
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as valid TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-notes", "[package].name mismatch");

    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Notes"],
        "[components].exports must list exactly Notes"
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
fn notes_mil_compiles() {
    let src = read_source("Notes.mil");
    let out = mosmodel_compiler::compile(&src)
        .unwrap_or_else(|e| panic!("Notes.mil failed to compile via mosmodel_compiler:\n{:#?}", e));
    assert_eq!(out.component.component, "Notes");

    let slot_names: Vec<&str> = out.component.slots.iter().map(|s| s.name.as_str()).collect();
    for expected in [
        "notes-title",
        "note-rows",
        "selected-note-id",
        "title-value",
        "body-value",
        "task-name-value",
        // 0.3.0: every label is a slot (no English in the package).
        "new-note-label",
        "empty-hint",
        "title-label",
        "title-placeholder",
        "body-label",
        "body-placeholder",
        "task-label",
        "task-placeholder",
        "save-label",
        "delete-label",
        "cancel-label",
    ] {
        assert!(
            slot_names.contains(&expected),
            "Notes.mil must declare slot `{}`, got: {:?}",
            expected,
            slot_names
        );
    }

    let emit_names: Vec<&str> = out.component.emits.iter().map(|e| e.name.as_str()).collect();
    for expected in [
        "onSelectNote",
        "onNewNote",
        "onTitleChange",
        "onBodyChange",
        "onSave",
        "onDelete",
        "onCancel",
    ] {
        assert!(
            emit_names.contains(&expected),
            "Notes.mil must declare emit `{}`, got: {:?}",
            expected,
            emit_names
        );
    }
}

// ---------------------------------------------------------------------------
// 3. moslayout — .mll compilation
// ---------------------------------------------------------------------------

#[test]
fn notes_mll_compiles_against_its_interface() {
    let mil_src = read_source("Notes.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|e| panic!("Notes.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Notes.mll");
    moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Notes.mll failed to compile via moslayout_compiler:\n{:#?}", e));
}

/// Notes' layout with every comment line removed, so a test can ask what the
/// layout *does* without tripping over what its comments explain.
fn layout_code() -> String {
    read_source("Notes.mll")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The package built for one backend, as the single emitted source file.
///
/// The per-file compiles above cannot see a composition: RecordList's and
/// DraftEditor's layouts are inlined only by a package build, which is also
/// where an unbound required slot of either is refused (UI13 §1).
fn built_source(backend: mosaic_package_artifact_builder::Backend, file: &str) -> String {
    use mosaic_package_artifact_builder::{build_package, BuildOptions};
    let out = tempfile::TempDir::new().expect("temp dir");
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
fn notes_mounts_record_list_and_draft_editor() {
    // 0.3.0 (task-app-notes-ui-v1.md addendum v1.3): the list is the
    // toolkit's RecordList and the title, body and actions are DraftEditor.
    // The parts they replaced must not come back beside them.
    let code = layout_code();
    assert_eq!(code.matches("pkg::mosaic-pkg-toolkit::RecordList").count(), 1);
    assert_eq!(code.matches("pkg::mosaic-pkg-draft-editor::DraftEditor").count(), 1);
    for retired in [
        "notes-row-on",
        "notes-row-off",
        "notes-title-input",
        "notes-body-input",
        "notes-save",
        "notes-delete",
        "notes-cancel",
    ] {
        assert!(!code.contains(retired), "Notes.mll must not draw `{retired}` itself any more");
    }
}

#[test]
fn notes_declares_every_package_it_mounts() {
    // A `pkg::` reference to a package the manifest does not declare is
    // resolved today only because the search path happens to contain it.
    let manifest: toml::Value = toml::from_str(
        &fs::read_to_string(package_root().join("mosaic-package.toml")).expect("manifest"),
    )
    .expect("manifest parses");
    let dependencies = manifest
        .get("dependencies")
        .and_then(|d| d.as_table())
        .expect("[dependencies] must be a table");
    let code = layout_code();
    for package in ["mosaic-pkg-toolkit", "mosaic-pkg-draft-editor"] {
        assert!(code.contains(&format!("pkg::{package}::")), "Notes mounts {package}");
        assert!(
            dependencies.contains_key(package),
            "mosaic-package.toml must declare `{package}`, which Notes.mll mounts"
        );
    }
}

#[test]
fn notes_draws_no_english_of_its_own() {
    // Every label is a slot the host binds (0.3.0), as DraftEditor's are, so
    // a host can localise Notes. A quoted string in the layout is a literal
    // label.
    let code = layout_code();
    assert!(
        !code.contains('"'),
        "Notes.mll must take every label as a slot, found a literal in:\n{code}"
    );
}

#[test]
fn the_selection_compares_row_keys_with_the_selected_note_id() {
    // Regression guard, found live-testing this package in 0.1.0: a slot
    // named inside an expression must be its camelCase identifier. Written
    // kebab-case it compiles everywhere and is silently wrong at runtime:
    // `selected-note-id` is subtraction of three undefined identifiers.
    // Since 0.3.0 the comparison is RecordList's (`row[0] == selectedKey`),
    // and Notes binds `selected-key : slot: selected-note-id`. Substitution
    // must turn that into the camelCase identifier, which only the built
    // output shows.
    let code = layout_code();
    assert!(code.contains("selected-key : slot: selected-note-id"));
    let tsx = built_source(mosaic_package_artifact_builder::Backend::React, "react/Notes.tsx");
    assert!(
        tsx.contains("row [ 0 ] == selectedNoteId"),
        "RecordList's selection must compare against Notes' selectedNoteId:\n{tsx}"
    );
    assert!(!tsx.contains("selected - note - id") && !tsx.contains("selected-note-id ="));
}

#[test]
fn the_body_is_multiline_and_saves_on_ctrl_or_cmd_enter() {
    // The body is DraftEditor's text area, and its Ctrl/Cmd+Enter chord
    // raises DraftEditor's onSave, which Notes forwards to its own onSave,
    // exactly as the Save button does (UI25 §4).
    let code = layout_code();
    assert!(code.contains("onSave : emit: onSave"), "Notes must forward DraftEditor's onSave");
    let tsx = built_source(mosaic_package_artifact_builder::Backend::React, "react/Notes.tsx");
    let textarea = tsx
        .lines()
        .find(|line| line.contains("<textarea"))
        .unwrap_or_else(|| panic!("the body must lower to a <textarea>:\n{tsx}"));
    assert!(textarea.contains("value={bodyValue}"), "{textarea}");
    assert!(textarea.contains("e.ctrlKey || e.metaKey"), "{textarea}");
    assert!(textarea.contains("dispatch({ type: \"save\" })"), "{textarea}");
}

#[test]
fn every_field_has_an_accessible_name_from_its_visible_label() {
    // DraftEditor names its title and body by their label slots. The
    // attach-to-task field is Notes' own, and used to have no name at all
    // beyond its placeholder; it now takes its visible label as its name.
    let code = layout_code();
    assert!(code.contains("a11y-label : slot: task-label"));
    assert!(code.contains("content : slot: task-label"));
    let tsx = built_source(mosaic_package_artifact_builder::Backend::React, "react/Notes.tsx");
    for name in ["taskLabel", "titleLabel", "bodyLabel"] {
        assert!(
            tsx.contains(&format!("aria-label={{{name}}}")),
            "a field must be named by `{name}`:\n{tsx}"
        );
    }
}

#[test]
fn delete_is_drawn_only_when_the_host_labels_it() {
    // A host passes `delete-label: ""` for a new note that was never saved,
    // and DraftEditor then draws no Delete button.
    let tsx = built_source(mosaic_package_artifact_builder::Backend::React, "react/Notes.tsx");
    assert!(tsx.contains("{deleteLabel && ("), "Delete must be gated on its label:\n{tsx}");
}

// ---------------------------------------------------------------------------
// 4. mosstyle — .msl compilation
// ---------------------------------------------------------------------------

#[test]
fn each_msl_theme_compiles_against_the_part_map() {
    let mll_src = read_source("Notes.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Notes.mll precompile failed: {:#?}", e));

    for theme in ["Notes.dark.msl", "Notes.light.msl"] {
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
    for name in ["Notes.mil", "Notes.mll", "Notes.dark.msl", "Notes.light.msl"] {
        let path = src_path(name);
        assert!(path.exists(), "expected package source file missing: {}", path.display());
    }
}
