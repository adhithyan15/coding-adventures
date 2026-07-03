use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["RatingControls"];

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

fn compiled_rating_controls() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("RatingControls.mil"))
        .expect("RatingControls.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("RatingControls.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("RatingControls.mll should compile against RatingControls.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("RatingControls.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("RatingControls.dark.msl should compile against RatingControls.mll parts");

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
    assert_eq!(name, "mosaic-pkg-rating-controls");

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
fn rating_controls_frontend_sources_compile() {
    let (component, layout, style) = compiled_rating_controls();

    assert_eq!(component.component, "RatingControls");
    assert_eq!(layout.component_name, "RatingControls");
    assert_eq!(style.component_name, "RatingControls");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec!["again-label", "hard-label", "good-label", "easy-label"]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(emit_names, vec!["onAgain", "onHard", "onGood", "onEasy"]);
}

#[test]
fn rating_controls_layout_wires_all_answer_events() {
    let source = read_source("RatingControls.mll");

    for (part, label, emit) in [
        ("rating-again", "again-label", "onAgain"),
        ("rating-hard", "hard-label", "onHard"),
        ("rating-good", "good-label", "onGood"),
        ("rating-easy", "easy-label", "onEasy"),
    ] {
        assert!(
            source.contains(&format!("HostButton [ {part} ]")),
            "RatingControls.mll must expose {part}"
        );
        assert!(
            source.contains(&format!("label : slot: {label}")),
            "RatingControls.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("onClick : emit: {emit}")),
            "RatingControls.mll must wire {emit}"
        );
    }
}

#[test]
fn rating_controls_pipeline_emitters_all_accept_the_same_sources() {
    let (component, layout, style) = compiled_rating_controls();

    let react = mosaic_emit_react::pipeline::from_pipeline(&component, &layout, &style)
        .expect("React emitter should compile RatingControls");
    assert!(react.output.contains("RatingControls"));

    let html = mosaic_emit_html::from_pipeline(&component, &layout, &style)
        .expect("HTML emitter should compile RatingControls");
    assert!(html
        .output
        .contains("data-mosaic-component=\"RatingControls\""));
    assert!(html.output.contains("#f87171"));

    let swift = mosaic_emit_swiftui::from_pipeline(&component, &layout, &style)
        .expect("SwiftUI emitter should compile RatingControls");
    assert!(swift.output.contains("struct RatingControls"));

    let qt = mosaic_emit_qt::from_pipeline(&component, &layout, &style)
        .expect("Qt emitter should compile RatingControls");
    assert!(qt.output.contains("RatingControls"));

    let compose = mosaic_emit_compose::from_pipeline(&component, &layout, &style)
        .expect("Compose emitter should compile RatingControls");
    assert!(compose.output.contains("RatingControls"));

    let flutter = mosaic_emit_flutter::from_pipeline(&component, &layout, &style)
        .expect("Flutter emitter should compile RatingControls");
    assert!(flutter.output.contains("RatingControls"));

    let xaml = mosaic_emit_xaml::from_pipeline(
        &component,
        &layout,
        &style,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("XAML emitter should compile RatingControls");
    assert!(xaml.xaml.contains("RatingControls"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "RatingControls.mil",
        "RatingControls.mll",
        "RatingControls.dark.msl",
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
