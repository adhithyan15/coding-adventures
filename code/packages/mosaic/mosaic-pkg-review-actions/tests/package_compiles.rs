use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["ReviewActions"];

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

fn compiled_review_actions() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("ReviewActions.mil"))
        .expect("ReviewActions.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("ReviewActions.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("ReviewActions.mll should compile against ReviewActions.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("ReviewActions.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("ReviewActions.dark.msl should compile against ReviewActions.mll parts");

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
    assert_eq!(name, "mosaic-pkg-review-actions");

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
fn review_actions_frontend_sources_compile() {
    let (component, layout, style) = compiled_review_actions();

    assert_eq!(component.component, "ReviewActions");
    assert_eq!(layout.component_name, "ReviewActions");
    assert_eq!(style.component_name, "ReviewActions");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "undo-label",
            "bury-card-label",
            "bury-siblings-label",
            "suspend-card-label",
            "mark-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onUndo",
            "onBuryCard",
            "onBurySiblings",
            "onSuspendCard",
            "onToggleMark",
        ]
    );
}

#[test]
fn review_actions_layout_wires_all_events() {
    let source = read_source("ReviewActions.mll");

    for (part, label, emit) in [
        ("action-undo", "undo-label", "onUndo"),
        ("action-bury-card", "bury-card-label", "onBuryCard"),
        (
            "action-bury-siblings",
            "bury-siblings-label",
            "onBurySiblings",
        ),
        ("action-suspend-card", "suspend-card-label", "onSuspendCard"),
        ("action-toggle-mark", "mark-label", "onToggleMark"),
    ] {
        assert!(
            source.contains(&format!("HostButton [ {part} ]")),
            "ReviewActions.mll must expose {part}"
        );
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "ReviewActions.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "ReviewActions.mll must wire {emit}"
        );
    }
}

#[test]
fn review_actions_pipeline_emitters_all_accept_the_same_sources() {
    let (component, layout, style) = compiled_review_actions();

    let react = mosaic_emit_react::pipeline::from_pipeline(&component, &layout, &style)
        .expect("React emitter should compile ReviewActions");
    assert!(react.output.contains("ReviewActions"));

    let html = mosaic_emit_html::from_pipeline(&component, &layout, &style)
        .expect("HTML emitter should compile ReviewActions");
    assert!(html
        .output
        .contains("data-mosaic-component=\"ReviewActions\""));
    assert!(html.output.contains("#7c3aed"));

    let swift = mosaic_emit_swiftui::from_pipeline(&component, &layout, &style)
        .expect("SwiftUI emitter should compile ReviewActions");
    assert!(swift.output.contains("struct ReviewActions"));

    let qt = mosaic_emit_qt::from_pipeline(&component, &layout, &style)
        .expect("Qt emitter should compile ReviewActions");
    assert!(qt.output.contains("ReviewActions"));

    let compose = mosaic_emit_compose::from_pipeline(&component, &layout, &style)
        .expect("Compose emitter should compile ReviewActions");
    assert!(compose.output.contains("ReviewActions"));

    let flutter = mosaic_emit_flutter::from_pipeline(&component, &layout, &style)
        .expect("Flutter emitter should compile ReviewActions");
    assert!(flutter.output.contains("ReviewActions"));

    let xaml = mosaic_emit_xaml::from_pipeline(
        &component,
        &layout,
        &style,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("XAML emitter should compile ReviewActions");
    assert!(xaml.xaml.contains("ReviewActions"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "ReviewActions.mil",
        "ReviewActions.mll",
        "ReviewActions.dark.msl",
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
