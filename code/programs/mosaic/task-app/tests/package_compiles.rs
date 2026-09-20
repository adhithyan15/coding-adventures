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
    let slots: Vec<&str> = mil
        .component
        .slots
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    for expected in [
        "app-title",
        "new-task-name",
        "new-task-due",
        "new-task-name-error",
        "new-task-due-error",
        "new-task-name-focus",
        "new-task-due-focus",
        "summary",
        "task-rows",
    ] {
        assert!(slots.contains(&expected), "missing slot: {expected}");
    }
}

#[test]
fn manifest_declares_task_app() {
    let manifest_src =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"))
            .expect("mosaic-package.toml must exist");
    let package = mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse");
    assert_eq!(package.package.name, "task-app");
    assert_eq!(package.components.exports, ["TaskApp"]);
    let window = package
        .app
        .initial_window_size
        .expect("TaskApp must declare the desktop window used by acceptance tests");
    assert_eq!((window.width, window.height), (1280, 900));

    let acceptance = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/rust/task-mosaic-app/conformance/compose/TaskAppUiTest.kt"),
    )
    .expect("Compose acceptance source must exist");
    assert!(
        acceptance.contains(&format!("Size({}f, {}f)", window.width, window.height)),
        "Compose acceptance viewport must match [app] initial window size"
    );
}

/// #15263: the persistence status and long local-data path must not compete
/// for one horizontal line. Compose deliberately preserves intrinsic Row-child
/// widths, so the old Row painted the two strings over each other at 1280px.
#[test]
fn storage_summary_stacks_status_and_location() {
    let layout = read("TaskApp.mll");
    assert!(layout.contains("Column [ storage-summary ]"));
    assert!(!layout.contains("Row [ storage-summary ]"));

    for theme in ["light", "dark"] {
        let style = read(&format!("TaskApp.{theme}.msl"));
        let summary = style
            .split("part storage-summary {")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("storage-summary style must exist");
        assert!(summary.contains("gap : 4 ;"));
        assert!(
            !summary.contains("align"),
            "a vertical Row alignment has no meaning on the stacked summary"
        );
    }
}

/// #15486: TaskApp delegates narrow-window adaptation and the project-pane
/// landmark to the kernel primitive instead of freezing a two-column Row.
#[test]
fn shell_is_the_adaptive_navigation_split() {
    let layout = read("TaskApp.mll");
    assert!(layout.contains("HostNavigationSplit [ app-shell ]"));
    assert!(layout.contains("pane-title : \"Projects\""));
    assert!(layout.contains("pane-width : 236"));
    assert!(layout.contains("collapse : auto"));

    let mil = mosmodel_compiler::compile(&read("TaskApp.mil")).expect("TaskApp.mil should compile");
    moslayout_compiler::compile(&layout, Some(&mil.descriptor_json))
        .expect("the adaptive TaskApp shell should compile");
}

/// #14016: the view switcher is one toolkit SegmentedControl, not a six-way
/// If/Else around 36 hand-styled `seg-*` parts. Pinned so the inline copy
/// cannot come back piecemeal.
#[test]
fn view_switcher_is_the_toolkit_segmented_control() {
    let mil = mosmodel_compiler::compile(&read("TaskApp.mil")).expect("TaskApp.mil should compile");
    let slots: Vec<&str> = mil
        .component
        .slots
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(slots.contains(&"nav-options"));
    assert!(slots.contains(&"nav-selected-index"));
    let show_view = mil
        .component
        .emits
        .iter()
        .find(|e| e.name == "onShowView")
        .expect("onShowView must be declared");
    assert_eq!(
        show_view.params.len(),
        1,
        "onShowView carries the option index"
    );

    let layout = read("TaskApp.mll");
    assert!(layout.contains("pkg::mosaic-pkg-toolkit::SegmentedControl"));
    assert!(layout.contains("onSelect : emit: onShowView"));
    let mll = moslayout_compiler::compile(&layout, Some(&mil.descriptor_json)).unwrap();
    let stale: Vec<&str> = mll
        .parts
        .iter()
        .map(|p| p.name.as_str())
        .filter(|name| *name == "seg" || name.starts_with("seg-"))
        .collect();
    assert!(
        stale.is_empty(),
        "inline switcher parts are back: {stale:?}"
    );
    for theme in ["light", "dark"] {
        let style = read(&format!("TaskApp.{theme}.msl"));
        assert!(
            !style.contains("part seg"),
            "TaskApp.{theme}.msl still styles the removed seg parts"
        );
    }

    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"))
            .unwrap();
    let package = mosaic_package_manifest::parse(&manifest).unwrap();
    assert!(
        package.dependencies.contains_key("mosaic-pkg-toolkit"),
        "the toolkit must be a declared dependency"
    );
}
