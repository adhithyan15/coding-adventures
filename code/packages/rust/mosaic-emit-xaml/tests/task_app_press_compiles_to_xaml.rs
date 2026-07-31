//! Complex-app acceptance gate for native MSL pressed-state activation.
//!
//! Task App authors its primary action feedback in Mosaic. The XAML backend
//! must connect that shared `state pressed` block to WinUI ButtonBase state
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
fn task_app_pressed_state_lowers_to_native_xaml_button_state() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let interface = mosmodel_compiler::compile(&mil).expect("compile Task App interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Task App layout");

    for (theme, pressed_color) in [
        ("TaskApp.light.msl", "#a96816"),
        ("TaskApp.dark.msl", "#b87822"),
    ] {
        let msl = fs::read_to_string(root.join(theme)).expect("read Task App style");
        let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
            .expect("compile Task App style");
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
                .matches("Binding IsPressed, ElementName=AddBtn")
                .count(),
            1,
            "{theme} has one property-scoped pressed override:\n{}",
            result.xaml
        );
        assert!(
            result.xaml.contains(&format!(
                "<Setter Target=\"AddBtn.(Control.Background).(SolidColorBrush.Color)\" Value=\"{pressed_color}\"/>"
            )),
            "the {theme} pressed background must target the native Button:\n{}",
            result.xaml
        );
    }
}
