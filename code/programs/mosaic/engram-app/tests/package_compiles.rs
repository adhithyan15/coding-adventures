use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_package_resolver::{Resolution, Resolver};

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
        package.dependencies.get("mosaic-pkg-review-card"),
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
    assert!(source.contains("pkg::mosaic-pkg-review-card::ReviewCard"));
    assert!(!source.contains("layout ReviewCard"));
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
