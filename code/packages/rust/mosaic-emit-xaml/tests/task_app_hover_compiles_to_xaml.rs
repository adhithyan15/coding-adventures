//! Complex-app acceptance gate for native MSL hover activation.
//!
//! Task App deliberately authors all interaction styling in Mosaic. Its light
//! theme exercises nine independently named HostButton hover surfaces,
//! including controls inside repeated project and task DataTemplates.

use std::fs;
use std::path::PathBuf;

fn task_app_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(|path| {
            path.join("programs")
                .join("mosaic")
                .join("task-app")
                .join("src")
        })
        .expect("derive Task App source root from CARGO_MANIFEST_DIR")
}

#[test]
fn task_app_hover_states_lower_to_native_row_local_xaml() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let msl = fs::read_to_string(root.join("TaskApp.light.msl")).expect("read TaskApp.light.msl");

    let interface = mosmodel_compiler::compile(&mil).expect("compile TaskApp interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile TaskApp layout");
    let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
        .expect("compile TaskApp light style");
    let output = mosaic_emit_xaml::from_pipeline(
        &interface.component,
        &layout.def,
        &style.def,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("emit TaskApp XAML")
    .xaml;

    assert_eq!(
        output.matches("Binding IsPointerOver").count(),
        12,
        "each property-scoped hover state must use native pointer state:\n{output}"
    );
    for target in [
        "ProjectOff",
        "ProjectAdd",
        "ProjectSub",
        "SegListOff",
        "SegTlOff",
        "AddBtn",
        "Toggle",
        "TaskName",
        "DelBtn",
    ] {
        assert!(
            output.contains(&format!("ElementName={target}")),
            "missing native hover target {target}:\n{output}"
        );
    }
}
