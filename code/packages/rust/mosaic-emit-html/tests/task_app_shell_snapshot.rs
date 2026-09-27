//! Product-scoped static HTML acceptance for TaskApp/Trestle.
//!
//! Emitter-local fixtures prove individual lowerings, but they cannot prove
//! that TaskApp's real package-expanded shell still reaches this backend.
//! Static HTML deliberately makes no interaction claim: this gate pins the
//! authored List-first structure that a design review or downstream hydrator
//! must still be able to see.

use std::fs;
use std::path::PathBuf;

fn code_packages_root() -> PathBuf {
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
        .expect("derive TaskApp source root from CARGO_MANIFEST_DIR")
}

fn resolve_packages(layout: &mut moslayout_compiler::LayoutDef) {
    let packages_root = code_packages_root();
    let search_paths = vec![packages_root.clone(), packages_root.join("mosaic")];
    mosaic_package_resolver::LayoutPackageResolver::new(search_paths)
        .resolve(layout)
        .expect("resolve package-qualified components in TaskApp.mll");
}

fn emit_task_app_snapshot() -> String {
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

    mosaic_emit_html::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit TaskApp static HTML")
        .output
}

#[test]
fn task_app_list_first_shell_survives_static_html_lowering() {
    let output = emit_task_app_snapshot();

    for required in [
        "data-mosaic-component=\"TaskApp\"",
        "placeholder=\"What needs doing?\"",
        ">Add task</button>",
        "mosaic-for each=\"taskRows\" as=\"row\" index=\"i\"",
        "mosaic-for each=\"navOptions\" as=\"option\" index=\"i\"",
        "aria-pressed=\"true\"",
        "aria-pressed=\"false\"",
    ] {
        assert!(
            output.contains(required),
            "TaskApp static HTML lost `{required}`:\n{output}"
        );
    }

    assert!(
        output
            .matches("<!-- emit \"onAddTask\" dropped: HTML is static -->")
            .count()
            >= 1,
        "the static snapshot must state that task creation is non-interactive:\n{output}"
    );
    assert!(
        output
            .matches("<!-- emit \"onShowView\" dropped: HTML is static -->")
            .count()
            >= 1,
        "the static snapshot must state that view selection is non-interactive:\n{output}"
    );
}
