use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use engram_core_wasm::EngramSession;
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_package_resolver::{Resolution, Resolver};
use serde_json::Value;

const COMPONENTS: &[&str] = &["EngramApp"];

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

fn package_search_root() -> PathBuf {
    package_root()
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
}

fn dependency_resolver() -> Resolver {
    mosaic_package_resolver::build(&package_root(), &[package_search_root()])
        .expect("Engram app dependencies should resolve")
}

#[test]
fn manifest_declares_app_package_boundary() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("mosaic-package.toml must exist");
    let package =
        mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse and validate");

    assert_eq!(package.package.name, "engram-app");
    assert_eq!(package.components.exports, COMPONENTS);
    assert_eq!(
        package.dependencies.get("mosaic-pkg-deck-stats"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-review-card"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-session-progress"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(package.kernel.version, "1");
}

#[test]
fn app_sources_compile_without_owning_review_card_component() {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("EngramApp.mll"), Some(&mil.descriptor_json))
            .expect("EngramApp.mll should compile against EngramApp.mil");
    let msl =
        mosstyle_compiler::compile(&read_source("EngramApp.dark.msl"), Some(&mll.part_map_json))
            .expect("EngramApp.dark.msl should compile against EngramApp.mll parts");

    assert_eq!(mil.component.component, "EngramApp");
    assert_eq!(mll.def.component_name, "EngramApp");
    assert_eq!(msl.def.component_name, "EngramApp");

    let source = read_source("EngramApp.mll");
    assert!(source.contains("pkg::mosaic-pkg-deck-stats::DeckStatsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-review-card::ReviewCard"));
    assert!(source.contains("pkg::mosaic-pkg-session-progress::SessionProgress"));
    assert!(!source.contains("layout DeckStatsPanel"));
    assert!(!source.contains("layout ReviewCard"));
    assert!(!source.contains("layout SessionProgress"));
}

#[test]
fn shared_engram_app_props_match_mosaic_slots() {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    let expected_slots = mil
        .component
        .slots
        .iter()
        .map(|slot| slot.name.as_str())
        .collect::<BTreeSet<_>>();

    let session = EngramSession::new();
    let props: Value = serde_json::from_str(&session.engram_app_props("", 0))
        .expect("Engram app props should be valid JSON");
    let prop_keys = props["props"]
        .as_object()
        .expect("props should be a JSON object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    assert_eq!(props["ok"], true);
    assert_eq!(prop_keys, expected_slots);
    assert_eq!(props["props"]["answer-visible"], false);
}

#[test]
fn app_manifest_resolves_deck_stats_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("DeckStatsPanel") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-deck-stats");
            assert_eq!(component, "DeckStatsPanel");
            assert!(package_path.ends_with("mosaic-pkg-deck-stats"));
        }
        other => panic!("expected DeckStatsPanel component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_review_card_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("ReviewCard") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-review-card");
            assert_eq!(component, "ReviewCard");
            assert!(package_path.ends_with("mosaic-pkg-review-card"));
        }
        other => panic!("expected ReviewCard component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_session_progress_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("SessionProgress") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-session-progress");
            assert_eq!(component, "SessionProgress");
            assert!(package_path.ends_with("mosaic-pkg-session-progress"));
        }
        other => panic!("expected SessionProgress component resolution, got {other:?}"),
    }
}

#[test]
fn app_package_emits_multi_backend_artifacts_from_component_dependency() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/EngramApp.html"),
        (Backend::React, "react/EngramApp.tsx"),
        (Backend::SwiftUI, "swiftui/EngramApp.swift"),
        (Backend::Qt, "qt/EngramApp.qml"),
        (Backend::Xaml, "xaml/EngramApp.xaml"),
        (Backend::Flutter, "flutter/EngramApp.dart"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build EngramApp: {e}"));

        assert_eq!(result.components_built, vec!["EngramApp"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("EngramApp.html"))
        .expect("EngramApp HTML artifact should be readable");
    assert!(
        html.contains("#2563eb"),
        "DeckStatsPanel package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#e94560"),
        "ReviewCard package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#0f766e"),
        "SessionProgress package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#f87171"),
        "nested RatingControls package styles should reach EngramApp HTML"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = ["EngramApp.mil", "EngramApp.mll", "EngramApp.dark.msl"];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }
}
