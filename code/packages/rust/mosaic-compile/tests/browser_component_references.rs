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

fn temporary_directory(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "mosaic-compile-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}

#[test]
fn browser_backends_inline_grid_cell_and_accept_notes_input() {
    let repository = repository_root();
    let package_collection = repository.join("code/packages/mosaic");
    let output_directory = temporary_directory("browser-component-references");
    let cases = [
        ("mosaic-pkg-grid", "Grid", Some("Grid.dark.msl")),
        (
            "mosaic-pkg-grid",
            "RowHeaderGrid",
            Some("RowHeaderGrid.dark.msl"),
        ),
        ("mosaic-pkg-notes", "Notes", None),
    ];

    for (package, component, style) in cases {
        let package_root = package_collection.join(package);
        let source_root = package_root.join("src");
        for (backend, extension) in [("html", "html"), ("webcomponent", "js"), ("react", "tsx")] {
            let output = output_directory.join(format!("{component}-{backend}.{extension}"));
            let mut args = vec![
                "--backend".to_string(),
                backend.to_string(),
                "--interface".to_string(),
                source_root
                    .join(format!("{component}.mil"))
                    .to_string_lossy()
                    .into_owned(),
                "--layout".to_string(),
                source_root
                    .join(format!("{component}.mll"))
                    .to_string_lossy()
                    .into_owned(),
                "--package-manifest".to_string(),
                package_root
                    .join("mosaic-package.toml")
                    .to_string_lossy()
                    .into_owned(),
                "--package-search-path".to_string(),
                package_collection.to_string_lossy().into_owned(),
                "--output".to_string(),
                output.to_string_lossy().into_owned(),
            ];
            if let Some(style) = style {
                args.push("--style".to_string());
                args.push(source_root.join(style).to_string_lossy().into_owned());
            }

            let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
                .args(&args)
                .output()
                .expect("run mosaic-compile");
            assert!(
                result.status.success(),
                "{component} failed on {backend}:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(
                fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0),
                "missing non-empty output for {component} on {backend}"
            );
            if component == "Notes" && backend != "react" {
                let artifact = fs::read_to_string(&output).expect("read Notes artifact");
                assert!(
                    artifact.contains("<textarea") && artifact.contains("bodyValue"),
                    "Notes did not lower its multiline Input on {backend}:\n{artifact}"
                );
            }
        }
    }

    fs::remove_dir_all(output_directory).ok();
}

#[test]
fn unknown_pascal_case_nodes_remain_explicit_errors() {
    let workspace = temporary_directory("unknown-component-reference");
    let interface = workspace.join("Unknown.mil");
    let layout = workspace.join("Unknown.mll");
    fs::write(&interface, "component Unknown { }").unwrap();
    fs::write(&layout, "layout Unknown { FutureWidget }").unwrap();

    for backend in ["html", "webcomponent", "react"] {
        let output = workspace.join(format!("Unknown-{backend}.out"));
        let result = Command::new(env!("CARGO_BIN_EXE_mosaic-compile"))
            .args([
                "--backend",
                backend,
                "--interface",
                interface.to_str().unwrap(),
                "--layout",
                layout.to_str().unwrap(),
                "--output",
                output.to_str().unwrap(),
            ])
            .output()
            .expect("run mosaic-compile");
        assert!(!result.status.success(), "{backend} accepted FutureWidget");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("FutureWidget") && stderr.contains("not yet supported"),
            "{backend} did not report the unknown node explicitly:\n{stderr}"
        );
    }

    fs::remove_dir_all(workspace).ok();
}
