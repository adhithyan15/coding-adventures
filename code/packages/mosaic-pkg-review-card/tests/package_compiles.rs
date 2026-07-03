use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

const COMPONENTS: &[&str] = &["ReviewCard"];

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

fn compiled_review_card() -> (
    mosmodel_compiler::MosmodelComponent,
    moslayout_compiler::LayoutDef,
    mosstyle_compiler::StyleDef,
) {
    let mil = mosmodel_compiler::compile(&read_source("ReviewCard.mil"))
        .expect("ReviewCard.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("ReviewCard.mll"), Some(&mil.descriptor_json))
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
    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as TOML");

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

    let rating_controls_version = value
        .get("dependencies")
        .and_then(|d| d.get("mosaic-pkg-rating-controls"))
        .and_then(|v| v.as_str())
        .expect("[dependencies].mosaic-pkg-rating-controls must be set");
    assert_eq!(rating_controls_version, "0.1.0");

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
            "type-answer-active",
            "type-answer-label",
            "type-answer-value",
            "type-answer-placeholder",
            "type-answer-comparison-label",
            "type-answer-comparison-value",
            "type-answer-correct",
            "progress-label",
        ]
    );

    let emit_names: Vec<&str> = component.emits.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        emit_names,
        vec![
            "onReveal",
            "onTypeAnswerChange",
            "onAgain",
            "onHard",
            "onGood",
            "onEasy"
        ]
    );
}

#[test]
fn review_card_layout_keeps_review_interaction_semantics() {
    let source = read_source("ReviewCard.mll");

    assert!(source.contains("If ( when: slot: answer-visible )"));
    assert!(source.contains("If ( when: slot: type-answer-active )"));
    assert!(source.contains("HostInput [ type-answer-input ]"));
    assert!(source.contains("onChange : emit: onTypeAnswerChange"));
    assert!(source.contains("HostButton [ reveal-button ]"));
    assert!(source.contains("onClick : emit: onReveal"));
    assert!(source.contains("pkg::mosaic-pkg-rating-controls::RatingControls"));
    for (label, text) in [
        ("again-label", "Again"),
        ("hard-label", "Hard"),
        ("good-label", "Good"),
        ("easy-label", "Easy"),
    ] {
        assert!(
            source.contains(&format!("{label} : \"{text}\"")),
            "ReviewCard.mll must pass {label}"
        );
    }
    for emit in ["onAgain", "onHard", "onGood", "onEasy"] {
        assert!(
            source.contains(&format!("{emit} : emit: {emit}")),
            "ReviewCard.mll must forward {emit}"
        );
    }
}

#[test]
fn review_card_package_emitters_all_accept_nested_rating_controls() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/ReviewCard.html"),
        (Backend::React, "react/ReviewCard.tsx"),
        (Backend::SwiftUI, "swiftui/ReviewCard.swift"),
        (Backend::Qt, "qt/ReviewCard.qml"),
        (Backend::Xaml, "xaml/ReviewCard.xaml"),
        (Backend::Flutter, "flutter/ReviewCard.dart"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build ReviewCard: {e}"));

        assert_eq!(result.components_built, vec!["ReviewCard"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("ReviewCard.html"))
        .expect("ReviewCard HTML artifact should be readable");
    assert!(html.contains("data-mosaic-component=\"ReviewCard\""));
    assert!(
        html.contains("#f87171"),
        "nested RatingControls package styles should reach ReviewCard HTML"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = ["ReviewCard.mil", "ReviewCard.mll", "ReviewCard.dark.msl"];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }
}
