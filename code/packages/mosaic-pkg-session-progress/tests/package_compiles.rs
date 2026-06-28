use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["SessionProgress"];

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

fn compiled_session_progress() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("SessionProgress.mil"))
        .expect("SessionProgress.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("SessionProgress.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("SessionProgress.mll should compile against SessionProgress.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("SessionProgress.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("SessionProgress.dark.msl should compile against SessionProgress.mll parts");

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
    assert_eq!(name, "mosaic-pkg-session-progress");

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
fn session_progress_frontend_sources_compile() {
    let (component, layout, style) = compiled_session_progress();

    assert_eq!(component.component, "SessionProgress");
    assert_eq!(layout.component_name, "SessionProgress");
    assert_eq!(style.component_name, "SessionProgress");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "current-label",
            "current-value",
            "remaining-label",
            "remaining-value",
            "correct-label",
            "correct-value",
            "total-label",
            "total-value",
        ]
    );

    assert!(component.emits.is_empty());
}

#[test]
fn session_progress_layout_binds_all_counter_slots() {
    let source = read_source("SessionProgress.mll");

    for (metric, label, value) in [
        ("metric-current", "current-label", "current-value"),
        ("metric-remaining", "remaining-label", "remaining-value"),
        ("metric-correct", "correct-label", "correct-value"),
        ("metric-total", "total-label", "total-value"),
    ] {
        assert!(
            source.contains(&format!("Box [ {metric} ]")),
            "SessionProgress.mll must expose {metric}"
        );
        assert!(
            source.contains(&format!("content : slot: {label}")),
            "SessionProgress.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("content : slot: {value}")),
            "SessionProgress.mll must bind {value}"
        );
    }
}

#[test]
fn session_progress_pipeline_emitters_all_accept_the_same_sources() {
    let (component, layout, style) = compiled_session_progress();

    let react = mosaic_emit_react::pipeline::from_pipeline(&component, &layout, &style)
        .expect("React emitter should compile SessionProgress");
    assert!(react.output.contains("SessionProgress"));

    let html = mosaic_emit_html::from_pipeline(&component, &layout, &style)
        .expect("HTML emitter should compile SessionProgress");
    assert!(html
        .output
        .contains("data-mosaic-component=\"SessionProgress\""));
    assert!(html.output.contains("#0f766e"));

    let swift = mosaic_emit_swiftui::from_pipeline(&component, &layout, &style)
        .expect("SwiftUI emitter should compile SessionProgress");
    assert!(swift.output.contains("struct SessionProgress"));

    let qt = mosaic_emit_qt::from_pipeline(&component, &layout, &style)
        .expect("Qt emitter should compile SessionProgress");
    assert!(qt.output.contains("SessionProgress"));

    let compose = mosaic_emit_compose::from_pipeline(&component, &layout, &style)
        .expect("Compose emitter should compile SessionProgress");
    assert!(compose.output.contains("SessionProgress"));

    let flutter = mosaic_emit_flutter::from_pipeline(&component, &layout, &style)
        .expect("Flutter emitter should compile SessionProgress");
    assert!(flutter.output.contains("SessionProgress"));

    let xaml = mosaic_emit_xaml::from_pipeline(
        &component,
        &layout,
        &style,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("XAML emitter should compile SessionProgress");
    assert!(xaml.xaml.contains("SessionProgress"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "SessionProgress.mil",
        "SessionProgress.mll",
        "SessionProgress.dark.msl",
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
