use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["DeckStatsPanel"];

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

fn compiled_deck_stats_panel() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("DeckStatsPanel.mil"))
        .expect("DeckStatsPanel.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("DeckStatsPanel.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("DeckStatsPanel.mll should compile against DeckStatsPanel.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("DeckStatsPanel.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("DeckStatsPanel.dark.msl should compile against DeckStatsPanel.mll parts");

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
    assert_eq!(name, "mosaic-pkg-deck-stats");

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
fn deck_stats_frontend_sources_compile() {
    let (component, layout, style) = compiled_deck_stats_panel();

    assert_eq!(component.component, "DeckStatsPanel");
    assert_eq!(layout.component_name, "DeckStatsPanel");
    assert_eq!(style.component_name, "DeckStatsPanel");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "deck-label",
            "deck-name",
            "total-label",
            "total-value",
            "new-label",
            "new-value",
            "due-label",
            "due-value",
            "learning-label",
            "learning-value",
            "hidden-label",
            "hidden-value",
        ]
    );

    assert!(component.emits.is_empty());
}

#[test]
fn deck_stats_layout_binds_all_stat_slots() {
    let source = read_source("DeckStatsPanel.mll");

    for (part, label, value) in [
        ("deck-stat-total", "total-label", "total-value"),
        ("deck-stat-new", "new-label", "new-value"),
        ("deck-stat-due", "due-label", "due-value"),
        ("deck-stat-learning", "learning-label", "learning-value"),
        ("deck-stat-hidden", "hidden-label", "hidden-value"),
    ] {
        assert!(
            source.contains(&format!("Box [ {part} ]")),
            "DeckStatsPanel.mll must expose {part}"
        );
        assert!(
            source.contains(&format!("content : slot: {label}")),
            "DeckStatsPanel.mll must bind {label}"
        );
        assert!(
            source.contains(&format!("content : slot: {value}")),
            "DeckStatsPanel.mll must bind {value}"
        );
    }
}

#[test]
fn deck_stats_pipeline_emitters_all_accept_the_same_sources() {
    let (component, layout, style) = compiled_deck_stats_panel();

    let react = mosaic_emit_react::pipeline::from_pipeline(&component, &layout, &style)
        .expect("React emitter should compile DeckStatsPanel");
    assert!(react.output.contains("DeckStatsPanel"));

    let html = mosaic_emit_html::from_pipeline(&component, &layout, &style)
        .expect("HTML emitter should compile DeckStatsPanel");
    assert!(html
        .output
        .contains("data-mosaic-component=\"DeckStatsPanel\""));
    assert!(html.output.contains("#2563eb"));

    let swift = mosaic_emit_swiftui::from_pipeline(&component, &layout, &style)
        .expect("SwiftUI emitter should compile DeckStatsPanel");
    assert!(swift.output.contains("struct DeckStatsPanel"));

    let qt = mosaic_emit_qt::from_pipeline(&component, &layout, &style)
        .expect("Qt emitter should compile DeckStatsPanel");
    assert!(qt.output.contains("DeckStatsPanel"));

    let compose = mosaic_emit_compose::from_pipeline(&component, &layout, &style)
        .expect("Compose emitter should compile DeckStatsPanel");
    assert!(compose.output.contains("DeckStatsPanel"));

    let flutter = mosaic_emit_flutter::from_pipeline(&component, &layout, &style)
        .expect("Flutter emitter should compile DeckStatsPanel");
    assert!(flutter.output.contains("DeckStatsPanel"));

    let xaml = mosaic_emit_xaml::from_pipeline(
        &component,
        &layout,
        &style,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("XAML emitter should compile DeckStatsPanel");
    assert!(xaml.xaml.contains("DeckStatsPanel"));
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "DeckStatsPanel.mil",
        "DeckStatsPanel.mll",
        "DeckStatsPanel.dark.msl",
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
