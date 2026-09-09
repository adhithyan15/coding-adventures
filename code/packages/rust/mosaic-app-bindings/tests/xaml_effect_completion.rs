//! The emitted XAML host answers effects, proved by running it.
//!
//! Same argument as the Qt, SwiftUI, Compose and Flutter acceptances beside it:
//! this crate's other tests assert on the *text* of the emitted host, which
//! cannot establish that it compiles, let alone that it behaves. The Compose
//! port of this change shipped three missing imports past every text assertion
//! and failed the first compiler invocation. `Effect` rode the wire and was read
//! by no native host for as long as it existed.
//!
//! The binding is compiled here against the same `Windows.UI.Color` value stub
//! the conformance harness uses, so it builds on a plain console target without
//! the Windows App SDK. The full WinUI compile happens in the TaskApp CI lane;
//! what this proves is the FFI and effect behaviour, on any platform.

use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("mosaic-app-bindings sits inside the rust workspace")
        .to_path_buf()
}

fn dotnet_available() -> bool {
    Command::new("dotnet")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn cargo() -> PathBuf {
    std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

/// The conformance runtime, as a shared library the host loads through P/Invoke.
fn conformance_runtime() -> PathBuf {
    let root = workspace_root();
    let status = Command::new(cargo())
        .current_dir(&root)
        .args(["build", "-p", "mosaic-app-conformance"])
        .status()
        .expect("build the conformance runtime");
    assert!(status.success(), "conformance runtime failed to build");
    let stem = root.join("target").join("debug");
    for name in [
        "libmosaic_app_conformance.dylib",
        "libmosaic_app_conformance.so",
        "mosaic_app_conformance.dll",
    ] {
        let candidate = stem.join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("missing conformance runtime under {stem:?}");
}

fn run(command: &mut Command, what: &str) -> String {
    let output = command
        .output()
        .unwrap_or_else(|err| panic!("{what}: {err}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "{what} failed ({});\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        output.status
    );
    stdout
}

#[test]
fn the_emitted_xaml_host_answers_effects() {
    if !dotnet_available() {
        eprintln!("skipping XAML effect acceptance: dotnet unavailable");
        return;
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project = std::env::temp_dir().join(format!(
        "mosaic-xaml-effects-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir(&project).expect("create the driver project");

    // The host under test, exactly as a real package would receive it. The
    // namespace matches the driver's so the emitted static class is in scope.
    let host = mosaic_app_bindings::xaml_runtime_binding_for_application(
        "Mosaic.Generated",
        "dev.mosaic.xaml-effects",
    );
    std::fs::write(project.join("MosaicRuntimeHost.cs"), &host).unwrap();
    std::fs::copy(
        manifest
            .join("tests")
            .join("xaml_effect_driver")
            .join("Driver.cs"),
        project.join("Driver.cs"),
    )
    .expect("copy the driver");
    // The same stub the conformance harness uses, not a second copy of it.
    std::fs::copy(
        manifest
            .join("conformance")
            .join("xaml")
            .join("WindowsColorStub.cs"),
        project.join("WindowsColorStub.cs"),
    )
    .expect("copy the Windows.UI.Color stub");
    // `ImportDirectoryBuild*` off: MSBuild otherwise walks UPWARD from the
    // project for `Directory.Build.props`, and this project lives under the
    // system temp directory, which on a shared host any local user can write.
    // That would import their targets into this build.
    std::fs::write(
        project.join("XamlEffectDriver.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n\
         \x20 <PropertyGroup>\n\
         \x20   <OutputType>Exe</OutputType>\n\
         \x20   <TargetFramework>net9.0</TargetFramework>\n\
         \x20   <ImplicitUsings>enable</ImplicitUsings>\n\
         \x20   <Nullable>enable</Nullable>\n\
         \x20   <ImportDirectoryBuildProps>false</ImportDirectoryBuildProps>\n\
         \x20   <ImportDirectoryBuildTargets>false</ImportDirectoryBuildTargets>\n\
         \x20 </PropertyGroup>\n\
         </Project>\n",
    )
    .unwrap();

    run(
        Command::new("dotnet")
            .current_dir(&project)
            .args(["build", "-v", "quiet", "--nologo"]),
        "dotnet build",
    );

    let runtime = conformance_runtime();

    // Each case gets its own process AND its own state file: the host reads
    // MOSAIC_APP_STATE_PATH once at load, so cases sharing one file would let
    // one restore another's count, making an assertion read a number its own
    // host never produced.
    let mut transcript = String::new();
    for case in [
        "unhandled",
        "answered",
        "batch-both",
        "batch-mixed",
        "throwing",
        "runaway",
        "closes",
        "deferred",
    ] {
        let stdout = run(
            Command::new("dotnet")
                .current_dir(&project)
                .args(["run", "--no-build"])
                .env("MOSAIC_APP_LIBRARY", &runtime)
                .env("MOSAIC_PROBE_CASE", case)
                .env(
                    "MOSAIC_APP_STATE_PATH",
                    project.join(format!("{case}.json")),
                ),
            &format!("xaml effect driver ({case})"),
        );
        assert!(
            stdout.contains("case passed"),
            "case `{case}` did not report success:\n{stdout}"
        );
        transcript.push_str(&stdout);
    }

    // A driver that printed nothing would satisfy the line above by accident.
    for expected in [
        "a fresh app awaits nothing",
        "an unanswered await is failed, not dropped",
        "the app is told why, rather than just waiting",
        "a failed completion does not advance the app",
        "an answered await is settled",
        "the handler's value reached the app",
        "the handler answered both effects of the batch",
        "a fully-answered chaining batch leaves nothing outstanding",
        "state still persists after a fully-answered chaining batch",
        "a partly-answered batch leaves nothing outstanding",
        "state still persists after a partly-answered batch",
        "a handler that throws does not leave the effect pending",
        "the app is told the handler failed, and why",
        "state still persists after a handler that threw",
        "a runaway chain returns instead of spinning forever",
        "a runaway chain is reported rather than abandoned quietly",
        "a handler that closes the host does not kill the process",
        "a closed host refuses further props rather than serving stale ones",
        "deferring an effect nothing awaits is refused",
        "the handler was offered the effect",
        "a deferred effect stays outstanding rather than being failed",
        "state cannot be persisted while a deferred effect is outstanding",
        "answering from another thread does not deadlock",
        "answering a deferred effect settles it",
        "the deferred answer's value reached the app",
        "state persists again once the deferred effect is answered",
    ] {
        assert!(
            transcript.contains(expected),
            "missing check `{expected}`:\n{transcript}"
        );
    }
    assert!(
        !transcript.contains("VACUOUS"),
        "a check ran vacuously:\n{transcript}"
    );

    let _ = std::fs::remove_dir_all(&project);
}
