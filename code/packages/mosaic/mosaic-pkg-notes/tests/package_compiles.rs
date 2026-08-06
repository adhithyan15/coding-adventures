//! package_compiles — smoke test for the mosaic-pkg-notes package.
//!
//! Mirrors mosaic-pkg-calendar's own `package_compiles.rs`: manifest parses
//! and declares the expected export; Notes.mil compiles via mosmodel;
//! Notes.mll compiles against that interface via moslayout; Notes.dark.msl /
//! Notes.light.msl each compile against the resulting part map via mosstyle.
//! No cross-package dependency to pin (built entirely from kernel
//! primitives, like mosaic-pkg-grid and mosaic-pkg-calendar).

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

#[test]
fn notes_mll_references_selected_note_id_in_camel_case() {
    // Regression guard: found live-testing this package. A slot referenced
    // bare inside a parenthesized expression must use its camelCase
    // identifier (`selectedNoteId`) — the kebab-case `slot:` spelling
    // (`selected-note-id`) compiles at every static layer (mosmodel/
    // moslayout/mosstyle all accept it) but is silently wrong at runtime:
    // the emitted JS parses `selected-note-id` as subtraction of three
    // undefined identifiers, not one reference to the slot.
    let src = read_source("Notes.mll");
    assert!(
        src.contains("selectedNoteId"),
        "Notes.mll must reference the selected-note-id slot as `selectedNoteId` inside expressions"
    );
    assert!(
        !src.contains("== selected-note-id"),
        "Notes.mll must not reference the slot by its kebab-case name inside an expression"
    );
}

#[test]
fn notes_mll_uses_a_multiline_body_field() {
    // Pins the deliberate legacy-`Input` use for the body's textarea — a
    // regression guard against silently regressing to a single-line
    // HostInput, which would be a real UX loss for a note body. See
    // task-app-notes-ui-v1.md for why HostInput alone isn't enough.
    let src = read_source("Notes.mll");
    assert!(src.contains("multiline : true"), "Notes.mll must use a multiline body field");
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
