use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["DeckOptionsPanel"];

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

fn compiled_deck_options_panel() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("DeckOptionsPanel.mil"))
        .expect("DeckOptionsPanel.mil should compile");
    let mll = moslayout_compiler::compile(
        &read_source("DeckOptionsPanel.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("DeckOptionsPanel.mll should compile against DeckOptionsPanel.mil");
    let msl = mosstyle_compiler::compile(
        &read_source("DeckOptionsPanel.dark.msl"),
        Some(&mll.part_map_json),
    )
    .expect("DeckOptionsPanel.dark.msl should compile against DeckOptionsPanel.mll parts");

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
    assert_eq!(name, "mosaic-pkg-deck-options");

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
fn deck_options_frontend_sources_compile() {
    let (component, layout, style) = compiled_deck_options_panel();

    assert_eq!(component.component, "DeckOptionsPanel");
    assert_eq!(layout.component_name, "DeckOptionsPanel");
    assert_eq!(style.component_name, "DeckOptionsPanel");

    let slot_names: Vec<&str> = component.slots.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        slot_names,
        vec![
            "settings-label",
            "learning-steps-label",
            "learning-steps-value",
            "relearning-steps-label",
            "relearning-steps-value",
            "new-cards-label",
            "new-cards-value",
            "reviews-label",
            "reviews-value",
            "graduating-interval-label",
            "graduating-interval-value",
            "easy-interval-label",
            "easy-interval-value",
            "initial-ease-label",
            "initial-ease-value",
            "maximum-interval-label",
            "maximum-interval-value",
            "interval-modifier-label",
            "interval-modifier-value",
            "hard-multiplier-label",
            "hard-multiplier-value",
            "easy-bonus-label",
            "easy-bonus-value",
            "lapse-multiplier-label",
            "lapse-multiplier-value",
            "leech-threshold-label",
            "leech-threshold-value",
            "desired-retention-label",
            "desired-retention-value",
            "fsrs-parameters-label",
            "fsrs-parameters-value",
            "fsrs-search-label",
            "fsrs-search-value",
            "ignore-review-history-before-label",
            "ignore-review-history-before-value",
            "historical-retention-label",
            "historical-retention-value",
            "easy-days-percentages-label",
            "easy-days-percentages-value",
            "leech-action-label",
            "leech-action-suspend-label",
            "leech-action-suspend-value",
            "leech-action-tag-only-label",
            "leech-action-tag-only-value",
            "bury-new-siblings-label",
            "bury-new-siblings-value",
            "bury-review-siblings-label",
            "bury-review-siblings-value",
            "bury-interday-learning-siblings-label",
            "bury-interday-learning-siblings-value",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onLearningStepsChange",
            "onRelearningStepsChange",
            "onNewCardsChange",
            "onReviewsChange",
            "onGraduatingIntervalChange",
            "onEasyIntervalChange",
            "onInitialEaseChange",
            "onMaximumIntervalChange",
            "onIntervalModifierChange",
            "onHardMultiplierChange",
            "onEasyBonusChange",
            "onLapseMultiplierChange",
            "onLeechThresholdChange",
            "onDesiredRetentionChange",
            "onFsrsParametersChange",
            "onFsrsSearchChange",
            "onIgnoreReviewHistoryBeforeChange",
            "onHistoricalRetentionChange",
            "onEasyDaysPercentagesChange",
            "onLeechActionChange",
            "onBuryNewSiblingsChange",
            "onBuryReviewSiblingsChange",
            "onBuryInterdayLearningSiblingsChange",
        ]
    );
}

#[test]
fn deck_options_layout_wires_number_controls() {
    let source = read_source("DeckOptionsPanel.mll");

    for (value_slot, emit) in [
        ("learning-steps-value", "onLearningStepsChange"),
        ("relearning-steps-value", "onRelearningStepsChange"),
    ] {
        assert!(
            source.contains(&format!("value : slot: {value_slot}")),
            "DeckOptionsPanel.mll must bind {value_slot}"
        );
        assert!(
            source.contains(&format!("onChange : emit: {emit}")),
            "DeckOptionsPanel.mll must wire {emit}"
        );
    }

    for (value_slot, emit) in [
        ("new-cards-value", "onNewCardsChange"),
        ("reviews-value", "onReviewsChange"),
        ("graduating-interval-value", "onGraduatingIntervalChange"),
        ("easy-interval-value", "onEasyIntervalChange"),
        ("initial-ease-value", "onInitialEaseChange"),
        ("maximum-interval-value", "onMaximumIntervalChange"),
        ("interval-modifier-value", "onIntervalModifierChange"),
        ("hard-multiplier-value", "onHardMultiplierChange"),
        ("easy-bonus-value", "onEasyBonusChange"),
        ("lapse-multiplier-value", "onLapseMultiplierChange"),
        ("leech-threshold-value", "onLeechThresholdChange"),
        ("desired-retention-value", "onDesiredRetentionChange"),
        ("historical-retention-value", "onHistoricalRetentionChange"),
    ] {
        assert!(
            source.contains(&format!("value : slot: {value_slot}")),
            "DeckOptionsPanel.mll must bind {value_slot}"
        );
        assert!(
            source.contains(&format!("onChange : emit: {emit}")),
            "DeckOptionsPanel.mll must wire {emit}"
        );
    }

    for label_slot in [
        "learning-steps-label",
        "relearning-steps-label",
        "new-cards-label",
        "reviews-label",
        "graduating-interval-label",
        "easy-interval-label",
        "initial-ease-label",
        "maximum-interval-label",
        "interval-modifier-label",
        "hard-multiplier-label",
        "easy-bonus-label",
        "lapse-multiplier-label",
        "leech-threshold-label",
        "desired-retention-label",
        "fsrs-parameters-label",
        "fsrs-search-label",
        "ignore-review-history-before-label",
        "historical-retention-label",
        "easy-days-percentages-label",
        "leech-action-label",
        "leech-action-suspend-label",
        "leech-action-tag-only-label",
        "bury-new-siblings-label",
        "bury-review-siblings-label",
        "bury-interday-learning-siblings-label",
    ] {
        assert!(
            source.contains(&format!("content : slot: {label_slot}"))
                || source.contains(&format!("label : slot: {label_slot}")),
            "DeckOptionsPanel.mll must bind {label_slot}"
        );
    }

    for (value_slot, emit) in [
        ("fsrs-parameters-value", "onFsrsParametersChange"),
        ("fsrs-search-value", "onFsrsSearchChange"),
        (
            "ignore-review-history-before-value",
            "onIgnoreReviewHistoryBeforeChange",
        ),
        ("easy-days-percentages-value", "onEasyDaysPercentagesChange"),
    ] {
        assert!(
            source.contains(&format!("value : slot: {value_slot}")),
            "DeckOptionsPanel.mll must bind {value_slot}"
        );
        assert!(
            source.contains(&format!("onChange : emit: {emit}")),
            "DeckOptionsPanel.mll must wire {emit}"
        );
    }

    for (value_slot, emit) in [
        ("bury-new-siblings-value", "onBuryNewSiblingsChange"),
        ("bury-review-siblings-value", "onBuryReviewSiblingsChange"),
        (
            "bury-interday-learning-siblings-value",
            "onBuryInterdayLearningSiblingsChange",
        ),
    ] {
        assert!(
            source.contains(&format!("checked : slot: {value_slot}")),
            "DeckOptionsPanel.mll must bind {value_slot}"
        );
        assert!(
            source.contains(&format!("onToggle : emit: {emit}")),
            "DeckOptionsPanel.mll must wire {emit}"
        );
    }

    for (value_slot, option) in [
        ("leech-action-suspend-value", "suspend"),
        ("leech-action-tag-only-value", "tag-only"),
    ] {
        assert!(
            source.contains(&format!("checked : slot: {value_slot}")),
            "DeckOptionsPanel.mll must bind {value_slot}"
        );
        assert!(
            source.contains(&format!("value : \"{option}\"")),
            "DeckOptionsPanel.mll must set radio value {option}"
        );
    }
    assert!(
        source.contains("onSelect : emit: onLeechActionChange"),
        "DeckOptionsPanel.mll must wire onLeechActionChange"
    );
}

#[test]
fn deck_options_package_emitters_accept_settings_surface() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/DeckOptionsPanel.html"),
        (Backend::React, "react/DeckOptionsPanel.tsx"),
        (Backend::SwiftUI, "swiftui/DeckOptionsPanel.swift"),
        (Backend::Qt, "qt/DeckOptionsPanel.qml"),
        (Backend::Xaml, "xaml/DeckOptionsPanel.xaml"),
        (Backend::Flutter, "flutter/DeckOptionsPanel.dart"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build DeckOptionsPanel: {e}"));

        assert_eq!(result.components_built, vec!["DeckOptionsPanel"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("DeckOptionsPanel.html"))
        .expect("DeckOptionsPanel HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"DeckOptionsPanel\""));
    assert!(
        html.contains("#f59e0b"),
        "deck options style should reach HTML artifact"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "DeckOptionsPanel.mil",
        "DeckOptionsPanel.mll",
        "DeckOptionsPanel.dark.msl",
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
