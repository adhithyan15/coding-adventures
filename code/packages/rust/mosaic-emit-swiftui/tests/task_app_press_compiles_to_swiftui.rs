//! Complex-app acceptance gate for native MSL pressed-state activation.
//!
//! Task App authors its primary action feedback in Mosaic. The SwiftUI
//! backend must connect that shared `state pressed` block to native gesture
//! state without app-specific AppKit code.

use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

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
fn task_app_pressed_state_lowers_to_native_swiftui_press_state() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let interface = mosmodel_compiler::compile(&mil).expect("compile Task App interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Task App layout");

    for theme in ["TaskApp.light.msl", "TaskApp.dark.msl"] {
        let msl = fs::read_to_string(root.join(theme)).expect("read Task App style");
        let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
            .expect("compile Task App style");
        let output =
            mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
                .expect("emit Task App SwiftUI")
                .output;

        assert!(
            output.contains("private struct _MosaicPressState<Content: View>: View"),
            "{theme} must generate native pressed-state support:\n{output}"
        );
        assert_eq!(
            output
                .matches("_MosaicPressState { __mosaicPressActive in")
                .count(),
            1,
            "{theme} has one authored pressed surface:\n{output}"
        );
        let press_start = output
            .find("_MosaicPressState { __mosaicPressActive in")
            .expect("press wrapper");
        let press_region = &output[press_start..output.len().min(press_start + 2_500)];
        assert!(
            press_region.contains("Button(action: { dispatch(.addTask) })")
                && press_region.contains("Text(\"Add task\")"),
            "the primary action must own the {theme} native press wrapper:\n{press_region}"
        );
        assert!(
            press_region.contains("__mosaicPressActive"),
            "the {theme} pressed background must consume native press state:\n{press_region}"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn native_press_wrapper_typechecks_with_swiftui() {
    let interface = mosmodel_compiler::compile(
        r#"
component PressButton {
  emit onSave ;
}
"#,
    )
    .expect("compile press fixture interface");
    let layout = moslayout_compiler::compile(
        r#"
layout PressButton {
  HostButton [ button ] ( label : "Save" , onClick : emit: onSave )
}
"#,
        Some(&interface.descriptor_json),
    )
    .expect("compile press fixture layout");
    let style = mosstyle_compiler::compile(
        r##"
style PressButton {
  part button {
    background : "#e0942a" ;
    state pressed {
      background : "#a96816" ;
    }
  }
}
"##,
        Some(&layout.part_map_json),
    )
    .expect("compile press fixture style");
    let output = mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit press fixture SwiftUI")
        .output;

    let source_path =
        std::env::temp_dir().join(format!("mosaic-press-{}.swift", std::process::id()));
    fs::write(&source_path, output).expect("write generated Swift fixture");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);

    assert!(
        result.status.success(),
        "generated native press wrapper must typecheck:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
