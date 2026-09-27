//! Compile-check for the PhotoPickerApp Mosaic package: the interface
//! (.mil), layout (.mll), and both style themes (.msl) must compile, and
//! the manifest must declare the exported component and the
//! XAML/Flutter `[host_effects]` handlers (Compose, SwiftUI and Qt answer
//! `files.open` from Mosaic's platform library, UI87 §7). Same shape of smoke
//! test `task-app`/`engram-app` use.

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
fn manifest_declares_photo_picker_app_and_the_xaml_and_flutter_files_open_handlers() {
    let manifest_src =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"))
            .expect("mosaic-package.toml must exist");
    let package = mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse");

    assert_eq!(package.package.name, "photo-picker-app");
    assert_eq!(package.components.exports, ["PhotoPickerApp"]);

    let xaml_file = package
        .host_effects
        .files
        .iter()
        .find(|file| file.backend == "xaml")
        .expect("must declare the XAML handler source file");
    assert_eq!(xaml_file.source, "host/xaml/PhotoPickerEffects.cs");
    assert_eq!(xaml_file.target, "PhotoPickerEffects.cs");

    let xaml_handler = package
        .host_effects
        .handlers
        .iter()
        .find(|handler| handler.backend == "xaml")
        .expect("must declare the XAML effect handler");
    assert_eq!(xaml_handler.install, "PhotoPickerHost.PhotoPickerEffects.Install");
    // No `include` for XAML -- the emitter refuses one outright (UI47
    // §5.5.2, confirmed against `xaml_main_with_host_effects`'s own error
    // message). A regression here would mean the manifest asks for
    // something the build will hard-fail on.
    assert_eq!(xaml_handler.include, None);

    // No Compose, SwiftUI or Qt handler: Mosaic's platform library answers
    // `files.open` on those backends (UI87 §7), and a package handler for a
    // backend would never be reached -- this app claims no kinds, so the
    // router sends the standard ones to the library (UI87 §7.2, §7.4).
    for backend in ["compose", "swiftui", "qt"] {
        assert!(
            package.host_effects.files.iter().all(|file| file.backend != backend),
            "no {backend} handler file: the platform library answers files.open"
        );
        assert!(
            package.host_effects.handlers.iter().all(|handler| handler.backend != backend),
            "no {backend} handler: the platform library answers files.open"
        );
    }

    // Dart resolves nothing across files without an import, so `include`
    // names the file relative to `lib/` -- matching engram-app's own
    // Flutter `[host_effects]` entry exactly.
    let flutter_file = package
        .host_effects
        .files
        .iter()
        .find(|file| file.backend == "flutter")
        .expect("must declare the Flutter handler source file");
    assert_eq!(flutter_file.source, "host/flutter/PhotoPickerEffects.dart");
    assert_eq!(flutter_file.target, "lib/PhotoPickerEffects.dart");

    let flutter_handler = package
        .host_effects
        .handlers
        .iter()
        .find(|handler| handler.backend == "flutter")
        .expect("must declare the Flutter effect handler");
    assert_eq!(flutter_handler.install, "installPhotoPickerEffects");
    assert_eq!(flutter_handler.include.as_deref(), Some("PhotoPickerEffects.dart"));

    // `[host_assets]` exists here ONLY for the Flutter handler's pub
    // dependency -- no `[host_assets].files` entries, unlike Engram.
    assert!(package.host_assets.files.is_empty());
    let flutter_dependency = package
        .host_assets
        .dependencies
        .iter()
        .find(|dependency| dependency.backend == "flutter")
        .expect("must declare file_selector as a Flutter host_assets dependency");
    assert_eq!(flutter_dependency.coordinate, "file_selector: '>=1.0.0 <2.0.0'");

    // No other backend is declared -- SwiftUI is out of scope for this
    // environment (UI59 §2), not silently half-wired here.
    for backend in ["swiftui"] {
        assert!(
            !package.host_effects.files.iter().any(|f| f.backend == backend),
            "{backend} should not have a host_effects file"
        );
        assert!(
            !package.host_effects.handlers.iter().any(|h| h.backend == backend),
            "{backend} should not have a host_effects handler"
        );
    }
}

#[test]
fn xaml_handler_source_exists_and_declares_install() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host/xaml/PhotoPickerEffects.cs"),
    )
    .expect("host/xaml/PhotoPickerEffects.cs must exist");
    assert!(source.contains("public static void Install()"));
    assert!(source.contains("MosaicRuntimeHost.EffectHandler"));
    assert!(source.contains("\"files.open\""));
}

/// UI87 §7.4: the Compose and Qt copies are gone for good, not merely unwired.
#[test]
fn the_compose_and_qt_handlers_are_retired_for_the_platform_library() {
    for backend in ["compose", "qt"] {
        assert!(
            !PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("host")
                .join(backend)
                .exists(),
            "host/{backend} is retired"
        );
    }
}

#[test]
fn flutter_handler_source_exists_and_declares_install() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host/flutter/PhotoPickerEffects.dart"),
    )
    .expect("host/flutter/PhotoPickerEffects.dart must exist");
    assert!(source.contains("void installPhotoPickerEffects(MosaicHost host)"));
    assert!(source.contains("effectHandler"));
    assert!(source.contains("'files.open'"));
}
