//! Complex-app acceptance gate for native MSL focus activation.
//!
//! Task App authors its project-composer focus ring in Mosaic. The XAML
//! backend must connect that shared `state focused` block to WinUI focus
//! without app-specific code-behind.

use std::fs;
use std::path::PathBuf;

fn code_packages_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../code/packages/rust/mosaic-emit-xaml — two
    // parents up is .../code/packages, the same default search root
    // mosaic-compile's CLI uses.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from)
        .expect("derive code/packages root from CARGO_MANIFEST_DIR")
}

fn task_app_src_root() -> PathBuf {
    code_packages_root()
        .parent()
        .map(|path| {
            path.join("programs")
                .join("mosaic")
                .join("task-app")
                .join("src")
        })
        .expect("derive Task App source root from CARGO_MANIFEST_DIR")
}

/// Resolve every `pkg::P::C` reference in `layout` in place — the same step
/// `mosaic-compile`'s CLI runs before handing a layout to any backend
/// emitter (UI34 §5). Every emitter's entry point assumes this has already
/// happened; skipping it surfaces as `UnsupportedPrimitive("pkg::...")`.
fn resolve_packages(layout: &mut moslayout_compiler::LayoutDef) {
    let packages_root = code_packages_root();
    let search_paths = vec![packages_root.clone(), packages_root.join("mosaic")];
    mosaic_package_resolver::LayoutPackageResolver::new(search_paths)
        .resolve(layout)
        .expect("resolve pkg:: references in TaskApp.mll");
}

#[test]
fn task_app_focused_state_lowers_to_native_xaml_focus() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let msl = fs::read_to_string(root.join("TaskApp.light.msl")).expect("read TaskApp.light.msl");

    let interface = mosmodel_compiler::compile(&mil).expect("compile Task App interface");
    let mut layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Task App layout");
    resolve_packages(&mut layout.def);
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
