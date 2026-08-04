//! Compile-check for the TaskApp Mosaic package: the interface (.mil), layout (.mll),
//! and style (.msl) must compile, and the manifest must declare the exported component.
//! This is the same shape of smoke test engram-app uses.

use std::fs;
use std::path::PathBuf;

fn read(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn task_app_sources_compile() {
    let mil = mosmodel_compiler::compile(&read("TaskApp.mil")).expect("TaskApp.mil should compile");
    let mll = moslayout_compiler::compile(&read("TaskApp.mll"), Some(&mil.descriptor_json))
        .expect("TaskApp.mll should compile against the interface");
    let light = mosstyle_compiler::compile(&read("TaskApp.light.msl"), Some(&mll.part_map_json))
        .expect("TaskApp.light.msl should compile against the layout parts");
    // Both themes are authored and must compile against the same layout parts;
    // `mosaic-compile --theme dark` resolves TaskApp.dark.msl.
    let dark = mosstyle_compiler::compile(&read("TaskApp.dark.msl"), Some(&mll.part_map_json))
        .expect("TaskApp.dark.msl should compile against the layout parts");

    assert_eq!(mil.component.component, "TaskApp");
    assert_eq!(mll.def.component_name, "TaskApp");
    assert_eq!(light.def.component_name, "TaskApp");
    assert_eq!(dark.def.component_name, "TaskApp");

    // The interface exposes exactly the slots the web host fills.
    let slots: Vec<&str> = mil.component.slots.iter().map(|s| s.name.as_str()).collect();
    for expected in [
        "app-title",
        "new-task-name",
        "new-task-due",
        "summary",
        "task-rows",
    ] {
        assert!(slots.contains(&expected), "missing slot: {expected}");
    }
}

#[test]
fn manifest_declares_task_app() {
    let manifest_src = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"),
    )
    .expect("mosaic-package.toml must exist");
    let package = mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse");
    assert_eq!(package.package.name, "task-app");
    assert_eq!(package.components.exports, ["TaskApp"]);
}
