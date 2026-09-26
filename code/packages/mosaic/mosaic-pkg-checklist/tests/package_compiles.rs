//! package_compiles — ChecklistRun round-trips through the three IR compilers,
//! and its interface and layout hold the properties the spec promises.

use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_source(name: &str) -> String {
    let path = package_root().join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn compiled() -> (mosmodel_compiler::MosmodelComponent, moslayout_compiler::LayoutDef) {
    let mil = mosmodel_compiler::compile(&read_source("ChecklistRun.mil")).expect("mil compiles");
    let mll = moslayout_compiler::compile(&read_source("ChecklistRun.mll"), Some(&mil.descriptor_json))
        .expect("mll compiles against the mil");
    for theme in ["light", "dark"] {
        mosstyle_compiler::compile(
            &read_source(&format!("ChecklistRun.{theme}.msl")),
            Some(&mll.part_map_json),
        )
        .unwrap_or_else(|e| panic!("{theme} msl compiles: {e:?}"));
    }
    (mil.component, mll.def)
}

fn prop(node: &moslayout_compiler::LayoutNode, name: &str) -> Option<String> {
    node.props.iter().find(|p| p.name == name).map(|p| format!("{:?}", p.value))
}

/// Every node in the tree, depth-first.
fn walk(node: &moslayout_compiler::LayoutNode) -> Vec<&moslayout_compiler::LayoutNode> {
    let mut out = vec![node];
    for c in &node.children {
        out.extend(walk(c));
    }
    out
}

#[test]
fn manifest_declares_expected_exports() {
    let value: toml::Value =
        toml::from_str(&fs::read_to_string(package_root().join("mosaic-package.toml")).unwrap()).unwrap();
    assert_eq!(value["package"]["name"].as_str(), Some("mosaic-pkg-checklist"));
    assert_eq!(value["components"]["exports"].as_array().unwrap().len(), 1);
    assert_eq!(value["components"]["exports"][0].as_str(), Some("ChecklistRun"));
    assert_eq!(value["kernel"]["version"].as_str(), Some("1"));
}

#[test]
fn interface_matches_spec() {
    let (c, _) = compiled();
    let slots: Vec<&str> = c.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slots,
        ["title", "progress-label", "rows", "yes-label", "no-label", "complete-label", "abandon-label"]
    );
    let emits: Vec<&str> = c.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emits, ["onToggle", "onAnswerYes", "onAnswerNo", "onComplete", "onAbandon"]);
}

/// The layout promises of the spec: a heading; every answer button reports the
/// kernel selected state and carries the row index (HostButton inside a For);
/// a check item is a real HostCheckbox (UI29-2 §2.1.1), checked when ticked,
/// never a "☐" glyph; no conditional compares strings (truthy markers only);
/// finishing actions are label-gated; no HostButton has children (#15921).
#[test]
fn layout_keeps_the_specs_promises() {
    let (_, layout) = compiled();
    let nodes = walk(&layout.root);

    let title = nodes.iter().find(|n| n.part_name.as_deref() == Some("checklist-title")).unwrap();
    assert_eq!(prop(title, "a11y-role").as_deref(), Some("Keyword(\"heading\")"));

    let mut toggles = 0;
    for button in nodes.iter().filter(|n| n.tag == "HostButton") {
        assert!(button.children.is_empty(), "{:?} has children", button.part_name);
        let part = button.part_name.as_deref().unwrap();
        if part.starts_with("checklist-yes") || part.starts_with("checklist-no") {
            let selected = prop(button, "selected").expect("row buttons report selection");
            assert_eq!(selected.contains("true"), part.ends_with("-on") || part.ends_with("-done"), "{part}");
            toggles += 1;
        }
    }
    assert_eq!(toggles, 4, "yes on/off + no on/off");

    let checkboxes: Vec<_> = nodes.iter().filter(|n| n.tag == "HostCheckbox").collect();
    assert_eq!(checkboxes.len(), 2, "one checkbox for a ticked item, one for an unticked");
    for checkbox in checkboxes {
        let part = checkbox.part_name.as_deref().unwrap();
        let checked = prop(checkbox, "checked").expect("a check item states its tick");
        assert_eq!(checked.contains("true"), part == "checklist-item-done", "{part}");
        assert_eq!(prop(checkbox, "onToggle"), Some("EmitRef(\"onToggle\")".to_string()));
    }
    assert!(
        !nodes.iter().any(|n| prop(n, "content").is_some_and(|c| c.contains('☐') || c.contains('☑'))),
        "no glyph stands in for a checkbox"
    );

    for n in nodes.iter().filter(|n| n.tag == "If") {
        let when = prop(n, "when").unwrap();
        assert!(!when.contains("=="), "truthy markers only, got {when}");
    }

    for (part, label, emit) in [
        ("checklist-complete", "complete-label", "onComplete"),
        ("checklist-abandon", "abandon-label", "onAbandon"),
    ] {
        let gate = nodes
            .iter()
            .find(|n| n.tag == "If" && n.children.iter().any(|c| c.part_name.as_deref() == Some(part)))
            .unwrap();
        assert_eq!(prop(gate, "when"), Some(format!("SlotRef({label:?})")));
        let button = &gate.children[0];
        assert_eq!(prop(button, "onClick"), Some(format!("EmitRef({emit:?})")));
    }
}

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
