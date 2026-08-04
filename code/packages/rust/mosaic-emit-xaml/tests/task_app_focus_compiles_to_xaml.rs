//! Complex-app acceptance gate for native MSL focus activation.
//!
//! Task App authors its project-composer focus ring in Mosaic. The XAML
//! backend must connect that shared `state focused` block to WinUI focus
//! without app-specific code-behind.

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
fn task_app_focused_state_lowers_to_native_xaml_focus() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let msl = fs::read_to_string(root.join("TaskApp.light.msl")).expect("read TaskApp.light.msl");

    let interface = mosmodel_compiler::compile(&mil).expect("compile Task App interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Task App layout");
    let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
        .expect("compile Task App light style");
    let result = mosaic_emit_xaml::from_pipeline(
        &interface.component,
        &layout.def,
        &style.def,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("emit Task App XAML");

    assert_eq!(
        result
            .xaml
            .matches("<local:FocusStateToBoolConverter x:Key=")
            .count(),
        1,
        "Task App must declare one native focus converter:\n{}",
        result.xaml
    );
    assert_eq!(
        result
            .xaml
            .matches("Binding FocusState, ElementName=ProjectInput")
            .count(),
        1,
        "Task App has one property-scoped focused override:\n{}",
        result.xaml
    );
    assert!(
        result.xaml.contains(
            "<Setter Target=\"ProjectInput.(Control.BorderBrush).(SolidColorBrush.Color)\" Value=\"#e0942a\"/>"
        ),
        "the shared project-input focus ring must target the native TextBox:\n{}",
        result.xaml
    );
    let helper = result
        .if_helpers
        .iter()
        .find(|file| file.filename == "FocusStateToBoolConverter.cs")
        .expect("Task App focus converter helper");
    assert!(
        helper.source.contains("state != FocusState.Unfocused"),
        "native pointer, keyboard, and programmatic focus must activate the shared state:\n{}",
        helper.source
    );
}
