//! Compile-check for the TaskApp Mosaic package: the interface (.mil), layout (.mll),
//! and style (.msl) must compile, and the manifest must declare the exported component.
//! This is the same shape of smoke test engram-app uses.

use std::fs;
use std::path::Path;
use std::path::PathBuf;

const PAINT_WIDTH: u32 = 1280;
const PAINT_HEIGHT: u32 = 900;

fn packages_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages")
}

fn render_paint_theme(theme: &str) -> Vec<u8> {
    let packages = packages_root();
    let composed = mosaic_package_artifact_builder::compose_component(
        "TaskApp",
        &read("TaskApp.mil"),
        &read("TaskApp.mll"),
        &read(&format!("TaskApp.{theme}.msl")),
        &[packages.clone(), packages.join("mosaic")],
        Some(theme),
    )
    .unwrap_or_else(|e| panic!("TaskApp {theme} package composition failed: {e}"));
    let scene = mosaic_emit_paint::render_scene_from_pipeline_with_sample_slot_values(
        &composed.model.component,
        &composed.layout.def,
        &composed.style,
        f64::from(PAINT_WIDTH),
        f64::from(PAINT_HEIGHT),
    )
    .unwrap_or_else(|e| panic!("TaskApp {theme} Paint scene failed: {e}"));
    barcode_2d::render_scene_png_with_backend(&scene, "skia")
        .unwrap_or_else(|e| panic!("TaskApp {theme} Skia PNG failed: {e}"))
}

fn png_dimensions(png: &[u8]) -> (u32, u32) {
    assert!(png.len() >= 24, "PNG must contain an IHDR chunk");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n", "invalid PNG signature");
    assert_eq!(&png[12..16], b"IHDR", "PNG must start with IHDR");
    (
        u32::from_be_bytes(png[16..20].try_into().unwrap()),
        u32::from_be_bytes(png[20..24].try_into().unwrap()),
    )
}

fn update_golden(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap_or_else(|e| panic!("failed to update {}: {e}", path.display()));
}

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
        // Checklists (C3a, task-app-checklists-view-v1.md).
        "checklists-mode",
        "checklist-library-rows",
        "checklist-library-empty",
        "selected-checklist-key",
        "checklist-template-mode",
        "checklist-run-mode",
        "checklist-outline-rows",
        "checklist-run-rows",
        "checklist-complete-label",
        "checklist-abandon-label",
    ] {
        assert!(slots.contains(&expected), "missing slot: {expected}");
    }
}

/// The package builds, dependencies inlined, on every backend.
///
/// The per-file compiles above cannot see what inlining does: a component
/// mounted twice keeps its part names, so the layout compiles alone yet the
/// package fails with DuplicatePart. This caught a second toolkit EmptyState
/// in the Checklists view (C3a) before any CI lane did.
#[test]
fn builds_on_every_backend() {
    use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
    for backend in [
        Backend::React,
        Backend::WebComponent,
        Backend::Html,
        Backend::SwiftUI,
        Backend::Compose,
        Backend::Flutter,
        Backend::Qt,
        Backend::Xaml,
    ] {
        let out = tempfile::TempDir::new().unwrap();
        build_package(&BuildOptions {
            package_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            output_root: out.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} build failed: {e}"));
    }
}

/// #16151: the package-expanded TaskApp has a reviewed, deterministic Paint
/// image for both product themes. This is an explicit CPU Skia CI fixture; it
/// does not claim that TaskApp ships a native Paint host or Paint artifacts.
#[test]
fn paint_images_match_reviewed_goldens() {
    let light = render_paint_theme("light");
    let dark = render_paint_theme("dark");

    assert_eq!(light, render_paint_theme("light"));
    assert_eq!(dark, render_paint_theme("dark"));
    assert_eq!(png_dimensions(&light), (PAINT_WIDTH, PAINT_HEIGHT));
    assert_eq!(png_dimensions(&dark), (PAINT_WIDTH, PAINT_HEIGHT));
    assert_ne!(light, dark, "light and dark themes must render differently");

    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden");
    if std::env::var("UPDATE_TASK_APP_PAINT_GOLDENS").as_deref() == Ok("1") {
        update_golden(&golden_dir.join("TaskApp.light.png"), &light);
        update_golden(&golden_dir.join("TaskApp.dark.png"), &dark);
        return;
    }

    assert_eq!(
        light.as_slice(),
        include_bytes!("golden/TaskApp.light.png"),
        "light Paint output changed; review it before regenerating with UPDATE_TASK_APP_PAINT_GOLDENS=1"
    );
    assert_eq!(
        dark.as_slice(),
        include_bytes!("golden/TaskApp.dark.png"),
        "dark Paint output changed; review it before regenerating with UPDATE_TASK_APP_PAINT_GOLDENS=1"
    );
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
    assert!(
        acceptance.contains("generatedStartupFailureIsVisibleAndRetryRerunsInitialization"),
        "Compose acceptance must drive the generated startup failure and retry path"
    );
    let flutter_acceptance = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/rust/task-mosaic-app/conformance/flutter/widget_test.dart"),
    )
    .expect("Flutter acceptance source must exist");
    for marker in [
        "forced TaskApp initial-props failure",
        "expect(failingHost.disposed, isTrue)",
        "mosaic-startup-loading",
        "mosaic-startup-failure",
        "mosaic-startup-retry",
        "expect(startupAttempts, 2)",
        "topbar fits the declared desktop viewport",
        "the generated topbar must not report a RenderFlex overflow",
        "timeline legend fits the default constrained viewport",
        "the generated Timeline must not report a RenderFlex overflow",
    ] {
        assert!(
            flutter_acceptance.contains(marker),
            "Flutter acceptance must drive generated startup failure and retry: {marker}"
        );
    }
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

/// #13465: Flutter Rows preserve intrinsic child widths and do not wrap. Keep
/// the descriptive title block above the compact action controls so the topbar
/// fits TaskApp's declared 1280px desktop viewport on every backend.
#[test]
fn topbar_stacks_title_and_controls() {
    let layout = read("TaskApp.mll");
    assert!(layout.contains("Column [ topbar ]"));
    assert!(!layout.contains("Row [ topbar ]"));
    assert!(layout.contains("Row [ topbar-controls ]"));

    for theme in ["light", "dark"] {
        let style = read(&format!("TaskApp.{theme}.msl"));
        let topbar = style
            .split("part topbar {")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("topbar style must exist");
        assert!(topbar.contains("gap : 12 ;"));
        assert!(
            !topbar.contains("align"),
            "a vertical Row alignment has no meaning on the stacked topbar"
        );

        let controls = style
            .split("part topbar-controls {")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("topbar-controls style must exist");
        assert!(controls.contains("gap : 18 ;"));
        assert!(controls.contains("align : center-vertical ;"));
    }
}

/// #16949: compact desktop hosts have only 800 x 600 logical pixels. The
/// view selector stays reachable through a horizontal viewport, task actions
/// reflow below the task identity, and the content viewport consumes the
/// remaining bounded height instead of making the root Column overflow.
#[test]
fn compact_controls_reflow_and_scroll() {
    let layout = read("TaskApp.mll");
    assert!(layout.contains("HostScroll [ view-switch-scroll ] ( axis : horizontal )"));
    assert!(layout.contains("Column [ task-row ]"));
    assert!(layout.contains("Row [ task-identity ]"));
    assert!(layout.contains("HostScroll [ task-actions-scroll ] ( axis : horizontal )"));
    assert!(layout.contains("Row [ task-actions ]"));

    for theme in ["light", "dark"] {
        let style = read(&format!("TaskApp.{theme}.msl"));
        let content_scroll = style
            .split("part content-scroll {")
            .nth(1)
            .and_then(|rest| rest.split('}').next())
            .expect("content-scroll style must exist");
        assert!(content_scroll.contains("flex-grow : 1 ;"));

        for part in ["task-identity", "task-actions"] {
            assert!(
                style.contains(&format!("part {part} {{")),
                "{theme} theme must style compact {part}"
            );
        }
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
