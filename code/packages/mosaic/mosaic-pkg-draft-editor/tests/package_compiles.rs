//! package_compiles — DraftEditor round-trips through the three IR compilers,
//! and its interface and layout hold the properties the spec promises.

use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["DraftEditor"];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_source(name: &str) -> String {
    let path = package_root().join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn compiled() -> (mosmodel_compiler::MosmodelComponent, moslayout_compiler::LayoutDef) {
    let mil = mosmodel_compiler::compile(&read_source("DraftEditor.mil"))
        .expect("DraftEditor.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("DraftEditor.mll"), Some(&mil.descriptor_json))
            .expect("DraftEditor.mll should compile against DraftEditor.mil");
    for theme in ["light", "dark"] {
        mosstyle_compiler::compile(
            &read_source(&format!("DraftEditor.{theme}.msl")),
            Some(&mll.part_map_json),
        )
        .unwrap_or_else(|e| panic!("DraftEditor.{theme}.msl should compile: {e:?}"));
    }
    (mil.component, mll.def)
}

fn prop(node: &moslayout_compiler::LayoutNode, name: &str) -> Option<String> {
    node.props
        .iter()
        .find(|p| p.name == name)
        .map(|p| format!("{:?}", p.value))
}

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("mosaic-package.toml must exist");
    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as TOML");
    let get = |section: &str, key: &str| value.get(section).and_then(|s| s.get(key)).cloned();
    assert_eq!(
        get("package", "name").and_then(|v| v.as_str().map(str::to_string)).as_deref(),
        Some("mosaic-pkg-draft-editor")
    );
    let exports: Vec<String> = get("components", "exports")
        .and_then(|v| v.as_array().cloned())
        .expect("[components].exports")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert_eq!(exports, COMPONENTS);
    assert_eq!(
        get("kernel", "version").and_then(|v| v.as_str().map(str::to_string)).as_deref(),
        Some("1")
    );
    // A package with dependencies could have its bare `Input` rewritten to a
    // dependency's component of the same name — the toolkit trap the spec
    // describes. Keep this package dependency-free.
    assert!(value.get("dependencies").is_none(), "DraftEditor depends on nothing");
}

/// The interface is the contract with Journal, Trestle and Engram.
#[test]
fn interface_matches_spec() {
    let (component, _) = compiled();
    let slots: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slots,
        [
            "title-label",
            "title-value",
            "title-placeholder",
            "body-label",
            "body-value",
            "save-label",
            "delete-label",
            "cancel-label",
        ]
    );
    let emits: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emits, ["onTitleChange", "onBodyChange", "onSave", "onDelete", "onCancel"]);
}

/// The layout properties the spec promises: the body is the multi-line legacy
/// `Input` named by its visible label; the title field and every action exist
/// only when labelled (so nothing unnamed can be drawn).
#[test]
fn layout_names_every_field_and_gates_every_optional_part() {
    let (_, layout) = compiled();
    let root = &layout.root;
    assert_eq!(root.tag, "Column");
    assert_eq!(root.children.len(), 4, "title gate, body label, body, actions");

    // Title: gated on its label, which is also its accessible name.
    let title_gate = &root.children[0];
    assert_eq!(title_gate.tag, "If");
    assert_eq!(prop(title_gate, "when").as_deref(), Some("SlotRef(\"title-label\")"));
    let title_input = &title_gate.children[0].children[1];
    assert_eq!(title_input.tag, "HostInput");
    assert_eq!(prop(title_input, "a11y-label").as_deref(), Some("SlotRef(\"title-label\")"));
    assert_eq!(prop(title_input, "onChange").as_deref(), Some("EmitRef(\"onTitleChange\")"));

    // Body: the multi-line primitive, named by the body label.
    let body = &root.children[2];
    assert_eq!(body.tag, "Input", "the legacy multi-line primitive, not HostInput");
    assert_eq!(prop(body, "multiline").as_deref(), Some("Keyword(\"true\")"));
    assert_eq!(prop(body, "a11y-label").as_deref(), Some("SlotRef(\"body-label\")"));
    assert_eq!(prop(body, "onChange").as_deref(), Some("EmitRef(\"onBodyChange\")"));
    assert!(prop(body, "placeholder").is_none(), "literal-only on three backends; see spec");

    // Actions: each gated on its own label, which is the button's label.
    let actions = &root.children[3];
    assert_eq!(actions.tag, "Row");
    for (branch, (label, emit)) in actions.children.iter().zip([
        ("save-label", "onSave"),
        ("delete-label", "onDelete"),
        ("cancel-label", "onCancel"),
    ]) {
        assert_eq!(branch.tag, "If");
        assert_eq!(prop(branch, "when"), Some(format!("SlotRef({label:?})")));
        let button = &branch.children[0];
        assert_eq!(button.tag, "HostButton");
        assert_eq!(prop(button, "label"), Some(format!("SlotRef({label:?})")));
        assert_eq!(prop(button, "onClick"), Some(format!("EmitRef({emit:?})")));
        assert!(button.children.is_empty(), "HostButton children are dropped (#15921)");
    }
}

/// The package builds for every backend (the web ones included).
#[test]
fn builds_on_every_backend() {
    for backend in [
        Backend::React,
        Backend::WebComponent,
        Backend::Html,
        Backend::SwiftUI,
        Backend::Compose,
        Backend::Flutter,
        Backend::Qt,
        Backend::Xaml,
    ] {
        let out = tempfile::TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: package_root(),
            output_root: out.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} build failed: {e}"));
    }
}
