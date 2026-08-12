use std::fs;
use std::path::{Path, PathBuf};

use mosaic_package_artifact_builder::{
    analyze_package_degradations, build_package, build_package_with_profile, Backend, BuildOptions,
    BuildProfile,
};
use tempfile::{Builder, TempDir};

const LEAF_COMPONENTS: &[&str] = &[
    "DisplayText",
    "HeadingText",
    "BodyText",
    "CaptionText",
    "FoundationIcon",
];

const COMPONENTS: &[&str] = &[
    "DisplayText",
    "HeadingText",
    "BodyText",
    "CaptionText",
    "FoundationIcon",
    "Surface",
];

const NATIVE_BACKENDS: &[Backend] = &[
    Backend::SwiftUI,
    Backend::Qt,
    Backend::Xaml,
    Backend::Flutter,
    Backend::Compose,
];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn build_options(package_root: &Path, output_root: &Path, backend: Backend) -> BuildOptions {
    BuildOptions {
        package_root: package_root.to_path_buf(),
        output_root: output_root.to_path_buf(),
        backend,
        emit_project: false,
        theme: None,
    }
}

fn make_standalone_leaf_package() -> TempDir {
    let package = TempDir::new().expect("temporary leaf package");
    fs::create_dir(package.path().join("src")).expect("leaf src directory");
    fs::create_dir(package.path().join("tokens")).expect("leaf token directory");
    fs::copy(
        package_root().join("tokens/foundation.json"),
        package.path().join("tokens/foundation.json"),
    )
    .expect("copy foundation palette");
    fs::write(
        package.path().join("mosaic-package.toml"),
        r#"[package]
name = "mosaic-std-foundation-leaves"
version = "0.1.0"
description = "Standalone acceptance fixture for Foundation leaf components"
license = "MIT OR Apache-2.0"

[components]
exports = ["DisplayText", "HeadingText", "BodyText", "CaptionText", "FoundationIcon"]

[dependencies]

[styles]
token_palette = "tokens/foundation.json"

[kernel]
version = "1"
"#,
    )
    .expect("leaf package manifest");

    for component in LEAF_COMPONENTS {
        for suffix in ["mil", "mll", "light.msl", "dark.msl"] {
            let file = format!("{component}.{suffix}");
            fs::copy(
                package_root().join("src").join(&file),
                package.path().join("src").join(file),
            )
            .expect("copy Foundation leaf source");
        }
    }
    package
}

#[test]
fn manifest_exposes_the_foundation_contract_and_palette() {
    let source = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("foundation manifest must be readable");
    let manifest: toml::Value = toml::from_str(&source).expect("foundation manifest must parse");

    assert_eq!(
        manifest["package"]["name"].as_str(),
        Some("mosaic-std-foundation")
    );
    assert_eq!(manifest["package"]["version"].as_str(), Some("0.1.0"));
    assert_eq!(manifest["kernel"]["version"].as_str(), Some("1"));
    assert_eq!(
        manifest["styles"]["token_palette"].as_str(),
        Some("tokens/foundation.json")
    );

    let exports = manifest["components"]["exports"]
        .as_array()
        .expect("exports must be an array")
        .iter()
        .map(|entry| entry.as_str().expect("exports must be strings"))
        .collect::<Vec<_>>();
    assert_eq!(exports, COMPONENTS);
    assert!(
        manifest["dependencies"]
            .as_table()
            .expect("dependencies table must exist")
            .is_empty(),
        "foundation must remain kernel-only"
    );

    let palette_source = fs::read_to_string(package_root().join("tokens/foundation.json"))
        .expect("foundation token palette must be readable");
    let palette: serde_json::Value =
        serde_json::from_str(&palette_source).expect("foundation token palette must parse");
    assert_eq!(palette["schema_version"], 1);
    for token in [
        "foundation-color-text-light",
        "foundation-color-text-dark",
        "foundation-color-surface-light",
        "foundation-color-surface-dark",
        "foundation-color-border-light",
        "foundation-color-border-dark",
        "foundation-color-accent",
        "foundation-font-display",
        "foundation-font-heading",
        "foundation-font-body",
        "foundation-font-caption",
        "foundation-icon-size",
        "foundation-space-xs",
        "foundation-space-sm",
        "foundation-space-md",
        "foundation-space-lg",
        "foundation-space-xl",
        "foundation-radius-sm",
        "foundation-radius-md",
        "foundation-radius-lg",
    ] {
        assert!(
            palette["tokens"].get(token).is_some(),
            "missing documented foundation token {token}"
        );
    }
}

#[test]
fn standalone_leaf_exports_remain_native_complete_and_surface_is_explicit_on_all_backends() {
    let leaf_package = make_standalone_leaf_package();
    for theme in ["light", "dark"] {
        for &backend in NATIVE_BACKENDS {
            let output = TempDir::new().expect("temporary output");
            let mut options = build_options(&package_root(), output.path(), backend);
            options.theme = Some(theme.to_owned());

            let report = analyze_package_degradations(&options, BuildProfile::NativeComplete)
                .unwrap_or_else(|error| panic!("{backend:?}/{theme} analysis failed: {error}"));
            assert!(
                !report.native_complete,
                "standalone Surface is still explicit"
            );
            assert_eq!(report.degradations.len(), 1, "{backend:?}/{theme} report");
            let degradation = &report.degradations[0];
            assert_eq!(degradation.component, "Surface");
            assert_eq!(
                degradation.code,
                "composition.child-slot-parameter-unimplemented"
            );

            let leaf_output = TempDir::new().expect("temporary leaf output");
            let mut leaf_options = build_options(leaf_package.path(), leaf_output.path(), backend);
            leaf_options.theme = Some(theme.to_owned());
            let result = build_package_with_profile(&leaf_options, BuildProfile::NativeComplete)
                .unwrap_or_else(|error| panic!("{backend:?}/{theme} leaf build failed: {error}"));
            assert_eq!(result.components_built, LEAF_COMPONENTS);
            assert!(
                result.artifacts.iter().all(|path| path.exists()),
                "{backend:?}/{theme} reported a missing artifact"
            );

            let emitted = result
                .artifacts
                .iter()
                .filter_map(|path| fs::read_to_string(path).ok())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !emitted.contains("$foundation-"),
                "{backend:?}/{theme} leaked an unresolved foundation token"
            );
        }
    }
}

fn make_consumer_package() -> TempDir {
    let foundation_root = package_root();
    let mosaic_packages = foundation_root
        .parent()
        .expect("foundation must live beneath code/packages/mosaic");
    let consumer = Builder::new()
        .prefix(".mosaic-foundation-consumer-")
        .tempdir_in(mosaic_packages)
        .expect("temporary consumer beside foundation package");
    fs::create_dir(consumer.path().join("src")).expect("consumer src directory");
    fs::write(
        consumer.path().join("mosaic-package.toml"),
        r#"[package]
name = "mosaic-foundation-consumer"
version = "0.1.0"
description = "Acceptance fixture for mosaic-std-foundation dependency inclusion"
license = "MIT OR Apache-2.0"

[components]
exports = ["Consumer"]

[dependencies]
mosaic-std-foundation = "0.1.0"

[kernel]
version = "1"
"#,
    )
    .expect("consumer manifest");
    fs::write(
        consumer.path().join("src/Consumer.mil"),
        "component Consumer { }\n",
    )
    .expect("consumer MIL");
    fs::write(
        consumer.path().join("src/Consumer.mll"),
        r#"layout Consumer {
  Column [ root ] {
    pkg::mosaic-std-foundation::Surface {
      pkg::mosaic-std-foundation::DisplayText ( content: "Welcome" )
      pkg::mosaic-std-foundation::BodyText (
        content: "Your native Mosaic app is ready."
      )
      pkg::mosaic-std-foundation::FoundationIcon (
        glyph: "star",
        accessible-label: "Featured"
      )
    }
  }
}
"#,
    )
    .expect("consumer MLL");
    fs::write(
        consumer.path().join("src/Consumer.msl"),
        r#"style Consumer {
  part root {
    padding : 24 ;
  }
}
"#,
    )
    .expect("consumer MSL");
    consumer
}

#[test]
fn consuming_package_inherits_components_and_token_defaults_on_every_native_backend() {
    let consumer = make_consumer_package();

    for &backend in NATIVE_BACKENDS {
        let output = TempDir::new().expect("temporary native output");
        let options = build_options(consumer.path(), output.path(), backend);
        let result = build_package_with_profile(&options, BuildProfile::NativeComplete)
            .unwrap_or_else(|error| panic!("consumer {backend:?} build failed: {error}"));

        let report_source = fs::read_to_string(
            result
                .artifacts
                .iter()
                .find(|path| path.ends_with("mosaic-degradations.json"))
                .expect("strict build must emit its degradation report"),
        )
        .expect("degradation report must be readable");
        let report: serde_json::Value =
            serde_json::from_str(&report_source).expect("degradation report must parse");
        assert_eq!(report["nativeComplete"], true, "{backend:?} report");
        assert_eq!(report["degradations"], serde_json::json!([]));

        let emitted = result
            .artifacts
            .iter()
            .filter_map(|path| fs::read_to_string(path).ok())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !emitted.contains("$foundation-"),
            "consumer {backend:?} leaked an unresolved foundation token"
        );
        assert!(
            !emitted.contains("$mosaic-child-slot"),
            "consumer {backend:?} leaked the Surface child mount"
        );
        assert!(
            emitted.contains("Welcome"),
            "consumer {backend:?} lost Surface children"
        );
    }

    let html_output = TempDir::new().expect("temporary HTML proof output");
    let mut html_options = build_options(consumer.path(), html_output.path(), Backend::Html);
    html_options.theme = Some("light".to_owned());
    let result = build_package(&html_options)
        .expect("HTML proof build must resolve dependency token defaults");
    let html = result
        .artifacts
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        html.contains("#172033"),
        "dependency text token was not applied"
    );
    assert!(
        html.contains("#5b5bd6"),
        "dependency icon token was not applied"
    );
    assert!(
        html.contains("32px"),
        "dependency type-scale token was not applied"
    );
    assert!(
        html.contains("#d8dee8"),
        "dependency Surface border token was not applied"
    );
    assert!(
        html.contains("10px"),
        "dependency Surface radius token was not applied"
    );
}
