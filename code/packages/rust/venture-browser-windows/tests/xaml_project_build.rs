#![cfg(target_os = "windows")]

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn venture_package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(|code| code.join("programs").join("mosaic").join("venture-browser"))
        .expect("derive Venture package root")
}

fn temporary_output() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("venture-xaml-{}-{nonce}", std::process::id()))
}

#[test]
fn package_owned_xaml_host_compiles_in_generated_winui_project() {
    let output = temporary_output();
    let result = build_package(&BuildOptions {
        package_root: venture_package_root(),
        output_root: output.clone(),
        backend: Backend::Xaml,
        emit_project: true,
        theme: Some("light".to_string()),
    })
    .expect("emit Venture XAML package");
    let project = output.join("xaml");
    let host = project.join("MosaicHost.cs");
    assert!(host.exists(), "package-owned XAML host must be installed");
    assert!(
        result.artifacts.iter().any(|artifact| artifact == &host),
        "installed host must be a package artifact"
    );

    let build = Command::new("dotnet")
        .arg("build")
        .arg("VentureChrome.csproj")
        .arg("-p:Platform=x64")
        .current_dir(&project)
        .output()
        .expect("run dotnet build for generated WinUI project");
    if !build.status.success() {
        panic!(
            "generated Venture WinUI project failed to build\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
    }

    fs::remove_dir_all(&output).expect("remove clean acceptance output");
}
