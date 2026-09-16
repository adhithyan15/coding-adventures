//! Compile-check for the PhotoPickerApp Mosaic package: the interface
//! (.mil), layout (.mll), and both style themes (.msl) must compile, and
//! the manifest must declare the exported component and the XAML/Qt/Compose
//! `[host_effects]` handlers. Same shape of smoke test `task-app`/
//! `engram-app` use.

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
fn manifest_declares_photo_picker_app_and_the_xaml_qt_and_compose_files_open_handlers() {
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

    // Qt names both its header and source under `files` (unlike SwiftUI/
    // Compose, which compile whole directories) -- matching engram-app's own
    // Qt `[host_effects]` entry exactly.
    let qt_header = package
        .host_effects
        .files
        .iter()
        .find(|file| file.backend == "qt" && file.target == "PhotoPickerEffects.h")
        .expect("must declare the Qt header file");
    assert_eq!(qt_header.source, "host/qt/PhotoPickerEffects.h");
    let qt_source = package
        .host_effects
        .files
        .iter()
        .find(|file| file.backend == "qt" && file.target == "PhotoPickerEffects.cpp")
        .expect("must declare the Qt source file");
    assert_eq!(qt_source.source, "host/qt/PhotoPickerEffects.cpp");

    let qt_handler = package
        .host_effects
        .handlers
        .iter()
        .find(|handler| handler.backend == "qt")
        .expect("must declare the Qt effect handler");
    assert_eq!(qt_handler.install, "installPhotoPickerEffects");
    assert_eq!(qt_handler.include.as_deref(), Some("PhotoPickerEffects.h"));

    // Compose compiles everything under `src/main/kotlin` -- same shape as
    // SwiftUI, unlike Qt -- so the target is a path there and no build-list
    // entry is needed, matching engram-app's own Compose `[host_effects]`
    // entry exactly.
    let compose_file = package
        .host_effects
        .files
        .iter()
        .find(|file| file.backend == "compose")
        .expect("must declare the Compose handler source file");
    assert_eq!(compose_file.source, "host/compose/PhotoPickerEffects.kt");
    assert_eq!(compose_file.target, "src/main/kotlin/PhotoPickerEffects.kt");

    let compose_handler = package
        .host_effects
        .handlers
        .iter()
        .find(|handler| handler.backend == "compose")
        .expect("must declare the Compose effect handler");
    assert_eq!(compose_handler.install, "installPhotoPickerEffects");
    // No `include` for Compose -- Kotlin has no include directive and the
    // emitter refuses one outright (confirmed against
    // `compose_main_with_host_effects`'s own refusal message).
    assert_eq!(compose_handler.include, None);

    // No other backend is declared yet -- Flutter is an explicit follow-up
    // PR (UI59 §2), not silently half-wired here.
    for backend in ["swiftui", "flutter"] {
        assert!(
            !package.host_effects.files.iter().any(|f| f.backend == backend),
            "{backend} should not have a host_effects file yet"
        );
        assert!(
            !package.host_effects.handlers.iter().any(|h| h.backend == backend),
            "{backend} should not have a host_effects handler yet"
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

#[test]
fn qt_handler_sources_exist_and_declare_install() {
    let header = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host/qt/PhotoPickerEffects.h"),
    )
    .expect("host/qt/PhotoPickerEffects.h must exist");
    assert!(header.contains("void installPhotoPickerEffects(MosaicHost &host)"));

    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host/qt/PhotoPickerEffects.cpp"),
    )
    .expect("host/qt/PhotoPickerEffects.cpp must exist");
    assert!(source.contains("void installPhotoPickerEffects(MosaicHost &host)"));
    assert!(source.contains("effectRequested"));
    assert!(source.contains("\"files.open\""));
}

#[test]
fn compose_handler_source_exists_and_declares_install() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("host/compose/PhotoPickerEffects.kt"),
    )
    .expect("host/compose/PhotoPickerEffects.kt must exist");
    assert!(source.contains("fun installPhotoPickerEffects(host: MosaicRuntimeHost)"));
    assert!(source.contains("effectHandler"));
    assert!(source.contains("\"files.open\""));
}
