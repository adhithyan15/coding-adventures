//! Reusable ProjectNav acceptance gate for native MSL focus activation.
//!
//! Trestle's ProjectNav package authors its project composer focus ring in
//! Mosaic. The SwiftUI backend must connect that shared `state focused` block
//! to native focus without app-specific AppKit code. Trestle itself is also
//! compiled below to pin its package-expanded multiline Notes editor.

use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

fn code_packages_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../code/packages/rust/mosaic-emit-swiftui — two
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

fn project_nav_src_root() -> PathBuf {
    code_packages_root()
        .join("mosaic")
        .join("mosaic-pkg-project-nav")
        .join("src")
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

fn emit_complete_task_app() -> String {
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
    mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Task App SwiftUI")
        .output
}

#[test]
fn project_nav_focused_state_lowers_to_native_swiftui_focus() {
    let root = project_nav_src_root();
    let mil = fs::read_to_string(root.join("ProjectNav.mil")).expect("read ProjectNav.mil");
    let mll = fs::read_to_string(root.join("ProjectNav.mll")).expect("read ProjectNav.mll");
    let msl =
        fs::read_to_string(root.join("ProjectNav.light.msl")).expect("read ProjectNav.light.msl");

    let interface = mosmodel_compiler::compile(&mil).expect("compile ProjectNav interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile ProjectNav layout");
    let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
        .expect("compile ProjectNav light style");
    let output = mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit ProjectNav SwiftUI")
        .output;

    assert!(
        output.contains("private struct _MosaicFocusState: View"),
        "ProjectNav must generate native focus support:\n{output}"
    );
    assert_eq!(
        output
            .matches("_MosaicFocusState(content: _mosaicFocusContent)")
            .count(),
        1,
        "ProjectNav has one authored focused surface:\n{output}"
    );
    assert!(
        output.contains(".focused($isFocused)"),
        "focus state must bind to SwiftUI's native focus system:\n{output}"
    );

    let focus_start = output
        .find("let _mosaicFocusContent: (Bool) -> AnyView = { __mosaicFocusActive in")
        .expect("focus wrapper");
    let focus_region = &output[focus_start..output.len().min(focus_start + 2_000)];
    assert!(
        focus_region.contains("TextField(\"New project\""),
        "the project composer input must own the native focus wrapper:\n{focus_region}"
    );
    assert!(
        focus_region.contains("__mosaicFocusActive"),
        "the authored focus border must consume native focus state:\n{focus_region}"
    );
}

#[test]
fn task_app_package_graph_lowers_notes_to_native_multiline_swiftui() {
    let output = emit_complete_task_app();

    assert!(
        output.contains("TextEditor(text: Binding(get: { noteBodyValue }"),
        "Trestle's Notes body must use a native multiline editor:\n{output}"
    );
    assert!(
        output.contains(".accessibilityIdentifier(\"notes-body-input\")"),
        "the authored Notes body identity must reach SwiftUI:\n{output}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn complete_task_app_generated_swift_typechecks() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let project_root =
        std::env::temp_dir().join(format!("mosaic-task-app-{}-{unique}", std::process::id()));
    let sources = project_root.join("Sources").join("App");
    fs::create_dir_all(&sources).expect("create temporary SwiftPM source tree");
    fs::write(
        project_root.join("Package.swift"),
        r#"// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "MosaicTaskAppCompileGate",
    platforms: [.macOS(.v13)],
    targets: [.executableTarget(name: "App")]
)
"#,
    )
    .expect("write temporary Package.swift");
    fs::write(sources.join("TaskApp.swift"), emit_complete_task_app())
        .expect("write generated Task App SwiftUI");
    fs::write(
        sources.join("App.swift"),
        r#"import SwiftUI

@main
struct CompileGateApp: App {
    var body: some Scene {
        WindowGroup { EmptyView() }
    }
}
"#,
    )
    .expect("write temporary SwiftUI app shell");

    let result = Command::new("swift")
        .arg("build")
        .arg("--jobs")
        .arg("2")
        .current_dir(&project_root)
        .output()
        .expect("run swift build");
    let _ = fs::remove_dir_all(&project_root);

    assert!(
        result.status.success(),
        "the complete generated Task App must build through SwiftPM:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_focus_wrapper_typechecks_with_swiftui() {
    let interface = mosmodel_compiler::compile(
        r#"
component FocusField {
  slot value : text ;
  emit onChange ( value : text ) ;
}
"#,
    )
    .expect("compile focus fixture interface");
    let layout = moslayout_compiler::compile(
        r#"
layout FocusField {
  HostInput [ field ] (
    value : slot: value ,
    placeholder : "Search" ,
    onChange : emit: onChange
  )
}
"#,
        Some(&interface.descriptor_json),
    )
    .expect("compile focus fixture layout");
    let style = mosstyle_compiler::compile(
        r##"
style FocusField {
  part field {
    border-width : 1 ;
    border-color : "#d0d0d0" ;
    state focused {
      border-color : "#e0942a" ;
    }
  }
}
"##,
        Some(&layout.part_map_json),
    )
    .expect("compile focus fixture style");
    let output = mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit focus fixture SwiftUI")
        .output;

    let source_path =
        std::env::temp_dir().join(format!("mosaic-focus-{}.swift", std::process::id()));
    fs::write(&source_path, output).expect("write generated Swift fixture");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);

    assert!(
        result.status.success(),
        "generated native focus wrapper must typecheck:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn native_multiline_input_typechecks_with_swiftui() {
    let interface = mosmodel_compiler::compile(
        r#"
component NotesEditor {
  slot value : text ;
  emit onBodyChange ( value : text ) ;
}
"#,
    )
    .expect("compile multiline fixture interface");
    let layout = moslayout_compiler::compile(
        r#"
layout NotesEditor {
  Input [ body ] (
    value : slot: value ,
    placeholder : "Write something…" ,
    multiline : true ,
    max-length : 2000 ,
    onChange : emit: onBodyChange
  )
}
"#,
        Some(&interface.descriptor_json),
    )
    .expect("compile multiline fixture layout");
    let style = mosstyle_compiler::compile(
        "style NotesEditor { part body { min-height: 200; } }",
        Some(&layout.part_map_json),
    )
    .expect("compile multiline fixture style");
    let output = mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit multiline fixture SwiftUI")
        .output;

    let source_path =
        std::env::temp_dir().join(format!("mosaic-multiline-{}.swift", std::process::id()));
    fs::write(&source_path, output).expect("write generated multiline fixture");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);

    assert!(
        result.status.success(),
        "generated native multiline editor must typecheck:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
