//! package_compiles — the Journal app package contract.
//!
//! The sources compile in both themes, the manifest parses, the three
//! components it composes resolve, the Rust app's props are exactly this
//! package's slots, and it builds on all eight backends.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use journal_mosaic_app::JournalMosaicApp;
use mosaic_app_runtime::{MosaicApp, Platform, StartContext};
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_package_resolver::{Resolution, Resolver};

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_source(name: &str) -> String {
    let path = package_root().join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn resolver() -> Resolver {
    let packages = package_root().join("../../../packages");
    mosaic_package_resolver::build(&package_root(), &[packages.clone(), packages.join("mosaic")])
        .expect("Journal's dependencies resolve")
}

#[test]
fn manifest_declares_the_app_and_its_dependencies() {
    let manifest = mosaic_package_manifest::parse(
        &fs::read_to_string(package_root().join("mosaic-package.toml")).unwrap(),
    )
    .expect("manifest parses and validates");
    assert_eq!(manifest.package.name, "journal-app");
    assert_eq!(manifest.components.exports, ["JournalApp"]);
    assert!(manifest.dependencies.contains_key("mosaic-pkg-toolkit"));
    assert!(manifest.dependencies.contains_key("mosaic-pkg-draft-editor"));
}

#[test]
fn sources_compile_in_both_themes() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).expect("mil compiles");
    let mll = moslayout_compiler::compile(&read_source("JournalApp.mll"), Some(&mil.descriptor_json))
        .expect("mll compiles against the mil");
    for theme in ["light", "dark"] {
        mosstyle_compiler::compile(
            &read_source(&format!("JournalApp.{theme}.msl")),
            Some(&mll.part_map_json),
        )
        .unwrap_or_else(|e| panic!("{theme} msl compiles: {e:?}"));
    }
}

#[test]
fn the_composed_components_resolve() {
    let r = resolver();
    for (component, package) in [
        ("RecordList", "mosaic-pkg-toolkit"),
        ("EmptyState", "mosaic-pkg-toolkit"),
        ("DraftEditor", "mosaic-pkg-draft-editor"),
    ] {
        match r.resolve(component) {
            Some(Resolution::Component { package: p, .. }) => assert_eq!(p, package, "{component}"),
            other => panic!("expected {component} to resolve, got {other:?}"),
        }
    }
}

/// The Rust app and this interface are one contract: every slot is a prop the
/// app provides, and the app provides nothing the interface does not declare.
#[test]
fn the_apps_props_are_exactly_the_slots() {
    let mil = mosmodel_compiler::compile(&read_source("JournalApp.mil")).unwrap();
    let slots: BTreeSet<String> = mil.component.slots.iter().map(|s| s.name.clone()).collect();

    let mut app = JournalMosaicApp::default();
    let update = app.start(StartContext::new("en-US", Platform::Linux)).unwrap();
    let props: BTreeSet<String> = update.props.as_object().unwrap().keys().cloned().collect();
    assert_eq!(props, slots);
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
