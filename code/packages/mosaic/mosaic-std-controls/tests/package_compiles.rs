use std::fs;
use std::path::{Path, PathBuf};

use mosaic_package_artifact_builder::{
    build_package, build_package_with_profile, Backend, BuildOptions, BuildProfile,
};
use tempfile::{Builder, TempDir};

const COMPONENTS: &[&str] = &["Button", "Input", "Checkbox", "NumberInput", "Slider"];
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

#[test]
fn manifest_exposes_the_core_standard_controls() {
    let source = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("controls manifest must be readable");
    let manifest: toml::Value = toml::from_str(&source).expect("controls manifest must parse");

    assert_eq!(
        manifest["package"]["name"].as_str(),
        Some("mosaic-std-controls")
    );
    assert_eq!(manifest["package"]["version"].as_str(), Some("0.3.0"));
    let exports = manifest["components"]["exports"]
        .as_array()
        .expect("exports array")
        .iter()
        .map(|entry| entry.as_str().expect("string export"))
        .collect::<Vec<_>>();
    assert_eq!(exports, COMPONENTS);
    assert_eq!(
        manifest["dependencies"]["mosaic-pkg-toolkit"].as_str(),
        Some("0.11.0")
    );
    assert_eq!(
        manifest["dependencies"]["mosaic-std-foundation"].as_str(),
        Some("0.1.0")
    );
    assert_eq!(
        manifest["styles"]["token_palette"].as_str(),
        Some("tokens/controls.json")
    );
}

#[test]
fn controls_are_native_complete_on_all_five_backends_and_both_themes() {
    for theme in ["light", "dark"] {
        for &backend in NATIVE_BACKENDS {
            let output = TempDir::new().expect("temporary output");
            let mut options = build_options(&package_root(), output.path(), backend);
            options.theme = Some(theme.to_owned());
            let result = build_package_with_profile(&options, BuildProfile::NativeComplete)
                .unwrap_or_else(|error| panic!("{backend:?}/{theme} build failed: {error}"));
            assert_eq!(result.components_built, COMPONENTS);
            let emitted = result
                .artifacts
                .iter()
                .filter_map(|path| fs::read_to_string(path).ok())
                .collect::<Vec<_>>()
                .join("\n");
            let required_slider_shapes: &[&str] = match backend {
                Backend::Compose => &[
                    "contentDescription = label",
                    "clearAndSetSemantics { }",
                    "if ((step).toDouble() > 0.0)",
                ],
                Backend::Flutter => &[
                    "Semantics(label: label",
                    "ExcludeSemantics(child: Text(displayValue))",
                    "divisions: (step).toDouble() > 0 ?",
                ],
                Backend::Qt => &[
                    "Accessible.name: label",
                    "Accessible.ignored: true",
                    "stepSize: mosaicRoot.step",
                ],
                Backend::SwiftUI => &[
                    ".accessibilityLabel(_mosaicText(label))",
                    ".accessibilityHidden(true)",
                    "step: step,",
                ],
                Backend::Xaml => &[
                    "AutomationProperties.Name=\"{x:Bind Label}\"",
                    "AutomationProperties.AccessibilityView=\"Raw\"",
                    "MosaicStep=\"{x:Bind Step, Mode=OneWay}\"",
                ],
                _ => unreachable!("native backend fixture"),
            };
            for shape in required_slider_shapes {
                assert!(
                    emitted.contains(shape),
                    "{backend:?}/{theme} lost standard Slider shape {shape:?}:\n{emitted}"
                );
            }
            assert!(!emitted.contains("$foundation-"));
            assert!(!emitted.contains("pkg::"));
        }
    }
}

#[test]
fn slider_requires_a_human_label_and_attaches_it_to_the_native_control() {
    let interface = fs::read_to_string(package_root().join("src/Slider.mil"))
        .expect("Slider interface must be readable");
    let layout = fs::read_to_string(package_root().join("src/Slider.mll"))
        .expect("Slider layout must be readable");

    assert!(interface.contains("slot label : text ;"));
    assert!(interface.contains("slot display-value : text = \"\" ;"));
    assert!(layout.contains("Text [ slider-label ] ( content: slot: label )"));
    assert!(layout.contains("a11y-label: slot: label"));
    assert!(layout.contains("a11y-hidden: true"));
}

fn make_consumer_package() -> TempDir {
    let controls_package = package_root();
    let mosaic_packages = controls_package.parent().expect("controls package parent");
    let consumer = Builder::new()
        .prefix(".mosaic-controls-consumer-")
        .tempdir_in(mosaic_packages)
        .expect("temporary consumer beside standard packages");
    fs::create_dir(consumer.path().join("src")).expect("consumer src");
    fs::write(
        consumer.path().join("mosaic-package.toml"),
        r#"[package]
name = "mosaic-controls-consumer"
version = "0.1.0"
description = "Acceptance fixture for standard controls"
license = "MIT OR Apache-2.0"

[components]
exports = ["Consumer"]

[dependencies]
mosaic-std-controls = "0.3.0"
mosaic-std-foundation = "0.1.0"

[kernel]
version = "1"
"#,
    )
    .expect("consumer manifest");
    fs::write(
        consumer.path().join("src/Consumer.mil"),
        r#"component Consumer {
  emit onContinue ;
  emit onEmailChange ( value : text ) ;
  emit onEmailCommit ;
  emit onRememberChange ( checked : bool ) ;
  emit onTeamSizeChange ( value : number ) ;
  emit onVolumeChange ( value : number ) ;
  emit onVolumeCommit ( value : number ) ;
}
"#,
    )
    .expect("consumer MIL");
    fs::write(
        consumer.path().join("src/Consumer.mll"),
        r#"layout Consumer {
  pkg::mosaic-std-foundation::Surface {
    pkg::mosaic-std-foundation::HeadingText ( content: "Sign in" )
    pkg::mosaic-std-controls::Input (
      placeholder: "Email address",
      onChange: emit: onEmailChange,
      onCommit: emit: onEmailCommit
    )
    pkg::mosaic-std-controls::NumberInput (
      placeholder: "Team size",
      onChange: emit: onTeamSizeChange
    )
    pkg::mosaic-std-controls::Checkbox (
      label: "Remember this device",
      onChange: emit: onRememberChange
    )
    pkg::mosaic-std-controls::Slider (
      label: "Notification volume",
      value: 65,
      min: 0,
      max: 100,
      step: 5,
      display-value: "65%",
      onChange: emit: onVolumeChange,
      onCommit: emit: onVolumeCommit
    )
    pkg::mosaic-std-controls::Button (
      label: "Continue",
      onClick: emit: onContinue
    )
  }
}
"#,
    )
    .expect("consumer MLL");
    fs::write(
        consumer.path().join("src/Consumer.msl"),
        "style Consumer { }\n",
    )
    .expect("consumer MSL");
    consumer
}

fn make_html_token_consumer_package() -> TempDir {
    let consumer = make_consumer_package();
    let mil_path = consumer.path().join("src/Consumer.mil");
    let mil = fs::read_to_string(&mil_path)
        .expect("HTML consumer MIL")
        .replace("  emit onVolumeChange ( value : number ) ;\n", "")
        .replace("  emit onVolumeCommit ( value : number ) ;\n", "");
    fs::write(mil_path, mil).expect("slider-free HTML consumer MIL");

    let mll_path = consumer.path().join("src/Consumer.mll");
    let mll = fs::read_to_string(&mll_path)
        .expect("HTML consumer MLL")
        .replace(
            r#"    pkg::mosaic-std-controls::Slider (
      label: "Notification volume",
      value: 65,
      min: 0,
      max: 100,
      step: 5,
      display-value: "65%",
      onChange: emit: onVolumeChange,
      onCommit: emit: onVolumeCommit
    )
"#,
            "",
        );
    fs::write(mll_path, mll).expect("slider-free HTML consumer MLL");
    consumer
}

#[test]
fn included_controls_build_a_native_complete_sign_in_surface_everywhere() {
    let consumer = make_consumer_package();
    for &backend in NATIVE_BACKENDS {
        let output = TempDir::new().expect("temporary output");
        let options = build_options(consumer.path(), output.path(), backend);
        let result = build_package_with_profile(&options, BuildProfile::NativeComplete)
            .unwrap_or_else(|error| panic!("consumer {backend:?} build failed: {error}"));
        let emitted = result
            .artifacts
            .iter()
            .filter_map(|path| fs::read_to_string(path).ok())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            emitted.contains("Email address"),
            "{backend:?} lost placeholder:\n{emitted}"
        );
        assert!(
            emitted.contains("Continue"),
            "{backend:?} lost button label"
        );
        assert!(
            emitted.contains("Team size"),
            "{backend:?} lost number placeholder:\n{emitted}"
        );
        assert!(
            emitted.contains("Remember this device"),
            "{backend:?} lost checkbox label:\n{emitted}"
        );
        assert!(
            emitted.contains("Notification volume"),
            "{backend:?} lost slider label:\n{emitted}"
        );
        assert!(
            emitted.contains("65%"),
            "{backend:?} lost formatted slider value:\n{emitted}"
        );
        assert!(!emitted.contains("$foundation-"));
        assert!(!emitted.contains("pkg::"));
        assert!(!emitted.contains("$mosaic-child-slot"));
    }

    let html_consumer = make_html_token_consumer_package();
    let html_output = TempDir::new().expect("HTML output");
    let mut options = build_options(html_consumer.path(), html_output.path(), Backend::Html);
    options.theme = Some("light".to_owned());
    let result = build_package(&options).expect("HTML consumer build");
    let html = result
        .artifacts
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n");
    for value in ["#5b5bd6", "#d8dee8", "4px", "6px", "16px"] {
        assert!(
            html.contains(value),
            "missing Foundation control value {value}"
        );
    }
}
