//! Compile-check for the PhotoPickerApp Mosaic package: the interface
//! (.mil), layout (.mll), and both style themes (.msl) must compile, and
//! the manifest must declare the exported component and no effect handler of
//! its own (every backend answers `files.open` from Mosaic's platform
//! library, UI87 §7). Same shape of smoke test `task-app`/`engram-app` use.

use std::fs;
use std::path::PathBuf;

fn read(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn photo_picker_app_sources_compile() {
    let mil =
        mosmodel_compiler::compile(&read("PhotoPickerApp.mil")).expect("PhotoPickerApp.mil should compile");
    let mll = moslayout_compiler::compile(&read("PhotoPickerApp.mll"), Some(&mil.descriptor_json))
        .expect("PhotoPickerApp.mll should compile against the interface");
    let light = mosstyle_compiler::compile(&read("PhotoPickerApp.light.msl"), Some(&mll.part_map_json))
        .expect("PhotoPickerApp.light.msl should compile against the layout parts");
    let dark = mosstyle_compiler::compile(&read("PhotoPickerApp.dark.msl"), Some(&mll.part_map_json))
        .expect("PhotoPickerApp.dark.msl should compile against the layout parts");

    assert_eq!(mil.component.component, "PhotoPickerApp");
    assert_eq!(mll.def.component_name, "PhotoPickerApp");
    assert_eq!(light.def.component_name, "PhotoPickerApp");
    assert_eq!(dark.def.component_name, "PhotoPickerApp");

    let slots: Vec<&str> = mil.component.slots.iter().map(|s| s.name.as_str()).collect();
    assert!(slots.contains(&"status"));
    assert!(slots.contains(&"picking"));

    let emits: Vec<&str> = mil.component.emits.iter().map(|e| e.name.as_str()).collect();
    assert!(emits.contains(&"onPickPhoto"));
}

#[test]
fn manifest_declares_photo_picker_app_and_no_handler_of_its_own() {
    let manifest_src =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"))
            .expect("mosaic-package.toml must exist");
    let package = mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse");

    assert_eq!(package.package.name, "photo-picker-app");
    assert_eq!(package.components.exports, ["PhotoPickerApp"]);

    // No handler on any backend: Mosaic's platform library answers
    // `files.open` on Compose, SwiftUI, Qt, XAML and Flutter (UI87 §7), and a
    // package handler would never be reached -- this app claims no kinds, so
    // the router sends the standard ones to the library (UI87 §7.2, §7.4).
    assert!(
        package.host_effects.files.is_empty(),
        "no handler files: the platform library answers files.open"
    );
    assert!(package.host_effects.handlers.is_empty(), "no handlers: the platform library answers files.open");

    // The Flutter handler's `file_selector` coordinate went with it: every
    // generated Flutter project pins `file_selector` for the library itself
    // (UI87 §7.7), and this app has no other host asset.
    assert!(package.host_assets.files.is_empty());
    assert!(package.host_assets.dependencies.is_empty());
}

/// UI87 §7.4: every backend's copy is gone for good, not merely unwired.
#[test]
fn every_backend_handler_is_retired_for_the_platform_library() {
    let host = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host");
    for backend in ["compose", "qt", "xaml", "flutter"] {
        assert!(!host.join(backend).exists(), "host/{backend} is retired");
    }
    assert!(!host.exists(), "host/ is gone: the app carries no host code");
}
