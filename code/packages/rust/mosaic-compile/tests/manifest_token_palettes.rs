use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("mosaic-compile lives at code/packages/rust/mosaic-compile")
        .to_path_buf()
}

fn temporary_directory() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-manifest-palettes-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

#[test]
fn browser_backends_load_foundation_and_dependent_control_palettes() {
    let repository = repository_root();
    let package_collection = repository.join("code/packages/mosaic");
    let output_directory = temporary_directory();
    let cases = [
        ("mosaic-std-foundation", "BodyText"),
        ("mosaic-std-controls", "Button"),
    ];

    for (package, component) in cases {
        let package_root = package_collection.join(package);
        let source_root = package_root.join("src");
        for (backend, extension) in [("html", "html"), ("webcomponent", "js"), ("react", "tsx")] {
            let output = output_directory.join(format!("{component}-{backend}.{extension}"));
            let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
                .args([
                    "--backend",
                    backend,
                    "--interface",
                    source_root
                        .join(format!("{component}.mil"))
                        .to_str()
                        .unwrap(),
                    "--layout",
                    source_root
                        .join(format!("{component}.mll"))
                        .to_str()
                        .unwrap(),
                    "--style",
                    source_root
                        .join(format!("{component}.light.msl"))
                        .to_str()
                        .unwrap(),
                    "--package-manifest",
                    package_root.join("mosaic-package.toml").to_str().unwrap(),
                    "--package-search-path",
                    package_collection.to_str().unwrap(),
                    "--output",
                    output.to_str().unwrap(),
                ])
                .output()
                .expect("run mosaic-compile");
            assert!(
                result.status.success(),
                "{component} failed on {backend}:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );

            let artifact = fs::read_to_string(&output).expect("read emitted artifact");
            assert!(
                artifact.contains("16px") && !artifact.contains("$foundation-"),
                "{component} did not resolve its manifest palette on {backend}:\n{artifact}"
            );
        }
    }

    fs::remove_dir_all(output_directory).ok();
}

#[test]
fn slider_reaches_the_separately_tracked_host_slider_gate() {
    // #14635 owns the remaining browser HostSlider lowering. Until that lands,
    // keep proving Slider gets past its manifest palette instead of regressing
    // to the earlier unresolved-token failure.
    let repository = repository_root();
    let package_collection = repository.join("code/packages/mosaic");
    let package_root = package_collection.join("mosaic-std-controls");
    let source_root = package_root.join("src");
    let output_directory = temporary_directory();

    for backend in ["html", "webcomponent", "react"] {
        let output = output_directory.join(format!("Slider-{backend}.out"));
        let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
            .args([
                "--backend",
                backend,
                "--interface",
                source_root.join("Slider.mil").to_str().unwrap(),
                "--layout",
                source_root.join("Slider.mll").to_str().unwrap(),
                "--style",
                source_root.join("Slider.light.msl").to_str().unwrap(),
                "--package-manifest",
                package_root.join("mosaic-package.toml").to_str().unwrap(),
                "--package-search-path",
                package_collection.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .expect("run mosaic-compile");
        assert!(
            !result.status.success(),
            "{backend} unexpectedly accepted HostSlider"
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("HostSlider") && !stderr.contains("Token '$foundation-"),
            "Slider did not advance past manifest palette loading on {backend}:\n{stderr}"
        );
    }

    fs::remove_dir_all(output_directory).ok();
}
