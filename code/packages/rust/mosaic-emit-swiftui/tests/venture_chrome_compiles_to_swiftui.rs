//! Venture browser-chrome acceptance gate for the SwiftUI backend.
//!
//! Venture authors its browser controls once in Mosaic. This test ensures the
//! checked-in package keeps lowering to native SwiftUI controls and a complete
//! macOS project shell without app-specific AppKit chrome.

use std::fs;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::Command;

fn venture_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(|path| {
            path.join("programs")
                .join("mosaic")
                .join("venture-browser")
                .join("src")
        })
        .expect("derive Venture source root from CARGO_MANIFEST_DIR")
}

fn compile_sources(
    theme: &str,
) -> (
    mosmodel_compiler::CompileOutput,
    moslayout_compiler::CompileOutput,
    mosstyle_compiler::CompileOutput,
) {
    let root = venture_src_root();
    let mil = fs::read_to_string(root.join("VentureChrome.mil")).expect("read VentureChrome.mil");
    let mll = fs::read_to_string(root.join("VentureChrome.mll")).expect("read VentureChrome.mll");
    let msl = fs::read_to_string(root.join(theme)).expect("read VentureChrome theme");
    let interface = mosmodel_compiler::compile(&mil).expect("compile Venture interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Venture layout");
    let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
        .expect("compile Venture style");
    (interface, layout, style)
}

#[test]
fn venture_chrome_lowers_to_native_swiftui_controls_and_project_shell() {
    for theme in ["VentureChrome.light.msl", "VentureChrome.dark.msl"] {
        let (interface, layout, style) = compile_sources(theme);
        let options = mosaic_emit_swiftui::pipeline::EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let result = mosaic_emit_swiftui::pipeline::from_pipeline_with_options(
            &interface.component,
            &layout.def,
            &style.def,
            &options,
        )
        .expect("emit Venture SwiftUI project");

        assert!(result
            .output
            .contains("Button(action: { dispatch(.back) })"));
        assert!(result.output.contains(".disabled(backDisabled)"));
        assert!(result.output.contains(".disabled(forwardDisabled)"));
        assert!(result.output.contains(
            "TextField(\"Enter a URL\", text: Binding(get: { address }, set: { dispatch(.addressChange(value: $0)) }))"
        ));
        assert!(result.output.contains(".onSubmit { dispatch(.navigate) }"));
        assert!(
            result.output.matches("contentSurface").count() >= 2,
            "the node slot must be declared and mounted in the SwiftUI body"
        );
        assert!(result
            .output
            .contains("_MosaicPressState { __mosaicPressActive in"));
        assert!(result
            .output
            .contains("_MosaicFocusState { __mosaicFocusActive in"));

        let project = result.project.expect("SwiftUI project shell");
        assert!(project.package_swift.contains(".executableTarget("));
        assert!(project.package_swift.contains("name: \"App\""));
        assert!(project.app_swift.contains("VentureChromeView("));
        assert!(project
            .app_swift
            .contains("contentSurface: AnyView(EmptyView())"));
        assert!(project.app_swift.contains("host.dispatch(event)"));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn venture_chrome_generated_swift_typechecks() {
    let (interface, layout, style) = compile_sources("VentureChrome.light.msl");
    let output = mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit Venture SwiftUI")
        .output;
    let source_path =
        std::env::temp_dir().join(format!("venture-chrome-{}.swift", std::process::id()));
    fs::write(&source_path, output).expect("write generated Venture SwiftUI");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);

    assert!(
        result.status.success(),
        "generated Venture SwiftUI must typecheck:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
