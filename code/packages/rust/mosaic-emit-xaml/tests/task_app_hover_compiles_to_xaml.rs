//! Complex-app acceptance gate for native MSL hover activation.
//!
//! Task App deliberately authors all interaction styling in Mosaic. Its light
//! theme exercises nineteen independently named HostButton hover surfaces,
//! including controls inside repeated project and task DataTemplates. (The
//! board view's HostDraggable card hover, and the sheet view's Grid/Select/
//! toolbar hover states from mosaic-pkg-sheet/mosaic-pkg-grid/mosaic-pkg-
//! toolkit's OWN stylesheets, are authored in Mosaic too, but this test only
//! compiles TaskApp's own TaskApp.light.msl — a package's stylesheet is a
//! separate compile unit the real build pipeline merges in, not something
//! this acceptance test exercises. Likewise the XAML backend doesn't lower
//! HostDraggable pointer states yet — native shells for the board are a
//! later roadmap phase — so it isn't counted here either.)

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
fn task_app_hover_states_lower_to_native_row_local_xaml() {
    let root = task_app_src_root();
    let mil = fs::read_to_string(root.join("TaskApp.mil")).expect("read TaskApp.mil");
    let mll = fs::read_to_string(root.join("TaskApp.mll")).expect("read TaskApp.mll");
    let msl = fs::read_to_string(root.join("TaskApp.light.msl")).expect("read TaskApp.light.msl");

    let interface = mosmodel_compiler::compile(&mil).expect("compile TaskApp interface");
    let mut layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile TaskApp layout");
    resolve_packages(&mut layout.def);
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
        22,
        "each property-scoped hover state must use native pointer state:\n{output}"
    );
    for target in [
        "ProjectOff",
        "ProjectAdd",
        "ProjectSub",
        "SegListOff",
        "SegListOff2",
        "SegListOff3",
        "SegBoardOff",
        "SegBoardOff3",
        "SegBoardOff4",
        "SegSheetOff",
        "SegSheetOff2",
        "SegSheetOff3",
        "SegTlOff",
        "SegTlOff2",
        "SegTlOff3",
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
