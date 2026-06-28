use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["ReviewCard"];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn compiled_review_card() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("ReviewCard.mil"))
        .expect("ReviewCard.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("ReviewCard.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("ReviewCard.mll should compile against ReviewCard.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("ReviewCard.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("ReviewCard.dark.msl should compile against ReviewCard.mll parts");

    (mil.component, mll.def, msl.def)
}

#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("mosaic-package.toml must exist");
    let value: toml::Value = toml::from_str(&manifest_src)
        .expect("mosaic-package.toml must parse as TOML");

    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-review-card");

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
fn review_card_frontend_sources_compile() {
    let (component, layout, style) = compiled_review_card();

    assert_eq!(component.component, "ReviewCard");
    assert_eq!(layout.component_name, "ReviewCard");
    assert_eq!(style.component_name, "ReviewCard");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "deck-name",
            "prompt-label",
            "prompt",
            "answer-label",
            "answer",
            "answer-visible",
            "progress-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec!["onReveal", "onAgain", "onHard", "onGood", "onEasy"]
    );
}

#[test]
fn review_card_layout_keeps_review_interaction_semantics() {
    let source = read_source("ReviewCard.mll");

    assert!(source.contains("If ( when: slot: answer-visible )"));
    assert!(source.contains("HostButton [ reveal-button ]"));
    assert!(source.contains("onClick : emit: onReveal"));
    for emit in ["onAgain", "onHard", "onGood", "onEasy"] {
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "ReviewCard.mll must wire {emit}"
        );
    }
}

#[test]
fn review_card_pipeline_emitters_all_accept_the_same_sources() {
    let (component, layout, style) = compiled_review_card();

    let react = mosaic_emit_react::pipeline::from_pipeline(&component, &layout, &style)
        .expect("React emitter should compile ReviewCard");
    assert!(react.output.contains("ReviewCard"));

    let html = mosaic_emit_html::from_pipeline(&component, &layout, &style)
        .expect("HTML emitter should compile ReviewCard");
    assert!(html.output.contains("data-mosaic-component=\"ReviewCard\""));

    let swift = mosaic_emit_swiftui::from_pipeline(&component, &layout, &style)
        .expect("SwiftUI emitter should compile ReviewCard");
    assert!(swift.output.contains("struct ReviewCard"));

    let qt = mosaic_emit_qt::from_pipeline(&component, &layout, &style)
        .expect("Qt emitter should compile ReviewCard");
    assert!(qt.output.contains("ReviewCard"));

    let compose = mosaic_emit_compose::from_pipeline(&component, &layout, &style)
        .expect("Compose emitter should compile ReviewCard");
    assert!(compose.output.contains("ReviewCard"));

    let flutter = mosaic_emit_flutter::from_pipeline(&component, &layout, &style)
        .expect("Flutter emitter should compile ReviewCard");
    assert!(flutter.output.contains("ReviewCard"));

    let xaml = mosaic_emit_xaml::from_pipeline(
        &component,
        &layout,
        &style,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("XAML emitter should compile ReviewCard");
    assert!(xaml.xaml.contains("ReviewCard"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "ReviewCard.mil",
        "ReviewCard.mll",
        "ReviewCard.dark.msl",
    ];
    for name in expected {
        let path = src_path(name);
        assert!(path.exists(), "expected source file missing: {}", path.display());
    }
}
