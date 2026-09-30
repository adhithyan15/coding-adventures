//! The XAML platform library behaves as the Compose one does, proved by
//! running it (UI87 §7.6).
//!
//! The text assertions in `src/lib.rs` pin the library's tables and messages
//! to the Compose library's; they cannot establish that the C# compiles, let
//! alone that it behaves. This builds the headless conformance harness in
//! `conformance/xaml-platform-effects/` against the library and the runtime
//! host exactly as `xaml_platform_effects` and `xaml_runtime_binding` emit
//! them, and runs it: every open, save, refusal and routing case, with a fake
//! picker and a fake host. The harness defines `MOSAIC_HEADLESS_TEST`, so the
//! WinUI pickers stay out of the build; they are compiled for real in the
//! TaskApp CI lane, which also runs this harness on Windows.

use std::path::PathBuf;
use std::process::Command;

fn dotnet_available() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[test]
fn the_xaml_platform_library_passes_its_headless_conformance() {
    if !dotnet_available() {
        eprintln!("skipping XAML platform-library conformance: dotnet unavailable");
        return;
    }

    let harness = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("conformance")
        .join("xaml-platform-effects");
    let project = std::env::temp_dir().join(format!(
        "mosaic-xaml-platform-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&project).expect("create the harness project");

    // The committed harness, not a second copy of it.
    for file in [
        "Program.cs",
        "WindowsColorStub.cs",
        "XamlPlatformEffectsConformance.csproj",
    ] {
        std::fs::copy(harness.join(file), project.join(file))
            .unwrap_or_else(|err| panic!("copy {file}: {err}"));
    }
    // The library and the host under test, as a generated project gets them.
    std::fs::write(
        project.join("MosaicPlatformEffects.cs"),
        mosaic_app_bindings::xaml_platform_effects("Mosaic.Generated"),
    )
    .unwrap();
    std::fs::write(
        project.join("MosaicRuntimeHost.cs"),
        mosaic_app_bindings::xaml_runtime_binding("Mosaic.Generated"),
    )
    .unwrap();
    // Stop MSBuild's upward walk at this project: it lives under the system
    // temp directory, where any local user could plant a Directory.Build.props
    // (see xaml_effect_completion.rs for why a project property cannot).
    for sentinel in ["Directory.Build.props", "Directory.Build.targets"] {
        std::fs::write(project.join(sentinel), "<Project />\n").unwrap();
    }

    let output = Command::new("dotnet")
        .current_dir(&project)
        .args([
            "run",
            "-c",
            "Release",
            "--project",
            "XamlPlatformEffectsConformance.csproj",
        ])
        .output()
        .expect("run dotnet");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&project);
    assert!(
        output.status.success(),
        "the XAML platform-library harness failed ({});\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("Mosaic XAML platform effects conformance passed"),
        "{stdout}"
    );
}
